/// Negative fixture for `pet_stats_forgery_is_locked`.
///
/// `kanari_pet::pet::Stats` carries the upgrade economics: `base_cost_usd`,
/// `power_per_level`, `max_level`. Both `upgrade` and `mint_with_level` take a
/// `&Stats`, so if a caller could construct their own they would choose the
/// economics — a one-unit price, a million power per level, no ceiling — and
/// buy arbitrary progression for pennies.
///
/// `new_stats` is therefore `#[test_only]`; `init` is the only thing that
/// builds a `Stats`, which makes the shared object the single source of truth.
///
/// This module tries the attack directly: build a forged `Stats` and upgrade a
/// pet against it. It MUST NOT compile.
module tester::pet_stats_forgery {
    use kanari_pet::pet::{Self, KanariPet, Stats};
    use kanari_system::coin::Coin;
    use kanari_system::tx_context::TxContext;
    use kanari_system::usd::USD;

    public entry fun forge_and_upgrade(
        pet: &mut KanariPet,
        payment: &mut Coin<USD>,
        ctx: &mut TxContext,
    ) {
        // One micro-USD per level, a million power per level, no ceiling.
        let forged: Stats = pet::new_stats(@0xBAD, 1, 1_000_000, 9_999, ctx);
        pet::upgrade(pet, &forged, payment, ctx);
    }
}