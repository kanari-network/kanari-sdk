//! Changeset application logic for the blockchain state.

use super::*;
use kanari_types::coin::CoinModule;
use kanari_types::collection::CollectionModule;
use kanari_types::env::env_flag;
use kanari_types::transaction::ObjectOwnerKind;

mod apply_balances;
mod apply_clock;
mod apply_phases;
mod apply_snapshots;
mod apply_treasury;

/// Gas-debit context resolved once per changeset.
///
/// Passing this instead of three loose scalars (`native_gas_debit_total`,
/// `implicit_gas_payer`, `has_dedicated_gas_object`) removes the chance of a
/// phase forwarding a placeholder zero and silently disabling its own debit.
#[derive(Clone, Copy)]
struct GasDebitContext {
    /// Total native gas debit to collect across all gas objects.
    total: u64,
    /// Implicit gas payer address (only used when no dedicated gas object exists).
    implicit_payer: Option<AccountAddress>,
    /// Whether at least one dedicated gas object is present.
    has_dedicated_object: bool,
}

impl GasDebitContext {
    fn enabled(&self, owner: &AccountAddress) -> bool {
        self.has_dedicated_object && Some(*owner) == self.implicit_payer
    }
}

impl StateManager {
    /// Apply ChangeSet from Move VM execution
    /// This is the ONLY way to modify state - all changes must come from Move VM
    pub fn apply_changeset(&mut self, changeset: &ChangeSet) -> Result<()> {
        // Guard: a positive native supply delta (mint) must be backed by a
        // detected TreasuryCap update or an explicit mint() call. Without this,
        // a malformed changeset could inflate total_supply without an explicit
        // mint.
        if changeset.is_unbacked_supply_inflation()? {
            let delta = changeset.native_supply_delta()?;
            anyhow::bail!(
                "Native supply inflation rejected: positive supply delta {} without treasury update or mint",
                delta
            );
        }
        // Guard against oversized changesets that could OOM the validation clone.
        const MAX_CHANGESET_OBJECTS: usize = 100_000;
        if changeset.created_objects.len() + changeset.deleted_objects.len() > MAX_CHANGESET_OBJECTS
        {
            anyhow::bail!(
                "changeset too large: {} objects (limit {})",
                changeset.created_objects.len() + changeset.deleted_objects.len(),
                MAX_CHANGESET_OBJECTS
            );
        }
        // Guard overlay size to prevent clone-based DoS on large state.
        const MAX_OVERLAY_ENTRIES: usize = 1_000_000;
        if self.overlay.len() > MAX_OVERLAY_ENTRIES {
            anyhow::bail!(
                "state overlay too large: {} entries (limit {})",
                self.overlay.len(),
                MAX_OVERLAY_ENTRIES
            );
        }
        if changeset.treasuries.is_empty()
            && self.global_token_supplies.get(GAS_COIN).copied().unwrap_or(0) > self.total_supply
        {
            self.validate_supply_invariants()?;
        }
        let mut changeset = changeset.clone();
        self.apply_changeset_with_options(&mut changeset, true)
    }

    /// Apply a changeset and retain runtime-generated audit events in the
    /// caller's changeset so transaction effects and block indexing can see
    /// them.
    pub fn apply_changeset_with_audit(&mut self, changeset: &mut ChangeSet) -> Result<()> {
        self.apply_changeset_with_options(changeset, true)
    }

    /// Apply a ChangeSet without validating supply invariants.
    pub fn apply_changeset_without_supply_validation(
        &mut self,
        changeset: &ChangeSet,
    ) -> Result<()> {
        let mut changeset = changeset.clone();
        self.apply_changeset_with_options(&mut changeset, false)
    }

    /// Entry point for changeset application. When `validate_supply` is true,
    /// validates on a cloned snapshot so rejected transactions cannot poison
    /// live state; otherwise delegates directly to [`apply_changeset_core`].
    fn apply_changeset_with_options(
        &mut self,
        changeset: &mut ChangeSet,
        validate_supply: bool,
    ) -> Result<()> {
        let profile_apply = env_flag("KANARI_STATE_APPLY_PROFILE");
        let apply_started_at = std::time::Instant::now();
        if validate_supply {
            // Validate on a cloned snapshot so rejected transactions cannot poison live state.
            let mut candidate = self.clone();
            let mut candidate_changeset = changeset.clone();
            candidate.apply_changeset_with_options(&mut candidate_changeset, false)?;
            candidate.repair_legacy_native_wallet_overcount()?;
            if let Err(error) = candidate.validate_cached_supply_invariants() {
                Self::report_supply_invariant_violation("after apply_changeset", &error)?;
                return Err(error);
            }
            *self = candidate;
            *changeset = candidate_changeset;
            return Ok(());
        }

        self.apply_changeset_core(changeset, profile_apply, apply_started_at)
    }

