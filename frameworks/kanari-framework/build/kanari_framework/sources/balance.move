module kanari_framework::balance {
    // Error constants
    const ENonZero: u64 = 0;
    const EOverflow: u64 = 1;
    const ENotEnough: u64 = 2;

    // Core structs
    struct Supply<phantom T> has store {
        value: u64
    }

    struct Balance<phantom T> has store {
        value: u64
    }

    // === Core Balance Operations ===
    
    public fun value<T>(self: &Balance<T>): u64 {
        self.value
    }

    public fun supply_value<T>(supply: &Supply<T>): u64 {
        supply.value
    }

    public fun zero<T>(): Balance<T> {
        Balance { value: 0 }
    }

    public fun join<T>(self: &mut Balance<T>, balance: Balance<T>): u64 {
        let Balance { value } = balance;
        self.value = self.value + value;
        self.value
    }

    public fun split<T>(self: &mut Balance<T>, value: u64): Balance<T> {
        assert!(self.value >= value, ENotEnough);
        self.value = self.value - value;
        Balance { value }
    }

    // === Supply Management ===

    public fun create_supply<T: drop>(_: T): Supply<T> {
        Supply { value: 0 }
    }

    public fun increase_supply<T>(self: &mut Supply<T>, value: u64): Balance<T> {
        assert!(value < (18446744073709551615u64 - self.value), EOverflow);
        self.value = self.value + value;
        Balance { value }
    }

    public fun decrease_supply<T>(self: &mut Supply<T>, balance: Balance<T>): u64 {
        let Balance { value } = balance;
        assert!(self.value >= value, EOverflow);
        self.value = self.value - value;
        value
    }

    // === Utility Functions ===

    public fun withdraw_all<T>(self: &mut Balance<T>): Balance<T> {
        let value = self.value;
        split(self, value)
    }

    public fun destroy_zero<T>(balance: Balance<T>) {
        assert!(balance.value == 0, ENonZero);
        let Balance { value: _ } = balance;
    }











    #[test_only]
    public fun create_for_testing<T>(value: u64): Balance<T> {
        Balance { value }
    }

    #[test_only]
    public fun destroy_for_testing<T>(self: Balance<T>): u64 {
        let Balance { value } = self;
        value
    }
}