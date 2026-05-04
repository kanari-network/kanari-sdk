module kanari_escrow::escrow {
    use std::string::{Self, String};
    use std::vector;
    use std::option::{Self, Option};
    use kanari_system::coin::{Self, Coin};
    use kanari_system::event;
    use kanari_system::tx_context::{Self, TxContext};
    use kanari_system::transfer;
    use kanari_system::object;

    // ─── Error codes ────────────────────────────────────────────────
    const E_NOT_BUYER:         u64 = 1;
    const E_NOT_SELLER:        u64 = 2;
    const E_WRONG_STATE:       u64 = 3;
    const E_NOT_AUTHORIZED:    u64 = 6;

    // ─── Deal states ─────────────────────────────────────────────────
    const STATE_LOCKED:     u8 = 1; // crypto locked, waiting delivery
    const STATE_DELIVERED:  u8 = 2; // seller confirmed delivery
    const STATE_COMPLETED:  u8 = 3; // buyer confirmed, funds released
    const STATE_DISPUTED:   u8 = 4; // dispute raised

    // ─── Structs ─────────────────────────────────────────────────────

    // Core escrow deal — stored under buyer's address
    struct EscrowDeal<phantom CoinType> has key, store {
        id:             object::UID,
        deal_id:        String,
        buyer:          address,
        seller:         address,
        amount:         u64,
        description:    String,
        state:          u8,
        locked_at:      u64,
        delivered_at:   u64,
        completed_at:   u64,
        funds:          Option<Coin<CoinType>>,
    }

    // On-chain proof record — Kanari metadata layer
    // Every state transition appends a ProofEntry here
    struct EscrowProof has key, store {
        id:         object::UID,
        deal_id:    String,
        entries:    vector<ProofEntry>,
    }

    struct ProofEntry has copy, drop, store {
        state:      u8,
        actor:      address,
        timestamp:  u64,
        note:       String,
    }

    // ─── Events ──────────────────────────────────────────────────────
    // Deal creation event
    struct DealCreated has copy, drop, store {
        deal_id:    String,
        buyer:      address,
        seller:      address,
        amount:     u64,
    }

    // State change event
    struct DealStateChanged has copy, drop, store {
        deal_id:    String,
        old_state:  u8,
        new_state:  u8,
        actor:      address,
        timestamp:  u64,
    }

    // ─── Functions ───────────────────────────────────────────────────

    // Step 1: Buyer creates deal and locks crypto
    public entry fun create_deal<CoinType>(
        deal_id:        String,
        seller:         address,
        amount:         u64,
        description:    String,
        buyer_coin:     &mut Coin<CoinType>,
        ctx:            &mut TxContext,
    ) {
        let buyer_addr = tx_context::sender(ctx);
        let now = now_ms(ctx);
        let deal_id_copy = copy deal_id;
        let funds = coin::split(buyer_coin, amount, ctx);

        // Store deal under a fresh object address (not buyer's address directly)
        let deal = EscrowDeal<CoinType> {
            id:             object::new(ctx),
            deal_id,
            buyer:          buyer_addr,
            seller,
            amount,
            description,
            state:          STATE_LOCKED,
            locked_at:      now,
            delivered_at:   0,
            completed_at:   0,
            funds: option::some(funds),
        };

        // Transfer ownership to buyer
        transfer::public_transfer(deal, buyer_addr);

        // Create proof record
        let entry = ProofEntry {
            state:      STATE_LOCKED,
            actor:      buyer_addr,
            timestamp:  now,
            note:       string::utf8(b"Deal created — funds locked in escrow"),
        };
        let entries = vector::empty<ProofEntry>();
        vector::push_back(&mut entries, entry);

        let proof = EscrowProof {
            id:         object::new(ctx),
            deal_id:    deal_id_copy,
            entries,
        };

        // Transfer proof to buyer
        transfer::public_transfer(proof, buyer_addr);

        // Emit event
        event::emit(DealCreated {
            deal_id: deal_id_copy,
            buyer: buyer_addr,
            seller,
            amount,
        });
    }

    // Helper function to get deal_id from deal reference
    fun get_deal_id<CoinType>(deal: &EscrowDeal<CoinType>): String {
        let EscrowDeal { 
            id: _, 
            deal_id, 
            buyer: _, 
            seller: _, 
            amount: _, 
            description: _, 
            state: _, 
            locked_at: _, 
            delivered_at: _, 
            completed_at: _, 
            funds: _ 
        } = deal;
        *deal_id
    }

    // Step 2: Seller confirms delivery
    public entry fun confirm_delivery<CoinType>(
        deal:           &mut EscrowDeal<CoinType>,
        proof:          &mut EscrowProof,
        ctx:            &mut TxContext,
    ) {
        let seller_addr = tx_context::sender(ctx);
        
        assert!(deal.seller == seller_addr, E_NOT_SELLER);
        assert!(deal.state == STATE_LOCKED, E_WRONG_STATE);

        let now = now_ms(ctx);
        let old_state = deal.state;
        deal.state = STATE_DELIVERED;
        deal.delivered_at = now;
        let deal_id = get_deal_id(deal);

        // Append proof
        append_proof(proof, STATE_DELIVERED, seller_addr, now,
            string::utf8(b"Seller confirmed delivery"));

        event::emit(DealStateChanged {
            deal_id,
            old_state,
            new_state:  STATE_DELIVERED,
            actor:      seller_addr,
            timestamp:  now,
        });
    }

    // Step 3: Buyer confirms receipt → funds auto-released to seller
    public entry fun release_funds<CoinType>(
        deal:           &mut EscrowDeal<CoinType>,
        proof:          &mut EscrowProof,
        ctx:            &mut TxContext,
    ) {
        let buyer_addr = tx_context::sender(ctx);
        
        assert!(deal.buyer == buyer_addr, E_NOT_BUYER);
        assert!(deal.state == STATE_DELIVERED, E_WRONG_STATE);

        // Get current timestamp using transaction context
        let now = now_ms(ctx);
        let old_state = deal.state;
        let seller = deal.seller;
        deal.state = STATE_COMPLETED;
        deal.completed_at = now;
        let deal_id = get_deal_id(deal);

        // Extract funds from Option
        let funds = option::extract(&mut deal.funds);
        
        let balance = coin::into_balance(funds);
        let seller_coin = coin::from_balance(balance, ctx);
        transfer::public_transfer(seller_coin, seller);

        append_proof(proof, STATE_COMPLETED, buyer_addr, now,
            string::utf8(b"Buyer confirmed — funds released to seller"));

        event::emit(DealStateChanged {
            deal_id,
            old_state,
            new_state:  STATE_COMPLETED,
            actor:      buyer_addr,
            timestamp:  now,
        });
    }

    // Raise dispute (either party)
    public entry fun raise_dispute<CoinType>(
        deal:           &mut EscrowDeal<CoinType>,
        proof:          &mut EscrowProof,
        ctx:            &mut TxContext,
    ) {
        let caller_addr = tx_context::sender(ctx);
        
        assert!(
            caller_addr == deal.buyer || caller_addr == deal.seller,
            E_NOT_AUTHORIZED
        );
        assert!(
            deal.state == STATE_LOCKED || deal.state == STATE_DELIVERED,
            E_WRONG_STATE
        );

        let now = now_ms(ctx);
        let old_state = deal.state;
        deal.state = STATE_DISPUTED;
        let deal_id = get_deal_id(deal);

        append_proof(proof, STATE_DISPUTED, caller_addr, now,
            string::utf8(b"Dispute raised — awaiting resolution"));

        event::emit(DealStateChanged {
            deal_id,
            old_state,
            new_state:  STATE_DISPUTED,
            actor:      caller_addr,
            timestamp:  now,
        });
    }

    // View: get current deal state (returns u8)
    public fun get_state<CoinType>(deal: &EscrowDeal<CoinType>): u8 {
        deal.state
    }

    // View: get proof entry count
    public fun get_proof_count(proof: &EscrowProof): u64 {
        vector::length(&proof.entries)
    }

    // ─── Internal helpers ────────────────────────────────────────────

    /// Get current timestamp from transaction context
    /// Note: This uses epoch_timestamp_ms from TxContext which provides 
    /// the same functionality as clock::timestamp_ms but is simpler to use
    fun now_ms(ctx: &TxContext): u64 {
        tx_context::epoch_timestamp_ms(ctx)
    }

    fun append_proof(
        proof:      &mut EscrowProof,
        state:      u8,
        actor:      address,
        timestamp:  u64,
        note:       String,
    ) {
        vector::push_back(&mut proof.entries, ProofEntry {
            state, actor, timestamp, note,
        });
    }
}