    /// Core changeset application (no validation clone). Orchestrates phases:
    /// supply delta → gas debit → snapshot → owner deltas → created objects →
    /// gas debits → indexes → reconciliation → move writes.
    fn apply_changeset_core(
        &mut self,
        changeset: &mut ChangeSet,
        profile_apply: bool,
        apply_started_at: std::time::Instant,
    ) -> Result<()> {
        let snapshots_at = std::time::Instant::now();

        // --- Phase 1: supply delta / total supply ---
        let next_total_supply = self.compute_supply_delta(changeset)?;

        // --- Phase 2: gas debit total ---
        let native_gas_debit_total =
            changeset.native_gas_credits.values().try_fold(0u64, |total, credit| {
                total.checked_add(*credit).require("Native gas debit total overflow")
            })?;

        // --- Phase 3: gas object setup ---
        let gas_object_ids = changeset
            .gas_object_refs
            .iter()
            .map(|gas_ref| Self::canonical_owned_object_id(&gas_ref.object_id))
            .collect::<BTreeSet<_>>();
        let created_object_owners_by_id = changeset
            .created_objects
            .iter()
            .map(|(object_id, created)| (Self::canonical_owned_object_id(object_id), created.owner))
            .collect::<BTreeMap<_, _>>();
        let implicit_gas_payer = changeset
            .gas_payment
            .as_ref()
            .and_then(|payment| AccountAddress::from_hex_literal(&payment.owner).ok());
        let has_dedicated_gas_object =
            native_gas_debit_total > 0 && !changeset.gas_object_refs.is_empty();
        let gas_debit_ctx = GasDebitContext {
            total: native_gas_debit_total,
            implicit_payer: implicit_gas_payer,
            has_dedicated_object: has_dedicated_gas_object,
        };

        // --- Phase 4: snapshot native balances + validate owner debits ---
        let native_balances_before = self.snapshot_native_balances(changeset)?;
        self.validate_owner_debits(changeset, &native_balances_before)?;

        // --- Phase 5: initialize mutable apply state ---
        let mut supplies_dirty = false;
        let mut owner_index_additions = Vec::with_capacity(changeset.owner_deltas.len());
        let reconcile_object_locked = self.needs_object_locked_reconciliation(changeset)?;
        let (issued_before, visible_before) = if reconcile_object_locked {
            self.capture_supply_snapshots(self.supply_tracking_token_types(changeset))?
        } else {
            (BTreeMap::new(), BTreeMap::new())
        };
        let mut owners_to_recompute: BTreeSet<AccountAddress> = BTreeSet::new();
        let mut native_object_changed_owners: BTreeSet<AccountAddress> = BTreeSet::new();
        let mut non_native_object_changed_owners: BTreeSet<AccountAddress> = BTreeSet::new();
        let mut native_object_owner_changed_owners: BTreeSet<AccountAddress> = BTreeSet::new();
        let mut native_object_created_owners: BTreeSet<AccountAddress> = BTreeSet::new();
        let mut native_object_gas_adjusted: BTreeMap<AccountAddress, u64> = BTreeMap::new();
        let mut native_object_before_from_changes: BTreeMap<AccountAddress, u64> = BTreeMap::new();
        let mut native_object_after_from_changes: BTreeMap<AccountAddress, u64> = BTreeMap::new();
        let mut native_balance_after_owner_deltas: BTreeMap<AccountAddress, u64> = BTreeMap::new();
        let mut object_owner_index_additions = Vec::with_capacity(
            changeset.created_objects.len().saturating_add(changeset.gas_object_refs.len()),
        );
        let mut object_index_additions = Vec::with_capacity(
            changeset.created_objects.len().saturating_add(changeset.gas_object_refs.len()),
        );
        let mut owned_object_index_additions: BTreeMap<AccountAddress, Vec<String>> =
            BTreeMap::new();

        // --- Phase 6: owner deltas ---
        self.apply_owner_deltas(
            changeset,
            &native_balances_before,
            &mut owner_index_additions,
            &mut supplies_dirty,
            &mut native_balance_after_owner_deltas,
        )?;
        let owner_deltas_at = std::time::Instant::now();

        // --- Phase 7: update total supply ---
        if let Some(next_total_supply) = next_total_supply {
            self.save_native_total_supply(next_total_supply)?;
        }

        // --- Phase 8: treasury/NFT + deleted objects ---
        self.apply_treasury_nft_balances(changeset, &mut owners_to_recompute)?;
        self.apply_deleted_objects(
            changeset,
            &mut native_object_before_from_changes,
            &mut native_object_after_from_changes,
            &mut native_object_owner_changed_owners,
            &mut owners_to_recompute,
            &mut native_object_changed_owners,
            &mut non_native_object_changed_owners,
        )?;
        self.index_collections(changeset)?;

        // --- Phase 9: created objects ---
        self.apply_created_objects(
            changeset,
            &gas_object_ids,
            &mut native_object_before_from_changes,
            &mut native_object_after_from_changes,
            &mut native_object_owner_changed_owners,
            &mut owners_to_recompute,
            &mut native_object_changed_owners,
            &mut non_native_object_changed_owners,
            &mut native_object_created_owners,
            &mut native_object_gas_adjusted,
            &mut supplies_dirty,
            gas_debit_ctx,
            &mut object_index_additions,
            &mut object_owner_index_additions,
            &mut owned_object_index_additions,
        )?;
        let created_objects_at = std::time::Instant::now();

        // --- Phase 10: gas object debits ---
        self.apply_gas_object_debits(
            changeset,
            &gas_object_ids,
            &created_object_owners_by_id,
            gas_debit_ctx,
            &mut native_object_gas_adjusted,
            &mut native_object_after_from_changes,
            &mut object_index_additions,
            &mut object_owner_index_additions,
            &mut native_object_changed_owners,
            &mut non_native_object_changed_owners,
            &mut owners_to_recompute,
        )?;
        let gas_objects_at = std::time::Instant::now();

        // --- Phase 11: index updates ---
        for (owner, object_ids) in owned_object_index_additions {
            self.add_many_owned_object_index(owner, object_ids)?;
        }
        self.add_many_to_index_list(b"object_index", object_index_additions)?;
        self.add_many_to_index_list(OWNER_INDEX_KEY, object_owner_index_additions)?;
        let indexes_at = std::time::Instant::now();

        // --- Phase 12: reconcile owner balances ---
        self.reconcile_owner_balances(
            changeset,
            &native_balances_before,
            &native_object_before_from_changes,
            &native_object_after_from_changes,
            &native_balance_after_owner_deltas,
            &owners_to_recompute,
            &mut supplies_dirty,
            &native_object_changed_owners,
            &non_native_object_changed_owners,
            &native_object_owner_changed_owners,
            &native_object_created_owners,
        )?;
        let recompute_at = std::time::Instant::now();

        // --- Phase 13: object locked reconciliation ---
        if reconcile_object_locked {
            self.reconcile_object_locked_coin_records(changeset, &issued_before, &visible_before)?;
        }

        // --- Phase 14: save supplies ---
        if supplies_dirty {
            let supplies_clone = self.global_token_supplies.clone();
            self.save_internal(b"global_token_supplies", &supplies_clone)?;
        }

        // --- Phase 15: optional post-apply supply audit (diagnostic) ---
        // Checkpoint finalize already performs strict supply validation; keep
        // this full scan opt-in to avoid hot-path overhead.
        let post_apply_audit = env_flag("KANARI_POST_APPLY_SUPPLY_AUDIT");
        if post_apply_audit && let Err(error) = self.validate_no_lost_tokens() {
            log::warn!("Post-apply supply audit: {}", error);
        }

        // --- Phase 16: move writes, dynamic fields, access versions ---
        self.apply_move_writes(changeset)?;
        let move_writes_at = std::time::Instant::now();

        self.apply_dynamic_fields(changeset)?;
        let dynamic_fields_at = std::time::Instant::now();

        self.advance_access_versions(&changeset.deterministic_access_set())?;
        let access_versions_at = std::time::Instant::now();

        // --- Phase 17: profiling ---
        if profile_apply {
            eprintln!(
                "state apply profile: owners={} created={} gas_refs={} move_writes={} snapshots={:.6}s owner_deltas={:.6}s created_objects={:.6}s gas_objects={:.6}s indexes={:.6}s recompute={:.6}s move_writes={:.6}s dynamic={:.6}s access_versions={:.6}s total={:.6}s",
                changeset.owner_deltas.len(),
                changeset.created_objects.len(),
                changeset.gas_object_refs.len(),
                changeset.move_writes.len(),
                snapshots_at.duration_since(apply_started_at).as_secs_f64(),
                owner_deltas_at.duration_since(snapshots_at).as_secs_f64(),
                created_objects_at.duration_since(owner_deltas_at).as_secs_f64(),
                gas_objects_at.duration_since(created_objects_at).as_secs_f64(),
                indexes_at.duration_since(gas_objects_at).as_secs_f64(),
                recompute_at.duration_since(indexes_at).as_secs_f64(),
                move_writes_at.duration_since(recompute_at).as_secs_f64(),
                dynamic_fields_at.duration_since(move_writes_at).as_secs_f64(),
                access_versions_at.duration_since(move_writes_at).as_secs_f64(),
                access_versions_at.duration_since(apply_started_at).as_secs_f64(),
            );
        }

        Ok(())
    }

