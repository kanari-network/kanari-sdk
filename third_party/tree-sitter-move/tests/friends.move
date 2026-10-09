module test::friends {
    friend test::helper;

    public struct Vault has key {
        id: UID,
        balance: u64,
    }

    public(friend) fun deposit(vault: &mut Vault, amount: u64) {
        vault.balance = vault.balance + amount;
    }

    fun helper_only(): u64 {
        42
    }
}
