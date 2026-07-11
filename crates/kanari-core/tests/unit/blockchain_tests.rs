use super::*;
use kanari_types::error::KanariUnwrapExt;
use kanari_types::transaction::{ObjectRef, Transaction};

fn test_tx(nonce: u64) -> SignedTransaction {
    SignedTransaction::new(Transaction::new_transfer_with_object_ref(
        "0x1".to_string(),
        ObjectRef::new("0xaaaa", Some(1), Some("0xtestdigest".to_string())),
        "0x2".to_string(),
        1,
        nonce,
    ))
}

#[test]
fn transaction_count_uses_queryable_index_not_stale_snapshot_counter() {
    let mut chain = Blockchain::new();
    let checkpoint = Checkpoint::new(
        1,
        vec![[1u8; 32]],
        vec![test_tx(0), test_tx(1)],
        vec![2u8; 32],
        1,
        chain
            .latest_checkpoint()
            .hash()
            .invariant("checkpoint hash"),
    );
    chain
        .add_checkpoint_with_validation(checkpoint, false)
        .invariant("test operation");
    chain.total_transaction_count = 99;

    assert_eq!(chain.get_transaction_count(), 2);
}

#[test]
fn checkpoint_validation_rejects_non_consecutive_sequence() {
    let mut chain = Blockchain::new();
    let checkpoint = Checkpoint::new(
        2,
        vec![[1u8; 32]],
        vec![test_tx(0)],
        vec![2u8; 32],
        1,
        chain
            .latest_checkpoint()
            .hash()
            .invariant("checkpoint hash"),
    );

    let err = chain
        .add_checkpoint_with_validation(checkpoint, true)
        .unwrap_err();

    assert!(
        err.to_string().contains("Invalid checkpoint sequence"),
        "unexpected error: {err}"
    );
}

#[test]
fn checkpoint_validation_rejects_wrong_previous_hash() {
    let mut chain = Blockchain::new();
    let checkpoint = Checkpoint::new(
        1,
        vec![[1u8; 32]],
        vec![test_tx(0)],
        vec![2u8; 32],
        1,
        vec![9u8; 32],
    );

    let err = chain
        .add_checkpoint_with_validation(checkpoint, true)
        .unwrap_err();

    assert!(
        err.to_string().contains("Invalid previous checkpoint hash"),
        "unexpected error: {err}"
    );
}

#[test]
fn checkpoint_validation_rejects_duplicate_transactions_within_checkpoint() {
    let mut chain = Blockchain::new();
    let tx = test_tx(0);
    let checkpoint = Checkpoint::new(
        1,
        vec![[1u8; 32]],
        vec![tx.clone(), tx],
        vec![2u8; 32],
        1,
        chain
            .latest_checkpoint()
            .hash()
            .invariant("checkpoint hash"),
    );

    let err = chain
        .add_checkpoint_with_validation(checkpoint, true)
        .unwrap_err();

    assert!(
        err.to_string()
            .contains("Duplicate transaction found within checkpoint"),
        "unexpected error: {err}"
    );
}

#[test]
fn checkpoint_validation_rejects_replayed_transaction_from_prior_checkpoint() {
    let mut chain = Blockchain::new();
    let tx = test_tx(0);
    let checkpoint = Checkpoint::new(
        1,
        vec![[1u8; 32]],
        vec![tx.clone()],
        vec![2u8; 32],
        1,
        chain
            .latest_checkpoint()
            .hash()
            .invariant("checkpoint hash"),
    );
    chain
        .add_checkpoint_with_validation(checkpoint, true)
        .invariant("test checkpoint must commit");

    let replay_checkpoint = Checkpoint::new(
        2,
        vec![[3u8; 32]],
        vec![tx],
        vec![4u8; 32],
        2,
        chain
            .latest_checkpoint()
            .hash()
            .invariant("checkpoint hash"),
    );

    let err = chain
        .add_checkpoint_with_validation(replay_checkpoint, true)
        .unwrap_err();

    assert!(
        err.to_string()
            .contains("Replay attack detected: Transaction already executed"),
        "unexpected error: {err}"
    );
}