    /// Compute the post-apply native total supply from the changeset's supply delta.
    /// Returns `None` when there is no mint/burn (delta == 0).
    fn compute_supply_delta(&self, changeset: &ChangeSet) -> Result<Option<u64>> {
        let supply_delta = changeset.native_supply_delta()?;
        if supply_delta > 0 {
            let mint_amount = u64::try_from(supply_delta)
                .require("Native supply delta overflowed u64 total supply")?;
            Ok(Some(
                self.total_supply
                    .checked_add(mint_amount)
                    .require("Native total supply overflow")?,
            ))
        } else if supply_delta < 0 {
            let burn_amount = u64::try_from(supply_delta.unsigned_abs())
                .require("Native supply delta overflowed u64 total supply")?;
            ensure!(
                self.total_supply >= burn_amount,
                "Native total supply underflow: tried to burn {} from {}",
                burn_amount,
                self.total_supply
            );
            Ok(Some(
                self.total_supply
                    .checked_sub(burn_amount)
                    .context("Native total supply underflow after burn precheck")?,
            ))
        } else {
            Ok(None)
        }
    }

    /// Native debit for an owner from owner_deltas (negative delta only).
    fn owner_native_debit(&self, changeset: &ChangeSet, owner: &AccountAddress) -> Result<u64> {
        changeset
            .owner_deltas
            .get(owner)
            .map(|change| change.balance_delta)
            .filter(|delta| *delta < 0)
            .map(|delta| {
                u64::try_from(delta.unsigned_abs())
                    .require("Native debit overflowed u64 gas object adjustment")
            })
            .transpose()
            .map(|debit| debit.unwrap_or(0))
    }

