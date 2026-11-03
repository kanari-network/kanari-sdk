#[test_only]
module escrow::escrow_tests {
    use escrow::escrow::{Self};
    use kanari_system::coin::{Self, Coin};
    use kanari_system::transfer;
    use kanari_system::tx_context::{Self, TxContext};
    use std::string;
    use std::option;

    #[test_only]
    public struct TestUSDC has drop {}

    #[test_only]
    public fun create_test_usdc_coin(amount: u64, ctx: &mut TxContext): Coin<TestUSDC> {
        let witness = TestUSDC {};
        let (mut treasury_cap, metadata) = coin::create_currency<TestUSDC>(
            witness,
            6,
            b"TestUSDC",
            b"Test USDC",
            b"Test USDC for escrow testing",
            option::none(),
            ctx,
        );
        let coin = coin::mint(&mut treasury_cap, amount, ctx);
        transfer::public_transfer(treasury_cap, @0x1);
        transfer::public_transfer(metadata, @0x1);
        coin
    }

    #[test]
    fun test_create_deal_with_usdc_basic() {
        let mut ctx = tx_context::dummy();
        let seller_addr = @0x2;
        let mut buyer_coin = create_test_usdc_coin(10000, &mut ctx);
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"deal_001"),
            seller_addr,
            1000,
            string::utf8(b"Test payment for services"),
            &mut buyer_coin,
            &mut ctx
        );
        transfer::public_transfer(buyer_coin, @0x1);
    }

    #[test]
    fun test_create_multiple_deals_with_usdc() {
        let mut ctx = tx_context::dummy();
        let seller_addr = @0x2;
        let mut buyer_coin = create_test_usdc_coin(10000, &mut ctx);
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"deal_001"),
            seller_addr,
            1000,
            string::utf8(b"First deal"),
            &mut buyer_coin,
            &mut ctx
        );
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b"deal_002"),
            seller_addr,
            2000,
            string::utf8(b"Second deal"),
            &mut buyer_coin,
            &mut ctx
        );
        transfer::public_transfer(buyer_coin, @0x1);
    }

    #[test]
    fun test_empty_strings() {
        let mut ctx = tx_context::dummy();
        let seller_addr = @0x2;
        let mut buyer_coin = create_test_usdc_coin(1000, &mut ctx);
        escrow::create_deal_from_coin<TestUSDC>(
            string::utf8(b""),
            seller_addr,
            1000,
            string::utf8(b""),
            &mut buyer_coin,
            &mut ctx
        );
        transfer::public_transfer(buyer_coin, @0x1);
    }
}
