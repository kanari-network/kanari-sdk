// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Secure multi-signature wallet.
///
/// Flow: create (+ fund) -> propose -> approve (until threshold) -> execute.
/// Every state-changing execution re-validates its preconditions at execution
/// time (balances, owner set, threshold), so proposals that go stale between
/// proposal and execution abort instead of doing the wrong thing.
///
/// Security properties:
/// - The wallet custodies `Coin<T>` itself; transfers split from that balance.
///   Executors cannot substitute their own funds.
/// - Every proposal is bound to exactly one wallet (`wallet_id`); a proposal
///   approved for wallet A can never execute against wallet B.
/// - Owner-set / threshold mutations happen for real inside `execute`.
/// - Proposals expire (`ttl_ms`, 0 = never) and can be cancelled by the
///   proposer or garbage-collected by anyone after expiry.
module kanari_system::multisig {
    use std::string;
    use std::vector;
    use kanari_system::bcs;
    use kanari_system::borrow;
    use kanari_system::coin::{Self, Coin};
    use kanari_system::deny_list::{Self, DenyList};
    use kanari_system::event;
    use kanari_system::math;
    use kanari_system::object::{Self, UID};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};

    // --- Error Codes ---
    const E_NOT_OWNER: u64 = 1;
    const E_ALREADY_APPROVED: u64 = 2;
    const E_THRESHOLD_NOT_MET: u64 = 3;
    const E_TRANSACTION_ALREADY_EXECUTED: u64 = 4;
    const E_INVALID_THRESHOLD: u64 = 5;
    const E_EMPTY_OWNERS: u64 = 6;
    const E_OWNER_NOT_FOUND: u64 = 7;
    const E_CANNOT_REMOVE_LAST_OWNER: u64 = 8;
    const E_INVALID_TRANSACTION_TYPE: u64 = 9;
    const E_INSUFFICIENT_BALANCE: u64 = 10;
    const E_ALREADY_OWNER: u64 = 11;
    const E_PROPOSAL_EXPIRED: u64 = 12;
    const E_BINDING_MISMATCH: u64 = 13;
    const E_NOT_PROPOSER: u64 = 14;
    const E_INVALID_PAYLOAD: u64 = 15;
    const E_NOT_EXPIRED: u64 = 16;
    const E_ZERO_AMOUNT: u64 = 17;
    const E_DENIED: u64 = 18;
    const E_ZERO_ADDRESS: u64 = 19;

    // --- Transaction Types ---
    const TX_TYPE_TRANSFER: u8 = 0;
    const TX_TYPE_ADD_OWNER: u8 = 1;
    const TX_TYPE_REMOVE_OWNER: u8 = 2;
    const TX_TYPE_CHANGE_THRESHOLD: u8 = 3;

    /// `ttl_ms == 0` means "never expires".
    const NO_EXPIRY: u64 = 0;

    // --- Data Structures ---

    /// Multisig wallet. Custodies `Coin<T>` directly: the only way funds leave
    /// is an executed transfer proposal.
    struct MultisigWallet<phantom T> has key, store {
        id: UID,
        owners: vector<address>,
        threshold: u64,
        transaction_count: u64,
        funds: Coin<T>,
    }

    /// Transaction proposal. Always bound to one wallet via `wallet_id`.
    struct TransactionProposal has key, store {
        id: UID,
        wallet_id: object::ID,
        tx_type: u8,
        proposer: address,
        target_address: address,
        amount: u64,
        payload: vector<u8>,
        description: string::String,
        approvers: vector<address>,
        executed: bool,
        created_at_ms: u64,
        expires_at_ms: u64,
    }

    struct WalletCreatedEvent has copy, drop {
        wallet_id: address,
        owners: vector<address>,
        threshold: u64,
    }

    struct FundsDepositedEvent has copy, drop {
        wallet_id: address,
        depositor: address,
        amount: u64,
    }

    struct TransactionProposedEvent has copy, drop {
        wallet_id: address,
        transaction_id: address,
        tx_type: u8,
        proposer: address,
        target_address: address,
        amount: u64,
        expires_at_ms: u64,
    }

    struct TransactionApprovedEvent has copy, drop {
        wallet_id: address,
        transaction_id: address,
        approver: address,
        approval_count: u64,
        threshold: u64,
    }

    struct TransactionExecutedEvent has copy, drop {
        wallet_id: address,
        transaction_id: address,
        executor: address,
    }

    struct ProposalCancelledEvent has copy, drop {
        wallet_id: address,
        transaction_id: address,
        canceller: address,
    }

    struct OwnerChangedEvent has copy, drop {
        wallet_id: address,
        action: u8, // 0 = added, 1 = removed
        owner: address,
    }

    struct ThresholdChangedEvent has copy, drop {
        wallet_id: address,
        old_threshold: u64,
        new_threshold: u64,
    }

    // --- Wallet lifecycle ---

    /// Create a wallet funded with `initial_funds` (use `coin::zero` for empty).
    public fun create_wallet<T>(
        owners: vector<address>,
        threshold: u64,
        initial_funds: Coin<T>,
        ctx: &mut TxContext,
    ): MultisigWallet<T> {
        let owners_len = vector::length(&owners);
        assert!(owners_len > 0, E_EMPTY_OWNERS);
        assert!(threshold > 0, E_INVALID_THRESHOLD);
        assert!(threshold <= (owners_len as u64), E_INVALID_THRESHOLD);
        check_duplicate_owners(&owners);
        // Reject the zero address: it can never approve and poisons the owner set.
        let i = 0;
        while (i < owners_len) {
            assert!(*vector::borrow(&owners, i) != @0x0, E_ZERO_ADDRESS);
            i = i + 1;
        };

        let wallet = MultisigWallet<T> {
            id: object::new(ctx),
            owners,
            threshold,
            transaction_count: 0,
            funds: initial_funds,
        };
        event::emit(WalletCreatedEvent {
            wallet_id: wallet_address(&wallet),
            owners: wallet.owners,
            threshold: wallet.threshold,
        });
        wallet
    }

    /// Top up the wallet. Anyone can deposit.
    public fun deposit<T>(wallet: &mut MultisigWallet<T>, funds: Coin<T>, ctx: &TxContext) {
        let amount = coin::value(&funds);
        coin::join(&mut wallet.funds, funds);
        event::emit(FundsDepositedEvent {
            wallet_id: wallet_address(wallet),
            depositor: tx_context::sender(ctx),
            amount,
        });
    }

    /// Current custodial balance.
    public fun balance<T>(wallet: &MultisigWallet<T>): u64 {
        coin::value(&wallet.funds)
    }

    // --- Proposals ---

    /// Shared constructor: owner-only, proposer auto-approves, TTL enforced.
    fun new_proposal<T>(
        wallet: &MultisigWallet<T>,
        tx_type: u8,
        target_address: address,
        amount: u64,
        payload: vector<u8>,
        description: string::String,
        ttl_ms: u64,
        ctx: &mut TxContext,
    ): TransactionProposal {
        let sender = tx_context::sender(ctx);
        assert!(is_owner(wallet, sender), E_NOT_OWNER);

        let now_ms = tx_context::epoch_timestamp_ms(ctx);
        let expires_at_ms = if (ttl_ms == NO_EXPIRY) {
            NO_EXPIRY
        } else {
            // Saturating add: absurd TTLs clamp instead of aborting with a
            // generic arithmetic error.
            math::saturating_add_u64(now_ms, ttl_ms)
        };
        let proposal = TransactionProposal {
            id: object::new(ctx),
            wallet_id: object::uid_to_inner(&wallet.id),
            tx_type,
            proposer: sender,
            target_address,
            amount,
            payload,
            description,
            approvers: vector::singleton(sender),
            executed: false,
            created_at_ms: now_ms,
            expires_at_ms,
        };
        event::emit(TransactionProposedEvent {
            wallet_id: wallet_address(wallet),
            transaction_id: object::id_to_address(&object::uid_to_inner(&proposal.id)),
            tx_type,
            proposer: sender,
            target_address,
            amount,
            expires_at_ms,
        });
        proposal
    }

    public fun propose_transfer<T>(
        wallet: &MultisigWallet<T>,
        target_address: address,
        amount: u64,
        description: string::String,
        ttl_ms: u64,
        ctx: &mut TxContext,
    ): TransactionProposal {
        assert!(amount > 0, E_ZERO_AMOUNT);
        new_proposal(
            wallet, TX_TYPE_TRANSFER, target_address, amount,
            vector::empty<u8>(), description, ttl_ms, ctx,
        )
    }

    public fun propose_add_owner<T>(
        wallet: &MultisigWallet<T>,
        new_owner: address,
        description: string::String,
        ttl_ms: u64,
        ctx: &mut TxContext,
    ): TransactionProposal {
        assert!(new_owner != @0x0, E_ZERO_ADDRESS);
        assert!(!is_owner(wallet, new_owner), E_ALREADY_OWNER);
        new_proposal(
            wallet, TX_TYPE_ADD_OWNER, new_owner, 0,
            vector::empty<u8>(), description, ttl_ms, ctx,
        )
    }

    public fun propose_remove_owner<T>(
        wallet: &MultisigWallet<T>,
        owner_to_remove: address,
        description: string::String,
        ttl_ms: u64,
        ctx: &mut TxContext,
    ): TransactionProposal {
        assert!(vector::length(&wallet.owners) > 1, E_CANNOT_REMOVE_LAST_OWNER);
        assert!(is_owner(wallet, owner_to_remove), E_OWNER_NOT_FOUND);
        new_proposal(
            wallet, TX_TYPE_REMOVE_OWNER, owner_to_remove, 0,
            vector::empty<u8>(), description, ttl_ms, ctx,
        )
    }

    public fun propose_change_threshold<T>(
        wallet: &MultisigWallet<T>,
        new_threshold: u64,
        description: string::String,
        ttl_ms: u64,
        ctx: &mut TxContext,
    ): TransactionProposal {
        assert!(new_threshold > 0, E_INVALID_THRESHOLD);
        assert!(new_threshold <= owner_count(wallet), E_INVALID_THRESHOLD);
        new_proposal(
            wallet, TX_TYPE_CHANGE_THRESHOLD, @0x0, 0,
            bcs::to_bytes(&new_threshold), description, ttl_ms, ctx,
        )
    }

    // --- Approval / cancellation / expiry ---

    public fun approve_transaction<T>(
        wallet: &MultisigWallet<T>,
        proposal: &mut TransactionProposal,
        ctx: &mut TxContext,
    ) {
        let sender = tx_context::sender(ctx);
        assert_bound(wallet, proposal);
        assert!(is_owner(wallet, sender), E_NOT_OWNER);
        assert!(!proposal.executed, E_TRANSACTION_ALREADY_EXECUTED);
        assert!(!is_expired(proposal, ctx), E_PROPOSAL_EXPIRED);
        assert!(!has_approved(proposal, sender), E_ALREADY_APPROVED);

        vector::push_back(&mut proposal.approvers, sender);
        event::emit(TransactionApprovedEvent {
            wallet_id: wallet_address(wallet),
            transaction_id: object::id_to_address(&object::uid_to_inner(&proposal.id)),
            approver: sender,
            approval_count: (vector::length(&proposal.approvers) as u64),
            threshold: wallet.threshold,
        });
    }

    /// Cancel a live proposal. Only the proposer may cancel.
    public fun cancel_proposal<T>(
        wallet: &MultisigWallet<T>,
        proposal: TransactionProposal,
        ctx: &TxContext,
    ) {
        let sender = tx_context::sender(ctx);
        assert_bound(wallet, &proposal);
        assert!(sender == proposal.proposer, E_NOT_PROPOSER);
        assert!(!proposal.executed, E_TRANSACTION_ALREADY_EXECUTED);

        event::emit(ProposalCancelledEvent {
            wallet_id: wallet_address(wallet),
            transaction_id: object::id_to_address(&object::uid_to_inner(&proposal.id)),
            canceller: sender,
        });
        let TransactionProposal {
            id, wallet_id: _, tx_type: _, proposer: _, target_address: _,
            amount: _, payload: _, description: _, approvers: _,
            executed: _, created_at_ms: _, expires_at_ms: _,
        } = proposal;
        object::delete(id);
    }

    /// Garbage-collect an expired proposal. Anyone may call; reclaims storage.
    public fun delete_expired_proposal(proposal: TransactionProposal, ctx: &TxContext) {
        assert!(is_expired(&proposal, ctx), E_NOT_EXPIRED);
        event::emit(ProposalCancelledEvent {
            wallet_id: object::id_to_address(&proposal.wallet_id),
            transaction_id: object::id_to_address(&object::uid_to_inner(&proposal.id)),
            canceller: tx_context::sender(ctx),
        });
        let TransactionProposal {
            id, wallet_id: _, tx_type: _, proposer: _, target_address: _,
            amount: _, payload: _, description: _, approvers: _,
            executed: _, created_at_ms: _, expires_at_ms: _,
        } = proposal;
        object::delete(id);
    }

    // --- Execution ---

    /// Execute a proposal whose threshold is met. Consumes the proposal.
    /// All effects are re-validated here: balance, owner set, threshold bounds.
    public fun execute_transaction<T>(
        wallet: &mut MultisigWallet<T>,
        proposal: TransactionProposal,
        ctx: &mut TxContext,
    ) {
        let sender = tx_context::sender(ctx);
        assert_bound(wallet, &proposal);
        assert!(is_owner(wallet, sender), E_NOT_OWNER);
        assert!(!proposal.executed, E_TRANSACTION_ALREADY_EXECUTED);
        assert!(!is_expired(&proposal, ctx), E_PROPOSAL_EXPIRED);
        assert!(has_enough_approvals(wallet, &proposal), E_THRESHOLD_NOT_MET);

        let wallet_id = wallet_address(wallet);
        let proposal_id = object::id_to_address(&object::uid_to_inner(&proposal.id));

        if (proposal.tx_type == TX_TYPE_TRANSFER) {
            let amount = proposal.amount;
            assert!(amount > 0, E_ZERO_AMOUNT);
            assert!(coin::value(&wallet.funds) >= amount, E_INSUFFICIENT_BALANCE);
            let out = coin::split(&mut wallet.funds, amount, ctx);
            transfer::public_transfer(out, proposal.target_address);
        } else if (proposal.tx_type == TX_TYPE_ADD_OWNER) {
            let new_owner = proposal.target_address;
            // Re-check: another proposal may have added them first.
            assert!(!is_owner(wallet, new_owner), E_ALREADY_OWNER);
            vector::push_back(&mut wallet.owners, new_owner);
            event::emit(OwnerChangedEvent { wallet_id, action: 0, owner: new_owner });
        } else if (proposal.tx_type == TX_TYPE_REMOVE_OWNER) {
            let doomed = proposal.target_address;
            assert!(is_owner(wallet, doomed), E_OWNER_NOT_FOUND);
            assert!(vector::length(&wallet.owners) > 1, E_CANNOT_REMOVE_LAST_OWNER);
            remove_owner_addr(&mut wallet.owners, doomed);
            // Never leave the wallet in a state where the threshold is
            // unreachable; proposers must lower the threshold first.
            assert!(owner_count(wallet) >= wallet.threshold, E_INVALID_THRESHOLD);
            event::emit(OwnerChangedEvent { wallet_id, action: 1, owner: doomed });
        } else if (proposal.tx_type == TX_TYPE_CHANGE_THRESHOLD) {
            let new_threshold = decode_threshold(&proposal.payload);
            assert!(new_threshold > 0, E_INVALID_THRESHOLD);
            assert!(new_threshold <= owner_count(wallet), E_INVALID_THRESHOLD);
            let old = wallet.threshold;
            wallet.threshold = new_threshold;
            event::emit(ThresholdChangedEvent {
                wallet_id, old_threshold: old, new_threshold,
            });
        } else {
            abort E_INVALID_TRANSACTION_TYPE
        };

        wallet.transaction_count = wallet.transaction_count + 1;
        event::emit(TransactionExecutedEvent {
            wallet_id, transaction_id: proposal_id, executor: sender,
        });

        let TransactionProposal {
            id, wallet_id: _, tx_type: _, proposer: _, target_address: _,
            amount: _, payload: _, description: _, approvers: _,
            executed: _, created_at_ms: _, expires_at_ms: _,
        } = proposal;
        object::delete(id);
    }

    /// Regulated execution: like `execute_transaction`, but transfer proposals
    /// additionally abort `E_DENIED` when the recipient is on `deny`.
    /// Non-transfer proposals ignore the list. Use this entry point whenever
    /// the wallet custodies a regulated coin.
    public fun execute_transfer_checked<T>(
        wallet: &mut MultisigWallet<T>,
        proposal: TransactionProposal,
        deny: &DenyList,
        ctx: &mut TxContext,
    ) {
        if (proposal.tx_type == TX_TYPE_TRANSFER) {
            assert!(!deny_list::contains(deny, proposal.target_address), E_DENIED);
        };
        execute_transaction(wallet, proposal, ctx);
    }

    // --- Entry wrappers (callable via CLI/RPC `call`) ---
    //
    // The `public fun` API above moves objects by value, which the generic
    // `call` path cannot always wire up. These entry points take object IDs
    // (resolved to refs by the runtime) so every step of the wallet flow is
    // drivable from `kanari multisig ...` / `kanari call ...`.

    /// Create a wallet funded with `amount` drawn from a coin object you
    /// own. The coin stays in your wallet: `amount` is split off into the
    /// new multisig wallet, the remainder is saved back. Pass the coin's
    /// 32-byte object ID as `funds_id` (declared in `object_inputs`).
    public entry fun create_wallet_entry<T>(
        owners: vector<address>,
        threshold: u64,
        funds_id: address,
        amount: u64,
        ctx: &mut TxContext,
    ) {
        let source = object::borrow_global_mut<Coin<T>>(funds_id);
        let initial_funds = coin::split(source, amount, ctx);
        object::save_object(source);
        let wallet = create_wallet(owners, threshold, initial_funds, ctx);
        transfer::public_transfer(wallet, tx_context::sender(ctx));
    }

    /// Top up the wallet by splitting `amount` off a coin you own.
    public entry fun deposit_entry<T>(
        wallet: &mut MultisigWallet<T>,
        funds_id: address,
        amount: u64,
        ctx: &mut TxContext,
    ) {
        let source = object::borrow_global_mut<Coin<T>>(funds_id);
        let funds = coin::split(source, amount, ctx);
        object::save_object(source);
        deposit(wallet, funds, ctx);
    }

    /// Propose a transfer. The proposal object goes to the proposer.
    ///
    /// Objects arrive by ID and are borrowed inside (the runtime's escrow
    /// pattern): entry params that are object refs cannot be bound from raw
    /// address args, so every entry below takes IDs and borrows.
    public entry fun propose_transfer_entry<T>(
        wallet_id: address,
        target_address: address,
        amount: u64,
        description: vector<u8>,
        ttl_ms: u64,
        ctx: &mut TxContext,
    ) {
        let wallet = object::borrow_global<MultisigWallet<T>>(wallet_id);
        let proposal = propose_transfer(
            wallet,
            target_address,
            amount,
            string::utf8(description),
            ttl_ms,
            ctx,
        );
        transfer::public_transfer(proposal, tx_context::sender(ctx));
    }

    /// Approve someone else's proposal (proposer auto-approved at creation).
    public entry fun approve_entry<T>(
        wallet_id: address,
        proposal_id: address,
        ctx: &mut TxContext,
    ) {
        let wallet = borrow::borrow<MultisigWallet<T>>(wallet_id);
        let proposal = object::borrow_global_mut<TransactionProposal>(proposal_id);
        approve_transaction(wallet, proposal, ctx);
        // Persist the new approval: without this the borrowed mutation only
        // lives in the VM writeback set for tracked borrows and the second
        // approval is lost on commit.
        borrow::save(proposal);
    }

    /// Execute a proposal whose threshold is met. Marks the proposal executed
    /// (tombstone) instead of deleting it: entry functions cannot move a
    /// stored object by value, and the flag blocks any re-execution.
    public entry fun execute_entry<T>(
        wallet_id: address,
        proposal_id: address,
        ctx: &mut TxContext,
    ) {
        borrow::assert_distinct(wallet_id, proposal_id);
        let wallet = object::borrow_global_mut<MultisigWallet<T>>(wallet_id);
        let proposal = object::borrow_global_mut<TransactionProposal>(proposal_id);
        execute_borrowed(wallet, proposal, ctx);
        borrow::save(wallet);
        borrow::save(proposal);
    }

    /// Cancel your own live proposal. Tombstones like `execute_entry`.
    public entry fun cancel_entry<T>(
        wallet_id: address,
        proposal_id: address,
        ctx: &TxContext,
    ) {
        let wallet = borrow::borrow<MultisigWallet<T>>(wallet_id);
        let proposal = object::borrow_global_mut<TransactionProposal>(proposal_id);
        cancel_borrowed(wallet, proposal, ctx);
        borrow::save(proposal);
    }

    /// Borrowed-ref variant of `execute_transaction` for entry calls.
    /// Same checks and effects; tombstones instead of deleting.
    fun execute_borrowed<T>(
        wallet: &mut MultisigWallet<T>,
        proposal: &mut TransactionProposal,
        ctx: &mut TxContext,
    ) {
        let sender = tx_context::sender(ctx);
        assert_bound(wallet, proposal);
        assert!(is_owner(wallet, sender), E_NOT_OWNER);
        assert!(!proposal.executed, E_TRANSACTION_ALREADY_EXECUTED);
        assert!(!is_expired(proposal, ctx), E_PROPOSAL_EXPIRED);
        assert!(has_enough_approvals(wallet, proposal), E_THRESHOLD_NOT_MET);

        let wallet_id = wallet_address(wallet);
        let proposal_id = object::id_to_address(&object::uid_to_inner(&proposal.id));

        if (proposal.tx_type == TX_TYPE_TRANSFER) {
            let amount = proposal.amount;
            assert!(amount > 0, E_ZERO_AMOUNT);
            assert!(coin::value(&wallet.funds) >= amount, E_INSUFFICIENT_BALANCE);
            let out = coin::split(&mut wallet.funds, amount, ctx);
            transfer::public_transfer(out, proposal.target_address);
        } else if (proposal.tx_type == TX_TYPE_ADD_OWNER) {
            let new_owner = proposal.target_address;
            assert!(!is_owner(wallet, new_owner), E_ALREADY_OWNER);
            vector::push_back(&mut wallet.owners, new_owner);
            event::emit(OwnerChangedEvent { wallet_id, action: 0, owner: new_owner });
        } else if (proposal.tx_type == TX_TYPE_REMOVE_OWNER) {
            let doomed = proposal.target_address;
            assert!(is_owner(wallet, doomed), E_OWNER_NOT_FOUND);
            assert!(vector::length(&wallet.owners) > 1, E_CANNOT_REMOVE_LAST_OWNER);
            remove_owner_addr(&mut wallet.owners, doomed);
            assert!(owner_count(wallet) >= wallet.threshold, E_INVALID_THRESHOLD);
            event::emit(OwnerChangedEvent { wallet_id, action: 1, owner: doomed });
        } else if (proposal.tx_type == TX_TYPE_CHANGE_THRESHOLD) {
            let new_threshold = decode_threshold(&proposal.payload);
            assert!(new_threshold > 0, E_INVALID_THRESHOLD);
            assert!(new_threshold <= owner_count(wallet), E_INVALID_THRESHOLD);
            let old = wallet.threshold;
            wallet.threshold = new_threshold;
            event::emit(ThresholdChangedEvent {
                wallet_id, old_threshold: old, new_threshold,
            });
        } else {
            abort E_INVALID_TRANSACTION_TYPE
        };

        wallet.transaction_count = wallet.transaction_count + 1;
        proposal.executed = true;
        event::emit(TransactionExecutedEvent {
            wallet_id, transaction_id: proposal_id, executor: sender,
        });
    }

    /// Borrowed-ref variant of `cancel_proposal` for entry calls.
    fun cancel_borrowed<T>(
        wallet: &MultisigWallet<T>,
        proposal: &mut TransactionProposal,
        ctx: &TxContext,
    ) {
        let sender = tx_context::sender(ctx);
        assert_bound(wallet, proposal);
        assert!(sender == proposal.proposer, E_NOT_PROPOSER);
        assert!(!proposal.executed, E_TRANSACTION_ALREADY_EXECUTED);

        proposal.executed = true;
        event::emit(ProposalCancelledEvent {
            wallet_id: wallet_address(wallet),
            transaction_id: object::id_to_address(&object::uid_to_inner(&proposal.id)),
            canceller: sender,
        });
    }

    // --- Read API ---

    public fun is_owner<T>(wallet: &MultisigWallet<T>, addr: address): bool {
        let (len, i) = (vector::length(&wallet.owners), 0);
        while (i < len) {
            if (*vector::borrow(&wallet.owners, i) == addr) { return true };
            i = i + 1;
        };
        false
    }

    public fun owner_count<T>(wallet: &MultisigWallet<T>): u64 {
        (vector::length(&wallet.owners) as u64)
    }

    public fun get_threshold<T>(wallet: &MultisigWallet<T>): u64 {
        wallet.threshold
    }

    public fun get_transaction_count<T>(wallet: &MultisigWallet<T>): u64 {
        wallet.transaction_count
    }

    public fun has_enough_approvals<T>(
        wallet: &MultisigWallet<T>,
        proposal: &TransactionProposal,
    ): bool {
        (vector::length(&proposal.approvers) as u64) >= wallet.threshold
    }

    public fun get_approval_count(proposal: &TransactionProposal): u64 {
        (vector::length(&proposal.approvers) as u64)
    }

    public fun is_executed(proposal: &TransactionProposal): bool {
        proposal.executed
    }

    public fun get_proposer(proposal: &TransactionProposal): address {
        proposal.proposer
    }

    public fun get_tx_type(proposal: &TransactionProposal): u8 {
        proposal.tx_type
    }

    public fun get_target_address(proposal: &TransactionProposal): address {
        proposal.target_address
    }

    public fun get_amount(proposal: &TransactionProposal): u64 {
        proposal.amount
    }

    public fun get_description(proposal: &TransactionProposal): &string::String {
        &proposal.description
    }

    public fun get_expires_at_ms(proposal: &TransactionProposal): u64 {
        proposal.expires_at_ms
    }

    /// True when the proposal can no longer be approved or executed.
    public fun is_expired(proposal: &TransactionProposal, ctx: &TxContext): bool {
        proposal.expires_at_ms != NO_EXPIRY
            && tx_context::epoch_timestamp_ms(ctx) >= proposal.expires_at_ms
    }

    // --- Private helpers ---

    fun wallet_address<T>(wallet: &MultisigWallet<T>): address {
        object::id_to_address(&object::uid_to_inner(&wallet.id))
    }

    /// A proposal approved for one wallet must never execute against another.
    fun assert_bound<T>(wallet: &MultisigWallet<T>, proposal: &TransactionProposal) {
        assert!(
            proposal.wallet_id == object::uid_to_inner(&wallet.id),
            E_BINDING_MISMATCH,
        );
    }

    fun check_duplicate_owners(owners: &vector<address>) {
        let len = vector::length(owners);
        let i = 0;
        while (i < len) {
            let addr_i = vector::borrow(owners, i);
            let j = i + 1;
            while (j < len) {
                assert!(*addr_i != *vector::borrow(owners, j), E_INVALID_THRESHOLD);
                j = j + 1;
            };
            i = i + 1;
        };
    }

    fun has_approved(proposal: &TransactionProposal, addr: address): bool {
        let (len, i) = (vector::length(&proposal.approvers), 0);
        while (i < len) {
            if (*vector::borrow(&proposal.approvers, i) == addr) { return true };
            i = i + 1;
        };
        false
    }

    fun remove_owner_addr(owners: &mut vector<address>, doomed: address) {
        let (len, i) = (vector::length(owners), 0);
        while (i < len) {
            if (*vector::borrow(owners, i) == doomed) {
                vector::swap_remove(owners, i);
                return
            };
            i = i + 1;
        };
        abort E_OWNER_NOT_FOUND
    }

    /// Strict BCS u64 decode: exactly 8 bytes, no trailing data.
    fun decode_threshold(payload: &vector<u8>): u64 {
        assert!(vector::length(payload) == 8, E_INVALID_PAYLOAD);
        let reader = bcs::new(*payload);
        let v = bcs::peel_u64(&mut reader);
        assert!(vector::length(&bcs::into_remainder_bytes(reader)) == 0, E_INVALID_PAYLOAD);
        v
    }

    // --- Tests ---

    /// Witness for the throwaway test coin. Module-private: unusable outside.
    struct TestCoin has drop {}

    #[test_only]
    fun owners1(): vector<address> { vector::singleton(@0x1) }

    #[test_only]
    fun owners2(): vector<address> {
        let v = vector::singleton(@0x1);
        vector::push_back(&mut v, @0x2);
        v
    }

    #[test_only]
    fun owners3(): vector<address> {
        let v = owners2();
        vector::push_back(&mut v, @0x3);
        v
    }

    #[test_only]
    fun setup_funded_wallet(
        owners: vector<address>,
        threshold: u64,
        fund_amount: u64,
        ctx: &mut TxContext,
    ): (MultisigWallet<TestCoin>, coin::TreasuryCap<TestCoin>, coin::CoinMetadata<TestCoin>) {
        let (cap, meta) = coin::create_currency(
            TestCoin {},
            6,
            b"TEST",
            b"Test Coin",
            b"multisig test coin",
            std::option::none(),
            ctx,
        );
        let funds = coin::mint(&mut cap, fund_amount, ctx);
        let wallet = create_wallet(owners, threshold, funds, ctx);
        (wallet, cap, meta)
    }

    #[test_only]
    fun destroy_wallet<T>(wallet: MultisigWallet<T>) {
        let MultisigWallet<T> {
            id, owners: _, threshold: _, transaction_count: _, funds: _,
        } = wallet;
        object::delete(id);
    }

    #[test_only]
    fun destroy_proposal(proposal: TransactionProposal) {
        let TransactionProposal {
            id, wallet_id: _, tx_type: _, proposer: _, target_address: _,
            amount: _, payload: _, description: _, approvers: _,
            executed: _, created_at_ms: _, expires_at_ms: _,
        } = proposal;
        object::delete(id);
    }

    #[test_only]
    fun destroy_cap<T>(cap: coin::TreasuryCap<T>, meta: coin::CoinMetadata<T>) {
        // Capabilities live outside this module, so they cannot be
        // destructured here: freeze them instead (fine in tests).
        transfer::public_freeze_object(cap);
        transfer::public_freeze_object(meta);
    }

    #[test]
    fun test_create_and_fund() {
        let ctx = tx_context::dummy();
        let owners = owners3();
        let (wallet, cap, meta) = setup_funded_wallet(owners, 2, 10_000, &mut ctx);
        assert!(owner_count(&wallet) == 3, 0);
        assert!(get_threshold(&wallet) == 2, 1);
        assert!(balance(&wallet) == 10_000, 2);

        // Anyone (even non-owner) can top up.
        let ctx2 = tx_context::new_from_hint(@0x9, 7, 0, 0, 0);
        let (cap2, meta2) = coin::create_currency(
            TestCoin {}, 6, b"T2", b"T2", b"t2", std::option::none(), &mut ctx2,
        );
        // NOTE: fresh currency => fresh cap; mint from it for the deposit check.
        let extra = coin::mint(&mut cap2, 500, &mut ctx2);
        deposit(&mut wallet, extra, &ctx2);
        assert!(balance(&wallet) == 10_500, 3);

        destroy_wallet(wallet);
        destroy_cap(cap, meta);
        destroy_cap(cap2, meta2);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_THRESHOLD)]
    fun test_create_wallet_invalid_threshold() {
        let ctx = tx_context::dummy();
        let (wallet, cap, meta) = setup_funded_wallet(owners1(), 2, 100, &mut ctx);
        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test]
    fun test_transfer_executes_for_real() {
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, meta) =
            setup_funded_wallet(owners2(), 2, 10_000, &mut ctx1);

        let desc = string::utf8(b"pay vendor");
        let proposal = propose_transfer(&wallet, @0x999, 3_000, desc, 60_000, &mut ctx1);
        assert!(get_approval_count(&proposal) == 1, 0); // proposer auto-approves
        assert!(!has_enough_approvals(&wallet, &proposal), 1);

        // Second owner approves in their own transaction.
        let ctx2 = tx_context::new_from_hint(@0x2, 2, 0, 1001, 0);
        approve_transaction(&wallet, &mut proposal, &mut ctx2);
        assert!(has_enough_approvals(&wallet, &proposal), 2);

        // Execute drains the custodial balance, not the executor's.
        let ctx3 = tx_context::new_from_hint(@0x1, 3, 0, 1002, 0);
        execute_transaction(&mut wallet, proposal, &mut ctx3);
        assert!(balance(&wallet) == 7_000, 3);
        assert!(get_transaction_count(&wallet) == 1, 4);

        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test]
    #[expected_failure(abort_code = E_THRESHOLD_NOT_MET)]
    fun test_execute_below_threshold() {
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, meta) =
            setup_funded_wallet(owners2(), 2, 10_000, &mut ctx1);
        let proposal = propose_transfer(
            &wallet, @0x999, 100, string::utf8(b"x"), 60_000, &mut ctx1,
        );
        execute_transaction(&mut wallet, proposal, &mut ctx1);
        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test]
    #[expected_failure(abort_code = E_INSUFFICIENT_BALANCE)]
    fun test_transfer_over_balance() {
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, meta) =
            setup_funded_wallet(owners1(), 1, 100, &mut ctx1);
        let proposal = propose_transfer(
            &wallet, @0x999, 10_000, string::utf8(b"too much"), 60_000, &mut ctx1,
        );
        execute_transaction(&mut wallet, proposal, &mut ctx1);
        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test]
    #[expected_failure(abort_code = E_BINDING_MISMATCH)]
    fun test_cross_wallet_execute_rejected() {
        // Approvals for wallet A must never drain wallet B.
        let ctx = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet_a, cap_a, meta_a) =
            setup_funded_wallet(owners1(), 1, 10_000, &mut ctx);
        let (wallet_b, cap_b, meta_b) =
            setup_funded_wallet(owners1(), 1, 10_000, &mut ctx);
        let evil = propose_transfer(
            &wallet_a, @0x999, 1_000, string::utf8(b"evil"), 60_000, &mut ctx,
        );
        execute_transaction(&mut wallet_b, evil, &mut ctx);
        destroy_wallet(wallet_a);
        destroy_wallet(wallet_b);
        destroy_cap(cap_a, meta_a);
        destroy_cap(cap_b, meta_b);
    }

    #[test]
    fun test_add_and_remove_owner_take_effect() {
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, meta) =
            setup_funded_wallet(owners2(), 2, 1_000, &mut ctx1);

        // Add @0x3 (second approval from @0x2).
        let p_add = propose_add_owner(
            &wallet, @0x3, string::utf8(b"add"), 60_000, &mut ctx1,
        );
        let ctx2 = tx_context::new_from_hint(@0x2, 2, 0, 1001, 0);
        approve_transaction(&wallet, &mut p_add, &mut ctx2);
        execute_transaction(&mut wallet, p_add, &mut ctx2);
        assert!(owner_count(&wallet) == 3, 0);
        assert!(is_owner(&wallet, @0x3), 1);

        // Remove @0x3 again.
        let p_del = propose_remove_owner(
            &wallet, @0x3, string::utf8(b"del"), 60_000, &mut ctx2,
        );
        let ctx3 = tx_context::new_from_hint(@0x1, 3, 0, 1002, 0);
        approve_transaction(&wallet, &mut p_del, &mut ctx3);
        execute_transaction(&mut wallet, p_del, &mut ctx3);
        assert!(owner_count(&wallet) == 2, 2);
        assert!(!is_owner(&wallet, @0x3), 3);

        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test]
    #[expected_failure(abort_code = E_ALREADY_OWNER)]
    fun test_double_add_owner_rejected_at_execute() {
        // Second add of the same owner aborts even though propose passed
        // while the first proposal was still pending.
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, meta) =
            setup_funded_wallet(owners1(), 1, 1_000, &mut ctx1);
        let p1 = propose_add_owner(&wallet, @0x2, string::utf8(b"a"), 60_000, &mut ctx1);
        let p2 = propose_add_owner(&wallet, @0x2, string::utf8(b"b"), 60_000, &mut ctx1);
        execute_transaction(&mut wallet, p1, &mut ctx1);
        execute_transaction(&mut wallet, p2, &mut ctx1);
        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test]
    #[expected_failure(abort_code = E_INVALID_THRESHOLD)]
    fun test_remove_owner_that_breaks_threshold() {
        // 2-of-2 wallet: removing anyone leaves threshold unreachable.
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, meta) =
            setup_funded_wallet(owners2(), 2, 1_000, &mut ctx1);
        let p = propose_remove_owner(&wallet, @0x2, string::utf8(b"del"), 60_000, &mut ctx1);
        let ctx2 = tx_context::new_from_hint(@0x2, 2, 0, 1001, 0);
        approve_transaction(&wallet, &mut p, &mut ctx2);
        execute_transaction(&mut wallet, p, &mut ctx2);
        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test]
    #[expected_failure(abort_code = E_ZERO_ADDRESS)]
    fun test_create_wallet_rejects_zero_address_owner() {
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, meta) =
            setup_funded_wallet(vector::singleton(@0x0), 1, 1_000, &mut ctx1);
        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test]
    #[expected_failure(abort_code = E_ZERO_ADDRESS)]
    fun test_propose_add_owner_rejects_zero_address() {
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, meta) =
            setup_funded_wallet(owners1(), 1, 1_000, &mut ctx1);
        let p = propose_add_owner(&wallet, @0x0, string::utf8(b"nope"), 60_000, &mut ctx1);
        // Unreachable: `propose_add_owner` aborts on the zero address.
        cancel_proposal(&wallet, p, &ctx1);
        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test]
    fun test_change_threshold_takes_effect() {
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, meta) =
            setup_funded_wallet(owners3(), 2, 1_000, &mut ctx1);
        let p = propose_change_threshold(
            &wallet, 3, string::utf8(b"stricter"), 60_000, &mut ctx1,
        );
        let ctx2 = tx_context::new_from_hint(@0x2, 2, 0, 1001, 0);
        approve_transaction(&wallet, &mut p, &mut ctx2);
        execute_transaction(&mut wallet, p, &mut ctx2);
        assert!(get_threshold(&wallet) == 3, 0);
        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test]
    fun test_cancel_and_expiry() {
        // Proposal with 1s TTL created at t=1000.
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, meta) =
            setup_funded_wallet(owners2(), 2, 1_000, &mut ctx1);
        let p_live = propose_transfer(
            &wallet, @0x999, 100, string::utf8(b"live"), 60_000, &mut ctx1,
        );
        assert!(!is_expired(&p_live, &ctx1), 0);
        // Proposer cancels.
        cancel_proposal(&wallet, p_live, &ctx1);
        assert!(balance(&wallet) == 1_000, 1); // funds untouched

        // Short-lived proposal, executed after expiry -> anyone can GC it.
        let p_short = propose_transfer(
            &wallet, @0x999, 100, string::utf8(b"short"), 1, &mut ctx1,
        );
        let ctx_late = tx_context::new_from_hint(@0x9, 9, 0, 5000, 0);
        assert!(is_expired(&p_short, &ctx_late), 2);
        delete_expired_proposal(p_short, &ctx_late);

        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test]
    #[expected_failure(abort_code = E_PROPOSAL_EXPIRED)]
    fun test_approve_after_expiry() {
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, meta) =
            setup_funded_wallet(owners2(), 2, 1_000, &mut ctx1);
        let p = propose_transfer(
            &wallet, @0x999, 100, string::utf8(b"s"), 1, &mut ctx1,
        );
        let ctx_late = tx_context::new_from_hint(@0x2, 2, 0, 5000, 0);
        approve_transaction(&wallet, &mut p, &mut ctx_late);
        destroy_proposal(p);
        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test]
    #[expected_failure(abort_code = E_NOT_PROPOSER)]
    fun test_cancel_by_non_proposer() {
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, meta) =
            setup_funded_wallet(owners2(), 2, 1_000, &mut ctx1);
        let p = propose_transfer(
            &wallet, @0x999, 100, string::utf8(b"x"), 60_000, &mut ctx1,
        );
        let ctx2 = tx_context::new_from_hint(@0x2, 2, 0, 1001, 0);
        cancel_proposal(&wallet, p, &ctx2);
        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test]
    #[expected_failure(abort_code = E_PROPOSAL_EXPIRED)]
    fun test_execute_after_expiry() {
        // Fully approved, but the TTL lapsed before execution.
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, meta) = setup_funded_wallet(owners2(), 2, 1_000, &mut ctx1);
        let p = propose_transfer(
            &wallet, @0x999, 100, string::utf8(b"s"), 1, &mut ctx1,
        );
        let ctx2 = tx_context::new_from_hint(@0x2, 2, 0, 1001, 0);
        approve_transaction(&wallet, &mut p, &mut ctx2);
        assert!(has_enough_approvals(&wallet, &p), 0);
        let ctx_late = tx_context::new_from_hint(@0x1, 3, 0, 5000, 0);
        execute_transaction(&mut wallet, p, &mut ctx_late);
        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test]
    #[expected_failure(abort_code = E_THRESHOLD_NOT_MET)]
    fun test_threshold_raise_invalidates_pending() {
        // Proposal gathered 2 approvals under threshold 2, but the threshold
        // was raised to 3 before execution: the threshold is evaluated at
        // execution time, so the stale approvals no longer suffice.
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, meta) = setup_funded_wallet(owners3(), 2, 10_000, &mut ctx1);
        let p_pay = propose_transfer(
            &wallet, @0x999, 100, string::utf8(b"pay"), 600_000, &mut ctx1,
        );
        let ctx2 = tx_context::new_from_hint(@0x2, 2, 0, 1001, 0);
        approve_transaction(&wallet, &mut p_pay, &mut ctx2);
        assert!(has_enough_approvals(&wallet, &p_pay), 0);

        // Raise threshold 2 -> 3 (approvals from @0x1 and @0x3).
        let p_cfg = propose_change_threshold(
            &wallet, 3, string::utf8(b"stricter"), 600_000, &mut ctx1,
        );
        approve_transaction(&wallet, &mut p_cfg, &mut ctx2);
        let ctx3 = tx_context::new_from_hint(@0x3, 3, 0, 1002, 0);
        approve_transaction(&wallet, &mut p_cfg, &mut ctx3);
        execute_transaction(&mut wallet, p_cfg, &mut ctx3);
        assert!(get_threshold(&wallet) == 3, 1);

        // The payment now needs 3 approvals but only has 2.
        let ctx4 = tx_context::new_from_hint(@0x1, 4, 0, 1003, 0);
        execute_transaction(&mut wallet, p_pay, &mut ctx4);
        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test]
    #[expected_failure(abort_code = E_ALREADY_APPROVED)]
    fun test_double_approve_rejected() {
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, meta) =
            setup_funded_wallet(owners2(), 2, 1_000, &mut ctx1);
        let p = propose_transfer(
            &wallet, @0x999, 100, string::utf8(b"x"), 60_000, &mut ctx1,
        );
        approve_transaction(&wallet, &mut p, &mut ctx1); // proposer already approved
        destroy_proposal(p);
        destroy_wallet(wallet);
        destroy_cap(cap, meta);
    }

    #[test_only]
    fun setup_regulated_wallet(
        owners: vector<address>,
        threshold: u64,
        fund_amount: u64,
        ctx: &mut TxContext,
    ): (
        MultisigWallet<TestCoin>,
        coin::TreasuryCap<TestCoin>,
        deny_list::DenyCap<TestCoin>,
        deny_list::DenyList,
        coin::CoinMetadata<TestCoin>,
    ) {
        let (cap, denycap, meta) = coin::create_regulated_currency(
            TestCoin {},
            6,
            b"REG",
            b"Regulated",
            b"multisig regulated tests",
            std::option::none(),
            ctx,
        );
        let funds = coin::mint(&mut cap, fund_amount, ctx);
        let wallet = create_wallet(owners, threshold, funds, ctx);
        let deny = deny_list::new_denylist();
        (wallet, cap, denycap, deny, meta)
    }

    #[test]
    fun test_regulated_execute_allowed() {
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, denycap, deny, meta) =
            setup_regulated_wallet(owners1(), 1, 1_000, &mut ctx1);
        deny_list::deny_list_add(&mut deny, &denycap, @0xBAD, &mut ctx1);
        let p = propose_transfer(
            &wallet, @0xCAFE, 100, string::utf8(b"ok"), 60_000, &mut ctx1,
        );
        execute_transfer_checked(&mut wallet, p, &deny, &mut ctx1);
        assert!(balance(&wallet) == 900, 0);
        destroy_wallet(wallet);
        destroy_cap(cap, meta);
        transfer::public_freeze_object(denycap);
    }

    #[test]
    #[expected_failure(abort_code = E_DENIED)]
    fun test_regulated_execute_denied() {
        let ctx1 = tx_context::new_from_hint(@0x1, 1, 0, 1000, 0);
        let (wallet, cap, denycap, deny, meta) =
            setup_regulated_wallet(owners1(), 1, 1_000, &mut ctx1);
        deny_list::deny_list_add(&mut deny, &denycap, @0xBAD, &mut ctx1);
        let p = propose_transfer(
            &wallet, @0xBAD, 100, string::utf8(b"no"), 60_000, &mut ctx1,
        );
        execute_transfer_checked(&mut wallet, p, &deny, &mut ctx1);
        destroy_wallet(wallet);
        destroy_cap(cap, meta);
        transfer::public_freeze_object(denycap);
    }
}