    /// Native gas debit for an owner. Only non-zero when the owner is the
    /// explicit gas object payer or the implicit gas payer with a dedicated
    /// gas object present.
    fn owner_native_gas_debit(
        &self,
        changeset: &ChangeSet,
        owner: &AccountAddress,
        explicit_gas_object: bool,
        ctx: GasDebitContext,
    ) -> Result<u64> {
        let enabled = explicit_gas_object || ctx.enabled(owner);
        Ok(if enabled { self.owner_native_debit(changeset, owner)?.min(ctx.total) } else { 0 })
    }

    /// Apply owner balance deltas (credits and debits) from the changeset.
    /// Updates owner records, owner index, and tracks post-delta balances.
    fn apply_owner_deltas(
        &mut self,
        changeset: &ChangeSet,
        native_balances_before: &BTreeMap<AccountAddress, (u64, u64)>,
        owner_index_additions: &mut Vec<String>,
        supplies_dirty: &mut bool,
        native_balance_after_owner_deltas: &mut BTreeMap<AccountAddress, u64>,
    ) -> Result<()> {
        for (address, change) in &changeset.owner_deltas {
            let mut owner_state = self.load_owner_state_or_default(*address)?;
            let old_balances = owner_state.token_balances.clone();
            let native_token = GAS_COIN.to_string();

            if change.balance_delta > 0 {
                let amount = u64::try_from(change.balance_delta)
                    .require("Native credit overflowed u64 owner balance")?;
                let next = owner_state
                    .native_balance()
                    .checked_add(amount)
                    .require("Native owner balance overflow")?;
                owner_state.set_token_balance_value(&native_token, next);
            } else if change.balance_delta < 0 {
                let debit = u64::try_from(change.balance_delta.unsigned_abs())
                    .require("Native debit overflowed u64 owner balance")?;
                let current = owner_state.native_balance();
                let effective_balance = if current >= debit {
                    current
                } else {
                    let (_, object_balance) =
                        native_balances_before.get(address).copied().unwrap_or((current, 0));
                    current.max(object_balance)
                };
                let next = effective_balance.checked_sub(debit).with_context(|| {
                    format!(
                        "Insufficient native balance for {} while applying owner delta: need {}, have {}",
                        address.to_hex_literal(),
                        debit,
                        effective_balance
                    )
                })?;
                owner_state.set_token_balance_value(&native_token, next);
            }
            native_balance_after_owner_deltas
                .insert(owner_state.address, owner_state.native_balance());
            for module_name in &change.modules_added {
                owner_state.add_module(module_name.clone());
            }
            if owner_state.is_empty() {
                self.overlay.insert(Self::owner_state_key(&owner_state.address), None);
                self.remove_from_index_list(
                    OWNER_INDEX_KEY,
                    &owner_state.address.to_hex_literal(),
                )?;
            } else {
                self.save_owner_record(&owner_state)?;
                owner_index_additions.push(owner_state.address.to_hex_literal());
            }
            *supplies_dirty |= self.capture_supply_changed(&owner_state, &old_balances)?;
        }
        self.add_many_to_index_list(OWNER_INDEX_KEY, std::mem::take(owner_index_additions))?;
        Ok(())
    }

