#[cfg(test)]
mod pending_objects_tests {
    use crate::move_runtime::MoveRuntime;
    use crate::move_vm_state::MoveVMState;
    use crate::objects::pending_objects::new_pending_ops;
    use crate::objects::{ObjectTransfer, PendingObjectOps};
    use move_core_types::account_address::AccountAddress;
    use move_vm_runtime::move_vm::MoveVM;
    use move_vm_test_utils::InMemoryStorage;
    use std::collections::HashSet;

    /// Helper to create in-memory runtime for testing (no DB access)
    fn create_test_runtime() -> MoveRuntime {
        let storage = InMemoryStorage::new();
        let vm = MoveVM::new(vec![]).expect("Failed to create MoveVM");
        let state = MoveVMState::new_in_memory().expect("Failed to create in-memory state");

        MoveRuntime {
            vm,
            storage,
            state,
            enable_gas_metering: false,
            published_modules: HashSet::new(),
            pending_objects: new_pending_ops(),
        }
    }

    #[test]
    fn test_clear_pending_objects() {
        let mut runtime = create_test_runtime();

        // Add some pending operations
        let object_id = vec![1, 2, 3, 4];
        let object_type = "TestObject".to_string();
        let object_data = vec![5, 6, 7, 8];
        let recipient = AccountAddress::from_hex_literal("0x1").unwrap();

        runtime.add_pending_transfer(object_id, object_type, object_data, recipient);

        // Verify operation was added
        let pending = runtime.get_pending_objects();
        assert_eq!(pending.transfers.len(), 1);

        // Clear operations
        runtime.clear_pending_objects();

        // Verify cleared
        let pending_after = runtime.get_pending_objects();
        assert_eq!(pending_after.transfers.len(), 0);
        assert!(pending_after.is_empty());
    }

    #[test]
    fn test_get_pending_objects_non_destructive() {
        let mut runtime = create_test_runtime();

        // Add operation
        runtime.add_pending_transfer(
            vec![1, 2, 3],
            "TestType".to_string(),
            vec![4, 5, 6],
            AccountAddress::from_hex_literal("0x42").unwrap(),
        );

        // Get operations (should not remove)
        let pending1 = runtime.get_pending_objects();
        assert_eq!(pending1.transfers.len(), 1);

        // Get again (should still be there)
        let pending2 = runtime.get_pending_objects();
        assert_eq!(pending2.transfers.len(), 1);
    }

    #[test]
    fn test_take_pending_objects_clears() {
        let mut runtime = create_test_runtime();

        // Add operation
        runtime.add_pending_transfer(
            vec![1, 2, 3],
            "TestType".to_string(),
            vec![4, 5, 6],
            AccountAddress::from_hex_literal("0x99").unwrap(),
        );

        // Verify exists
        assert_eq!(runtime.get_pending_objects().transfers.len(), 1);

        // Take operations (should clear)
        let taken = runtime.take_pending_objects();
        assert_eq!(taken.transfers.len(), 1);

        // Verify cleared
        let remaining = runtime.get_pending_objects();
        assert_eq!(remaining.transfers.len(), 0);
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_add_multiple_transfers() {
        let mut runtime = create_test_runtime();

        // Add multiple transfers
        for i in 0..5 {
            runtime.add_pending_transfer(
                vec![i],
                format!("Type{}", i),
                vec![i * 10],
                AccountAddress::from_hex_literal(&format!("0x{}", i + 1)).unwrap(),
            );
        }

        // Verify all added
        let pending = runtime.get_pending_objects();
        assert_eq!(pending.transfers.len(), 5);

        // Verify data
        for (i, transfer) in pending.transfers.iter().enumerate() {
            assert_eq!(transfer.object_id, vec![i as u8]);
            assert_eq!(transfer.object_type, format!("Type{}", i));
            assert_eq!(transfer.object_data, vec![i as u8 * 10]);
        }
    }

    #[test]
    fn test_pending_objects_merge() {
        // Create two separate PendingObjectOps
        let mut ops1 = PendingObjectOps::new();
        ops1.add_transfer(ObjectTransfer {
            object_id: vec![1],
            object_type: "Type1".to_string(),
            object_data: vec![10],
            recipient: AccountAddress::from_hex_literal("0x1").unwrap(),
        });

        let mut ops2 = PendingObjectOps::new();
        ops2.add_transfer(ObjectTransfer {
            object_id: vec![2],
            object_type: "Type2".to_string(),
            object_data: vec![20],
            recipient: AccountAddress::from_hex_literal("0x2").unwrap(),
        });

        // Merge ops2 into ops1
        let merged = ops1.merge(ops2);

        // Verify both transfers are present
        assert_eq!(merged.transfers.len(), 2);
        assert_eq!(merged.transfers[0].object_id, vec![1]);
        assert_eq!(merged.transfers[1].object_id, vec![2]);
    }

    #[test]
    fn test_pending_objects_is_empty() {
        let mut ops = PendingObjectOps::new();
        assert!(ops.is_empty());

        // Add transfer
        ops.add_transfer(ObjectTransfer {
            object_id: vec![1],
            object_type: "Test".to_string(),
            object_data: vec![],
            recipient: AccountAddress::from_hex_literal("0x1").unwrap(),
        });
        assert!(!ops.is_empty());
    }

    #[test]
    fn test_pending_objects_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let runtime = Arc::new(std::sync::Mutex::new(create_test_runtime()));

        // Spawn multiple threads that add operations
        let mut handles = vec![];
        for i in 0..3 {
            let runtime_clone = Arc::clone(&runtime);
            let handle = thread::spawn(move || {
                let mut rt = runtime_clone.lock().unwrap();
                rt.add_pending_transfer(
                    vec![i],
                    format!("Thread{}", i),
                    vec![i * 100],
                    AccountAddress::from_hex_literal(&format!("0x{}", i + 1)).unwrap(),
                );
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all operations added
        let rt = runtime.lock().unwrap();
        let pending = rt.get_pending_objects();
        assert_eq!(pending.transfers.len(), 3);
    }

    #[test]
    fn test_runtime_creation_with_empty_pending() {
        let runtime = create_test_runtime();

        // New runtime should have empty pending_objects
        let pending = runtime.get_pending_objects();
        assert!(pending.is_empty());
        assert_eq!(pending.transfers.len(), 0);
        assert_eq!(pending.freezes.len(), 0);
        assert_eq!(pending.shares.len(), 0);
    }
}
