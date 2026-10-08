module tester::gas_meter_event_e2e {
    use kanari_system::event;

    public entry fun emit_blob(data: vector<u8>) {
        event::emit<vector<u8>>(data);
    }
}