    /// Apply created objects: versioning, gas adjustments, treasury/NFT
    /// persistence, index updates, and coin metadata.
    #[allow(clippy::too_many_arguments)]
    fn apply_created_objects(
        &mut self,
        changeset: &ChangeSet,
        gas_object_ids: &BTreeSet<String>,
        native_object_before_from_changes: &mut BTreeMap<AccountAddress, u64>,
        native_object_after_from_changes: &mut BTreeMap<AccountAddress, u64>,
        native_object_owner_changed_owners: &mut BTreeSet<AccountAddress>,
        owners_to_recompute: &mut BTreeSet<AccountAddress>,
        native_object_changed_owners: &mut BTreeSet<AccountAddress>,
        non_native_object_changed_owners: &mut BTreeSet<AccountAddress>,
        native_object_created_owners: &mut BTreeSet<AccountAddress>,
        native_object_gas_adjusted: &mut BTreeMap<AccountAddress, u64>,
        supplies_dirty: &mut bool,
        gas_debit_ctx: GasDebitContext,
        object_index_additions: &mut Vec<String>,
        object_owner_index_additions: &mut Vec<String>,
        owned_object_index_additions: &mut BTreeMap<AccountAddress, Vec<String>>,
    ) -> Result<()> {
        for (obj_id, created) in &changeset.created_objects {
            let mut new_obj = created.clone();
            let existing_obj = self.load_stored_object_by_any_id(obj_id)?;
            let existing_stored_id = existing_obj.as_ref().map(|(stored_id, _)| stored_id.clone());
            let existing_owner = existing_obj.as_ref().map(|(_, existing)| existing.owner);
            let canonical_obj_id = Self::canonical_owned_object_id(obj_id);
            let obj_key = object_key(&canonical_obj_id);

            if let Some((stored_id, existing)) = existing_obj {
                // Canonical object versions must be derived from the visible state snapshot,
                // not the runtime object cache, so every authority commits the same version.
                // Replaying the consensus clock prologue for the same timestamp is idempotent.
                // Some checkpoint paths can observe the same prologue more than once; allowing
                // an identical replay to bump only the version would split state roots even
                // though the canonical clock value is unchanged.
                let identical_clock_replay = self
                    .is_system_clock_object(obj_id, &existing.type_name)?
                    && existing.type_name == new_obj.type_
                    && existing.data == new_obj.data
                    && existing.owner == new_obj.owner
                    && existing.owner_kind == new_obj.owner_kind;
                new_obj.version =
                    if identical_clock_replay { existing.version } else { existing.version + 1 };
                if crate::common::ids::object_id_for_address(&new_obj.owner) == *obj_id {
                    new_obj.owner = existing.owner;
                }
                let existing_balance =
                    Self::balance_token_amount(&existing.type_name, &existing.data);
                let existing_native_coin =
                    existing_balance.as_ref().is_some_and(|(token_type, _)| {
                        CoinModule::normalize_token_type(token_type) == GAS_COIN
                    });
                if let Some((token_type, amount)) = existing_balance
                    && CoinModule::normalize_token_type(&token_type) == GAS_COIN
                {
                    let owner_balance =
                        native_object_before_from_changes.entry(existing.owner).or_insert(0);
                    *owner_balance = owner_balance
                        .checked_add(amount)
                        .require("Native object balance snapshot overflow")?;
                }
                let is_designated_gas_object = gas_object_ids.contains(&canonical_obj_id);
                // Only explicitly declared gas objects are adjusted here. There is
                // intentionally no implicit fallback to some coin written by Move.
                if existing_native_coin && is_designated_gas_object {
                    let sender_native_debit = self.owner_native_gas_debit(
                        changeset,
                        &existing.owner,
                        is_designated_gas_object,
                        gas_debit_ctx,
                    )?;
                    let already_adjusted =
                        native_object_gas_adjusted.get(&existing.owner).copied().unwrap_or(0);
                    let remaining_debit = sender_native_debit.saturating_sub(already_adjusted);
                    if remaining_debit > 0 {
                        let new_struct_tag = StructTag::from_str(&new_obj.type_)
                            .context("Invalid transferred native coin object type")?;
                        let object_amount =
                            extract_balance_from_object_bytes(&new_obj.data, &new_struct_tag)
                                .context("Invalid transferred native coin object data")?;
                        ensure!(
                            object_amount >= remaining_debit,
                            "Insufficient native coin object balance for gas: balance={}, debit={}",
                            object_amount,
                            remaining_debit
                        );
                        let next_amount = object_amount - remaining_debit;
                        ensure!(
                            Self::write_balance_to_object_bytes(
                                &mut new_obj.data,
                                &new_struct_tag,
                                next_amount,
                            ),
                            "Failed to write adjusted native coin object balance"
                        );
                        native_object_gas_adjusted.insert(existing.owner, sender_native_debit);
                    }
                }

                if existing.owner != new_obj.owner
                    && matches!(existing.owner_kind, ObjectOwnerKind::AddressOwner(_))
                {
                    if existing_native_coin {
                        native_object_owner_changed_owners.insert(existing.owner);
                        native_object_owner_changed_owners.insert(new_obj.owner);
                    }
                    self.remove_owned_object_index_variants(
                        existing.owner,
                        obj_id,
                        Some(&stored_id),
                    )?;
                }
                if stored_id != canonical_obj_id {
                    self.overlay.insert(object_key(&stored_id), None);
                }
                Self::record_object_balance_owner_if_needed(
                    owners_to_recompute,
                    native_object_changed_owners,
                    non_native_object_changed_owners,
                    existing.owner,
                    &existing.type_name,
                    &existing.data,
                );
            } else {
                // For new objects, use version from ChangeSet or default to 1
                if new_obj.version == 0 {
                    new_obj.version = 1;
                }
            }
            Self::record_object_balance_owner_if_needed(
                owners_to_recompute,
                native_object_changed_owners,
                non_native_object_changed_owners,
                new_obj.owner,
                &new_obj.type_,
                &new_obj.data,
            );
            if let Some((token_type, amount)) =
                Self::balance_token_amount(&new_obj.type_, &new_obj.data)
                && CoinModule::normalize_token_type(&token_type) == GAS_COIN
            {
                if existing_stored_id.is_none() {
                    native_object_created_owners.insert(new_obj.owner);
                }
                let owner_balance =
                    native_object_after_from_changes.entry(new_obj.owner).or_insert(0);
                *owner_balance = owner_balance
                    .checked_add(amount)
                    .require("Native object balance snapshot overflow")?;
            }
            if let Some((token_type, total_supply)) =
                Self::treasury_cap_token_supply(&new_obj.type_, &new_obj.data)
            {
                *supplies_dirty |=
                    self.persist_treasury_cap_state(&token_type, new_obj.owner, total_supply)?;
            }

            let stored_obj = StoredObject {
                id: canonical_obj_id.clone(),
                owner: new_obj.owner,
                owner_kind: new_obj.owner_kind.clone(),
                type_name: new_obj.type_.clone(),
                data: new_obj.data.clone(),
                version: new_obj.version,
            };
            self.save_internal(&obj_key, &stored_obj)?;
            object_index_additions.push(canonical_obj_id.clone());

            if matches!(new_obj.owner_kind, ObjectOwnerKind::AddressOwner(_)) {
                object_owner_index_additions.push(new_obj.owner.to_hex_literal());
                if existing_stored_id.is_none() {
                    owned_object_index_additions
                        .entry(new_obj.owner)
                        .or_default()
                        .push(canonical_obj_id.clone());
                } else if existing_stored_id.as_deref() != Some(canonical_obj_id.as_str())
                    || existing_owner.is_some_and(|owner| owner != new_obj.owner)
                {
                    self.refresh_owned_object_index(
                        new_obj.owner,
                        obj_id,
                        existing_stored_id.as_deref(),
                    )?;
                }
            }

            if CoinModule::is_coin_metadata_type(&new_obj.type_)
                && let Some(start) = new_obj.type_.find('<')
                && let Some(end) = new_obj.type_.rfind('>')
            {
                let token_type = &new_obj.type_[start + 1..end];
                self.persist_coin_metadata(token_type, &new_obj.data)?;
            }
        }
        Ok(())
    }

