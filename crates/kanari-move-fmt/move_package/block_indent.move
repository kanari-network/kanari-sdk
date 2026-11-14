module M::b { #[test] fun test_option() {
        {
            let value = option::some(true);
            let bytes = new(to_bytes(&value));
            assert!(value == peel_option_bool(&mut bytes), 0);
        };
    } }
