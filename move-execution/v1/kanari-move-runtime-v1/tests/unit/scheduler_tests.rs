use super::*;
use kanari_types::transaction::{ObjectInput, ObjectOwnerKind, ObjectRef, Transaction};

fn create_dummy_tx(sender: &str, module: &str, object: Option<&str>) -> SignedTransaction {
    let mut args = Vec::new();
    if let Some(obj) = object {
        let mut id = vec![0u8; 32];
        let bytes = obj.as_bytes();
        for (i, b) in bytes.iter().enumerate().take(32) {
            id[i] = *b;
        }
        args.push(id);
    }

    let tx = Transaction::ExecuteFunction {
        sender: sender.to_string(),
        module: module.to_string(),
        function: "test".to_string(),
        type_args: vec![],
        args,
        object_inputs: object
            .map(|obj| {
                vec![ObjectInput {
                    object_ref: ObjectRef::new(obj.to_string(), None, None),
                    owner: Some(ObjectOwnerKind::AddressOwner(sender.to_string())),
                    mutable: true,
                }]
            })
            .unwrap_or_default(),
        gas_payment: None,
        gas_limit: 1000,
        gas_price: 1,
        nonce: 0,
    };
    SignedTransaction::new(tx)
}

#[test]
fn test_schedule_preserves_order_inside_speculative_wave() {
    let txs = vec![
        create_dummy_tx("A", "M1", Some("Obj1")),
        create_dummy_tx("B", "M2", Some("Obj2")),
        create_dummy_tx("C", "M3", Some("Obj1")),
        create_dummy_tx("D", "M4", Some("Obj2")),
    ];

    let expected_hashes = txs
        .iter()
        .map(|tx| tx.transaction_hash().to_vec())
        .collect::<Vec<_>>();
    let waves = TransactionScheduler::schedule(txs);

    assert_eq!(waves.len(), 1);
    assert_eq!(waves[0].len(), 4);

    let actual_hashes = waves
        .iter()
        .flat_map(|wave| wave.iter().map(|tx| tx.transaction_hash().to_vec()))
        .collect::<Vec<_>>();
    assert_eq!(actual_hashes, expected_hashes);
}

#[test]
fn test_schedule_empty_batch() {
    let waves = TransactionScheduler::schedule(Vec::new());
    assert!(waves.is_empty());
}

#[test]
fn test_apparently_independent_transactions_are_speculated_together() {
    let txs = vec![
        create_dummy_tx("A", "M1", Some("Obj1")),
        create_dummy_tx("B", "M2", Some("Obj2")),
    ];

    let waves = TransactionScheduler::schedule(txs);
    assert_eq!(waves.len(), 1);
    assert_eq!(waves[0].len(), 2);
}

#[test]
fn test_schedule_bounds_large_speculative_batches() {
    let txs = (0..130)
        .map(|i| create_dummy_tx(&format!("sender-{i}"), "M", None))
        .collect();
    let waves = TransactionScheduler::schedule(txs);
    assert_eq!(
        waves.iter().map(Vec::len).collect::<Vec<_>>(),
        vec![64, 64, 2]
    );
}