    /// Apply gas object debits: debit the explicitly declared gas object(s)
    /// rather than charging whichever coin happened to be mutated first.
    #[allow(clippy::too_many_arguments)]
    fn apply_gas_object_debits(
        &mut self,
        changeset: &ChangeSet,
        _gas_object_ids: &BTreeSet<String>,
        created_object_owners_by_id: &BTreeMap<String, AccountAddress>,
        gas_debit_ctx: GasDebitContext,
        native_object_gas_adjusted: &mut BTreeMap<AccountAddress, u64>,
        native_object_after_from_changes: &mut BTreeMap<AccountAddress, u64>,
        object_index_additions: &mut Vec<String>,
        object_owner_index_additions: &mut Vec<String>,
        native_object_changed_owners: &mut BTreeSet<AccountAddress>,
        non_native_object_changed_owners: &mut BTreeSet<AccountAddress>,
        owners_to_recompute: &mut BTreeSet<AccountAddress>,
    ) -> Result<()> {
        let gas_coin_normalized = CoinModule::normalize_token_type(changeset.gas_coin_type());
        let is_native_gas = gas_coin_normalized == GAS_COIN;

        for gas_ref in &changeset.gas_object_refs {
            let canonical_gas_id = Self::canonical_owned_object_id(&gas_ref.object_id);
            if let Some(owner) = created_object_owners_by_id.get(&canonical_gas_id) {
                let sender_debit = if is_native_gas {
                    self.owner_native_gas_debit(changeset, owner, true, gas_debit_ctx)?
                } else {
                    changeset.token_gas_credits.get(&gas_coin_normalized).copied().unwrap_or(0)
                };
                let already_adjusted = native_object_gas_adjusted.get(owner).copied().unwrap_or(0);
                if sender_debit > 0 && already_adjusted >= sender_debit {
                    continue;
                }
            }
            let Some((stored_id, existing)) =
                self.load_stored_object_by_any_id(&gas_ref.object_id)?
            else {
                continue;
            };
            let sender_debit = if is_native_gas {
                self.owner_native_gas_debit(changeset, &existing.owner, true, gas_debit_ctx)?
            } else {
                changeset.token_gas_credits.get(&gas_coin_normalized).copied().unwrap_or(0)
            };
            let already_adjusted =
                native_object_gas_adjusted.get(&existing.owner).copied().unwrap_or(0);
            let remaining_debit = sender_debit.saturating_sub(already_adjusted);
            if remaining_debit == 0 {
                continue;
            }
            let Some((token_type, object_amount)) =
                Self::balance_token_amount(&existing.type_name, &existing.data)
            else {
                continue;
            };
            if CoinModule::normalize_token_type(&token_type) != gas_coin_normalized {
                continue;
            }
            ensure!(
                object_amount >= remaining_debit,
                "Insufficient gas coin balance for {}: balance={}, debit={}",
                gas_coin_normalized,
                object_amount,
                remaining_debit
            );
            let struct_tag =
                StructTag::from_str(&existing.type_name).context("Invalid gas coin object type")?;
            let mut next_data = existing.data.clone();
            ensure!(
                Self::write_balance_to_object_bytes(
                    &mut next_data,
                    &struct_tag,
                    object_amount - remaining_debit,
                ),
                "Failed to write adjusted gas coin balance"
            );
            let next_object = StoredObject {
                id: canonical_gas_id.clone(),
                owner: existing.owner,
                owner_kind: existing.owner_kind.clone(),
                type_name: existing.type_name.clone(),
                data: next_data,
                version: existing.version.saturating_add(1),
            };
            self.save_internal(&object_key(&canonical_gas_id), &next_object)?;
            if stored_id != canonical_gas_id {
                self.overlay.insert(object_key(&stored_id), None);
            }
            object_index_additions.push(canonical_gas_id.clone());
            if matches!(next_object.owner_kind, ObjectOwnerKind::AddressOwner(_)) {
                object_owner_index_additions.push(next_object.owner.to_hex_literal());
                self.refresh_owned_object_index(
                    next_object.owner,
                    &gas_ref.object_id,
                    Some(&stored_id),
                )?;
            }
            Self::record_object_balance_owner_if_needed(
                owners_to_recompute,
                native_object_changed_owners,
                non_native_object_changed_owners,
                next_object.owner,
                &next_object.type_name,
                &next_object.data,
            );
            if let Some((token_type, amount)) =
                Self::balance_token_amount(&next_object.type_name, &next_object.data)
                && CoinModule::normalize_token_type(&token_type) == GAS_COIN
            {
                let owner_balance =
                    native_object_after_from_changes.entry(next_object.owner).or_insert(0);
                *owner_balance = owner_balance
                    .checked_add(amount)
                    .require("Native object balance snapshot overflow")?;
            }
            native_object_gas_adjusted.insert(existing.owner, sender_debit);
        }
        Ok(())
    }

