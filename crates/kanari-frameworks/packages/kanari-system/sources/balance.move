// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0
module kanari_system::balance {

    /// Error codes
    const ERR_INSUFFICIENT_BALANCE: u64 = 1;
    const ERR_OVERFLOW: u64 = 2;
    const ERR_ZERO_AMOUNT: u64 = 3;
    const ERR_INSUFFICIENT_SUPPLY: u64 = 4; // New: Distinguish supply vs balance errors

    /// Balance resource - Stores the balance value (generic per token type)
    /// REMOVED `drop` ability to enforce explicit destruction via `destroy()`
    struct Balance<phantom T> has store {
        value: u64
    }

    /// Supply: mutable minting handle consumed to create balances
    /// REMOVED `drop` ability to prevent accidental loss of minting authority
    struct Supply<phantom T> has store {
        total: u64
    }

    /// Create a new zero-value Balance
    public fun zero<T>(): Balance<T> {
        Balance<T> { value: 0 }
    }

    /// Create a new Balance with an initial value
    public fun create<T>(value: u64): Balance<T> {
        Balance<T> { value }
    }

    /// Get the current balance value
    public fun value<T>(balance: &Balance<T>): u64 {
        balance.value
    }

    /// Increase the balance value
    public fun increase<T>(balance: &mut Balance<T>, amount: u64) {
        assert!(amount > 0, ERR_ZERO_AMOUNT); // FIXED: Added zero check
        // Pre-check: `+` traps as an arithmetic error before the assert below runs.
        assert!(
            amount <= 18446744073709551615 - balance.value, ERR_OVERFLOW
        );
        let new_value = balance.value + amount;
        assert!(new_value >= balance.value, ERR_OVERFLOW);
        balance.value = new_value;
    }

    /// Decrease the balance value
    public fun decrease<T>(balance: &mut Balance<T>, amount: u64) {
        assert!(amount > 0, ERR_ZERO_AMOUNT);
        assert!(balance.value >= amount, ERR_INSUFFICIENT_BALANCE);
        balance.value = balance.value - amount;
    }

    /// Transfer value from one Balance to another
    public fun transfer<T>(
        from: &mut Balance<T>, to: &mut Balance<T>, amount: u64
    ) {
        assert!(amount > 0, ERR_ZERO_AMOUNT);
        decrease<T>(from, amount);
        increase<T>(to, amount);
    }

    /// Check if the balance is sufficient for a given amount
    public fun has_sufficient<T>(balance: &Balance<T>, amount: u64): bool {
        balance.value >= amount
    }

    /// Destroy the Balance and return its value
    public fun destroy<T>(balance: Balance<T>): u64 {
        let Balance { value } = balance;
        value
    }

    /// Create a new (empty) supply handle
    public fun new_supply<T>(): Supply<T> {
        Supply<T> { total: 0 }
    }

    /// Increase supply: add `amount` to `s` and return a `Balance` for the newly minted amount.
    public fun increase_supply<T>(s: &mut Supply<T>, amount: u64): Balance<T> {
        assert!(amount > 0, ERR_ZERO_AMOUNT);
        // Pre-check: `+` traps as an arithmetic error before the assert below runs.
        assert!(
            amount <= 18446744073709551615 - s.total, ERR_OVERFLOW
        );
        let new_total = s.total + amount;
        assert!(new_total >= s.total, ERR_OVERFLOW);
        s.total = new_total;
        create<T>(amount)
    }

    // REMOVED: destroy_supply<T>(s: Supply<T>)
    // Reason: Dangerous function allowing destruction of Supply cap with non-zero total.
    // Supply destruction should only happen at module initialization cleanup (if ever)
    // and must be handled explicitly, not via a public helper.

    // Test-only supply cleanup: unpacks the handle explicitly.
    #[test_only]
    public fun destroy_supply_for_testing<T>(s: Supply<T>) {
        let Supply { total: _ } = s;
    }

    /// Decrease supply by `amount`. Useful for burning coins.
    /// Made internal logic tighter. Consider making this `public(friend)`
    /// if only `coin` module should call it.
    public fun decrease_supply<T>(s: &mut Supply<T>, amount: u64) {
        assert!(amount > 0, ERR_ZERO_AMOUNT);
        assert!(s.total >= amount, ERR_INSUFFICIENT_SUPPLY); // FIXED: Specific error code
        s.total = s.total - amount;
    }

    /// Read the current total supply value from a supply handle.
    public fun supply_total<T>(s: &Supply<T>): u64 {
        s.total
    }

    /// Merge two Balances together
    public fun merge<T>(dst: &mut Balance<T>, src: Balance<T>) {
        let value = destroy<T>(src);
        // Zero-amount merge is a no-op: `increase` rejects 0, but joining
        // empty coins (e.g. `coin::join` of zero coins) must stay legal.
        if (value > 0) {
            increase<T>(dst, value);
        };
    }

    /// Split the Balance into two
    public fun split<T>(balance: &mut Balance<T>, amount: u64): Balance<T> {
        decrease<T>(balance, amount);
        create<T>(amount)
    }

    // ==========================================
    // Tests
    // ==========================================
    #[test]
    fun test_balance_operations() {
        let balance = create<u8>(1000); // ลบ mut ออก
        assert!(value(&balance) == 1000, 0);

        increase<u8>(&mut balance, 500); // ส่ง &mut ได้ตามปกติ
        assert!(value(&balance) == 1500, 1);

        decrease<u8>(&mut balance, 300);
        assert!(value(&balance) == 1200, 2);

        let final_value = destroy<u8>(balance);
        assert!(final_value == 1200, 3);
    }

    #[test]
    fun test_transfer() {
        let balance1 = create<u8>(1000);
        let balance2 = create<u8>(500);

        transfer<u8>(&mut balance1, &mut balance2, 300);

        assert!(value(&balance1) == 700, 0);
        assert!(value(&balance2) == 800, 1);

        destroy<u8>(balance1);
        destroy<u8>(balance2);
    }

    #[test]
    #[expected_failure(abort_code = ERR_ZERO_AMOUNT)]
    fun test_zero_amount_increase() {
        let balance = create<u8>(100);
        increase<u8>(&mut balance, 0);
        destroy<u8>(balance);
    }

    #[test]
    fun test_supply_operations() {
        let s = new_supply<u8>();
        // ต้องรับค่า Balance ที่ถูก Mint ออกมาเสมอ เพราะไม่มี drop ability
        let b1 = increase_supply<u8>(&mut s, 1000);
        assert!(supply_total(&s) == 1000, 0);

        let b2 = increase_supply<u8>(&mut s, 500);
        assert!(value(&b2) == 500, 1);
        assert!(supply_total(&s) == 1500, 2);

        decrease_supply<u8>(&mut s, 800);
        assert!(supply_total(&s) == 700, 3);

        destroy<u8>(b1); // ทำลายทิ้งให้ถูกต้อง
        destroy<u8>(b2);
        destroy_supply_for_testing(s);
    }

    #[test]
    #[expected_failure(abort_code = ERR_OVERFLOW)]
    fun test_supply_overflow() {
        let s = new_supply<u64>();
        // ใช้ค่าตัวเลขสูงสุดของ u64 แทน u64::MAX
        let b1 = increase_supply<u64>(&mut s, 18446744073709551615);
        let b2 = increase_supply<u64>(&mut s, 1);
        destroy<u64>(b1);
        destroy<u64>(b2);
        // Unreachable at runtime (aborts above), but satisfies the borrow checker.
        destroy_supply_for_testing(s);
    }
}

