/// Negative fixture for `deny_list_denycap_minting_is_locked`.
///
/// Every deny-list write path requires `&DenyCap<T>`, so a publicly mintable
/// capability is equivalent to no gate at all: this package mints its own
/// `DenyCap<T>` and could then freeze arbitrary addresses for arbitrary coin
/// types (e.g. KANARI) without ever being the currency issuer.
///
/// `deny_list::new_denycap` must therefore be `public(friend)` with
/// `kanari_system::coin` as the sole friend — the framework mints it inside
/// `coin::create_regulated_currency` and nowhere else.
///
/// This package MUST NOT compile.
module tester::deny_list_cap_forgery {
    use kanari_system::deny_list::{Self, DenyCap};
    use kanari_system::tx_context::TxContext;

    public entry fun forge_denycap<T>(ctx: &mut TxContext): DenyCap<T> {
        deny_list::new_denycap<T>(ctx)
    }
}