    /// Reconcile per-owner native balances after object mutations, repairing
    /// overcounts when object-backed balances diverge from the ledger.
    #[allow(clippy::too_many_arguments)]
    fn reconcile_owner_balances(
        &mut self,
        changeset: &ChangeSet,
        native_balances_before: &BTreeMap<AccountAddress, (u64, u64)>,
        native_object_before_from_changes: &BTreeMap<AccountAddress, u64>,
        native_object_after_from_changes: &BTreeMap<AccountAddress, u64>,
        native_balance_after_owner_deltas: &BTreeMap<AccountAddress, u64>,
        owners_to_recompute: &BTreeSet<AccountAddress>,
        supplies_dirty: &mut bool,
        native_object_changed_owners: &BTreeSet<AccountAddress>,
        non_native_object_changed_owners: &BTreeSet<AccountAddress>,
        native_object_owner_changed_owners: &BTreeSet<AccountAddress>,
        native_object_created_owners: &BTreeSet<AccountAddress>,
    ) -> Result<()> {
        for owner in owners_to_recompute {
            let native_delta = changeset
                .owner_deltas
                .get(owner)
                .map(|change| change.balance_delta)
                .unwrap_or(0i128);
            let native_object_changed = native_object_changed_owners.contains(owner);
            let native_gas_credit = changeset.native_gas_credits.get(owner).copied().unwrap_or(0);
            let (native_ledger_before, native_object_before) =
                native_balances_before.get(owner).copied().unwrap_or((0, 0));
            let native_object_before = if native_object_before > 0 {
                native_object_before
            } else {
                native_object_before_from_changes.get(owner).copied().unwrap_or(0)
            };
            if (native_delta <= 0 || native_gas_credit == 0)
                && native_object_changed
                && !non_native_object_changed_owners.contains(owner)
                && !native_object_owner_changed_owners.contains(owner)
                && !native_object_created_owners.contains(owner)
                && let Some(native_object_after) = native_object_after_from_changes.get(owner)
                && let Some(current_native_balance) =
                    native_balance_after_owner_deltas.get(owner).copied()
                && current_native_balance > 0
            {
                let prior_non_object_balance =
                    native_ledger_before.saturating_sub(native_object_before);
                let object_backed_with_ledger = native_object_after
                    .checked_add(prior_non_object_balance)
                    .and_then(|amount| amount.checked_add(native_gas_credit))
                    .require("Native owner balance overflow during object reconciliation")?;
                let expected_native_balance = if native_delta < 0 {
                    object_backed_with_ledger.min(current_native_balance)
                } else {
                    object_backed_with_ledger
                };
                if expected_native_balance == current_native_balance {
                    continue;
                }
            }
            let changed = self.recompute_token_balances_for_owner(
                *owner,
                native_delta,
                native_object_changed,
                native_gas_credit,
                native_ledger_before,
                native_object_before,
            )?;
            if changed {
                *supplies_dirty = true;
            }
        }
        Ok(())
    }

