module tester::gas_meter_hash_e2e {
    use std::hash;

    public entry fun hash_once(data: vector<u8>) {
        let _digest = hash::sha2_256(data);
    }
}