    pub fn try_apply_owned_native_burn_batch_without_supply_validation(
        &mut self,
        changesets: &[ChangeSet],
    ) -> Result<bool> {
        if changesets.is_empty() {
            return Ok(true);
        }

        struct OwnedBurnUpdate {
            owner_deltas: Vec<(AccountAddress, i128, bool)>,
            object_key: Vec<u8>,
            object_bytes: Vec<u8>,
        }

        // Per-changeset validation and object rewriting are independent. Prepare
        // them in parallel, then merge in original order so conflict/fallback
        // decisions and state writes stay deterministic.
        use rayon::prelude::*;
        let prepared = changesets
            .par_iter()
            .map(|changeset| -> Result<Option<OwnedBurnUpdate>> {
                if !changeset.success
                    || !changeset.events.is_empty()
                    || !changeset.treasuries.is_empty()
                    || !changeset.nft_caps.is_empty()
                    || !changeset.token_balance_sets.is_empty()
                    || !changeset.shared_inputs.is_empty()
                    || !changeset.immutable_inputs.is_empty()
                    || changeset.gas_payment.is_none()
                    || changeset.gas_object_refs.len() != 1
                    || changeset.created_objects.len() != 1
                    || !changeset.deleted_objects.is_empty()
                    || !changeset.added_dynamic_fields.is_empty()
                    || !changeset.removed_dynamic_fields.is_empty()
                    || !changeset.move_writes.is_empty()
                    || !changeset.resolver_reads.is_empty()
                {
                    return Ok(None);
                }

                let gas_ref = &changeset.gas_object_refs[0];
                let (object_id, created) = &changeset.created_objects[0];
                let canonical_object_id = Self::canonical_owned_object_id(object_id);
                if Self::canonical_owned_object_id(&gas_ref.object_id) != canonical_object_id {
                    return Ok(None);
                }
                let (token_type, object_balance) =
                    Self::balance_token_amount(&created.type_, &created.data)
                        .context("Native owned batch coin object has invalid balance bytes")?;
                if CoinModule::normalize_token_type(&token_type) != GAS_COIN {
                    return Ok(None);
                }

                let Some(payment) = &changeset.gas_payment else {
                    return Ok(None);
                };
                let payment_owner = payment
                    .owner_account_strict()
                    .context("Native owned batch gas payment owner is invalid")?;
                if payment_owner != created.owner {
                    return Ok(None);
                }

                let sender_delta = changeset
                    .owner_deltas
                    .get(&created.owner)
                    .map(|delta| delta.balance_delta)
                    .unwrap_or(0);
                if sender_delta >= 0 {
                    return Ok(None);
                }
                let sender_debit = u64::try_from(sender_delta.unsigned_abs())
                    .context("Native owned batch sender debit overflowed u64")?;
                let gas_credit_total =
                    changeset.native_gas_credits.values().try_fold(0u64, |acc, value| {
                        acc.checked_add(*value).context("Native gas credit overflow in owned batch")
                    })?;
                if gas_credit_total > sender_debit || object_balance < sender_debit {
                    return Ok(None);
                }

                let mut next_data = created.data.clone();
                let struct_tag = StructTag::from_str(&created.type_)
                    .context("Native owned batch coin object type is invalid")?;
                if !Self::write_balance_to_object_bytes(
                    &mut next_data,
                    &struct_tag,
                    object_balance - sender_debit,
                ) {
                    return Ok(None);
                }

                let owner_deltas = changeset
                    .owner_deltas
                    .iter()
                    .map(|(owner, delta)| {
                        (*owner, delta.balance_delta, !delta.modules_added.is_empty())
                    })
                    .collect();
                let object_key = crate::common::keys::object_key(&canonical_object_id);
                let object = StoredObject {
                    id: canonical_object_id,
                    owner: created.owner,
                    owner_kind: created.owner_kind.clone(),
                    type_name: created.type_.clone(),
                    data: next_data,
                    version: created.version.saturating_add(1),
                };
                let object_bytes = bcs::to_bytes(&object)
                    .context("Native owned batch object serialization failed")?;
                Ok(Some(OwnedBurnUpdate { owner_deltas, object_key, object_bytes }))
            })
            .collect::<Vec<_>>();

        let mut owner_deltas: BTreeMap<AccountAddress, i128> = BTreeMap::new();
        let mut object_writes = Vec::with_capacity(prepared.len());
        let mut written_keys = Vec::with_capacity(prepared.len());
        let mut supply_delta = 0i128;
        for prepared_update in prepared {
            let Some(update) = prepared_update? else {
                return Ok(false);
            };
            for (owner, delta, has_modules_added) in update.owner_deltas {
                if has_modules_added {
                    return Ok(false);
                }
                let entry = owner_deltas.entry(owner).or_insert(0);
                *entry =
                    entry.checked_add(delta).context("Native owned batch owner delta overflow")?;
                supply_delta = supply_delta
                    .checked_add(delta)
                    .context("Native owned batch supply delta overflow")?;
            }
            object_writes.push((update.object_key.clone(), update.object_bytes));
            written_keys.push(update.object_key);
        }

        if supply_delta > 0 {
            return Ok(false);
        }
        if supply_delta < 0 {
            let burn_amount = u64::try_from(supply_delta.unsigned_abs())
                .context("Native owned batch burn overflowed u64")?;
            ensure!(
                self.total_supply >= burn_amount,
                "Native total supply underflow: tried to burn {} from {}",
                burn_amount,
                self.total_supply
            );
            let next_total_supply = self
                .total_supply
                .checked_sub(burn_amount)
                .context("Native total supply underflow after burn precheck")?;
            self.save_native_total_supply(next_total_supply)?;
            let current_visible = self.global_token_supplies.get(GAS_COIN).copied().unwrap_or(0);
            ensure!(
                current_visible >= burn_amount,
                "Native visible supply underflow in owned batch: visible={}, burn={}",
                current_visible,
                burn_amount
            );
            let next_visible = current_visible
                .checked_sub(burn_amount)
                .context("Native visible supply underflow after burn precheck")?;
            if next_visible == 0 {
                self.global_token_supplies.remove(GAS_COIN);
            } else {
                self.global_token_supplies.insert(GAS_COIN.to_string(), next_visible);
            }
            let supplies_clone = self.global_token_supplies.clone();
            self.save_internal(b"global_token_supplies", &supplies_clone)?;
        }

        let mut owner_index_additions = Vec::with_capacity(owner_deltas.len());
        for (owner, delta) in owner_deltas {
            let mut owner_state = self.load_owner_state_or_default(owner)?;
            let current = owner_state.native_balance();
            let next = if delta >= 0 {
                current
                    .checked_add(u64::try_from(delta).context("Native credit overflowed u64")?)
                    .context("Native owner balance overflow")?
            } else {
                let debit =
                    u64::try_from(delta.unsigned_abs()).context("Native debit overflowed u64")?;
                ensure!(
                    current >= debit,
                    "Insufficient native balance for {}: need {}, have {}",
                    owner.to_hex_literal(),
                    debit,
                    current
                );
                current
                    .checked_sub(debit)
                    .context("Native owner balance underflow after debit precheck")?
            };
            owner_state.set_token_balance_value(GAS_COIN, next);
            if owner_state.is_empty() {
                self.overlay.insert(Self::owner_state_key(&owner), None);
                self.remove_from_index_list(OWNER_INDEX_KEY, &owner.to_hex_literal())?;
            } else {
                self.save_owner_record(&owner_state)?;
                owner_index_additions.push(owner.to_hex_literal());
            }
        }

        for (key, bytes) in object_writes {
            self.overlay.insert(key, Some(bytes));
        }
        self.add_many_to_index_list(OWNER_INDEX_KEY, owner_index_additions)?;
        self.advance_access_versions_for_keys(written_keys)?;
        Ok(true)
    }
}
