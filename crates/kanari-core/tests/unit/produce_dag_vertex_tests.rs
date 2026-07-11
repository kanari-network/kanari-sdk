use super::*;
use kanari_crypto::keys::{CurveType, generate_keypair};
use kanari_types::transaction::{ObjectRef, SignedTransaction, Transaction};

fn authority_key(seed: u8) -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[seed; 32])
}

fn signed_transfer(nonce: u64) -> SignedTransaction {
    let sender = generate_keypair(CurveType::Ed25519).unwrap();
    let recipient = generate_keypair(CurveType::Ed25519).unwrap();
    let tx = Transaction::new_transfer_with_object_ref(
        sender.tagged_address(),
        ObjectRef::new("0xaaaa", Some(1), Some("0xtestdigest".to_string())),
        recipient.address,
        1,
        nonce,
    );
    let mut signed_tx = SignedTransaction::new(tx);
    signed_tx
        .sign(&sender.private_key, sender.curve_type)
        .unwrap();
    signed_tx
}

fn signed_network_vertex(
    author: &str,
    signing_key: &ed25519_dalek::SigningKey,
    round: u64,
    parents: Vec<(String, u64, [u8; 32])>,
) -> DagVertex {
    let tx = signed_transfer(0);
    let mut vertex = DagVertex::new(
        round,
        author.to_string(),
        "kanari-v2-mysticeti".to_string(),
        parents,
        vec![tx],
        123,
    );
    use ed25519_dalek::Signer;
    vertex.signature = signing_key.sign(&vertex.id).to_bytes().to_vec();
    vertex
}

#[test]
fn test_dag_engine_defaults_to_mysticeti_protocol() {
    let engine = Arc::new(BlockchainEngine::new_in_memory().unwrap());
    let signing_key = authority_key(11);
    let mut public_keys = BTreeMap::new();
    public_keys.insert(
        "auth1".to_string(),
        signing_key.verifying_key().to_bytes().to_vec(),
    );
    let dag_engine = DagEngine::new_secure(
        engine,
        "auth1".to_string(),
        vec![
            "auth1".to_string(),
            "auth2".to_string(),
            "auth3".to_string(),
            "auth4".to_string(),
        ],
        signing_key,
        public_keys,
    )
    .unwrap();

    let state = lock_read(&dag_engine.state);
    assert_eq!(state.mysticeti.protocol.wave_length, 3);
    assert_eq!(state.mysticeti.protocol.direct_commit_quorum, 3);
    assert!(state.mysticeti.protocol.pipeline);
    assert!(state.mysticeti.protocol.leader_wait);
}

#[test]
fn test_dag_engine_secure_constructor_rejects_mismatched_local_key() {
    let engine = Arc::new(BlockchainEngine::new_in_memory().unwrap());
    let expected = authority_key(11);
    let wrong = authority_key(33);
    let mut public_keys = BTreeMap::new();
    public_keys.insert(
        "auth1".to_string(),
        expected.verifying_key().to_bytes().to_vec(),
    );

    let result = DagEngine::new_secure(
        engine,
        "auth1".to_string(),
        vec!["auth1".to_string()],
        wrong,
        public_keys,
    );

    assert!(result.is_err());
}

fn build_test_dag_engine(
    authorities: Vec<String>,
    local_authority: &str,
) -> (Arc<BlockchainEngine>, DagEngine, ed25519_dalek::SigningKey) {
    let engine = Arc::new(BlockchainEngine::new_in_memory().unwrap());
    let local_key = authority_key(11);
    let remote_key = authority_key(22);
    let mut public_keys = BTreeMap::new();
    for auth in &authorities {
        let key = if auth == local_authority {
            &local_key
        } else {
            &remote_key
        };
        public_keys.insert(auth.clone(), key.verifying_key().to_bytes().to_vec());
    }
    let dag_engine = DagEngine::new_secure(
        engine.clone(),
        local_authority.to_string(),
        authorities,
        local_key,
        public_keys,
    )
    .unwrap();
    (engine, dag_engine, remote_key)
}

#[test]
fn test_add_network_vertex_accepts_valid_remote_vertex() {
    let (_engine, dag_engine, remote_key) =
        build_test_dag_engine(vec!["auth1".to_string(), "auth2".to_string()], "auth1");

    let vertex = signed_network_vertex("auth2", &remote_key, 1, vec![]);
    dag_engine.add_network_vertex(vertex).unwrap();

    let state = lock_read(&dag_engine.state);
    assert_eq!(state.vertices.len(), 1);
    assert_eq!(state.vertices[0].author, "auth2");
}

#[test]
fn test_add_network_vertex_rejects_invalid_signature() {
    let (_engine, dag_engine, _remote_key) =
        build_test_dag_engine(vec!["auth1".to_string(), "auth2".to_string()], "auth1");
    let wrong_key = authority_key(33);

    let vertex = signed_network_vertex("auth2", &wrong_key, 1, vec![]);
    let error = dag_engine.add_network_vertex(vertex).unwrap_err();
    assert!(error.to_string().contains("Invalid DAG vertex signature"));
}

#[test]
fn test_add_network_vertex_rejects_missing_parent() {
    let (_engine, dag_engine, remote_key) =
        build_test_dag_engine(vec!["auth1".to_string(), "auth2".to_string()], "auth1");

    let mut vertex = signed_network_vertex("auth2", &remote_key, 2, vec![]);
    vertex.parents = vec![
        ("auth1".to_string(), 1u64, [1u8; 32]),
        ("auth2".to_string(), 1u64, [2u8; 32]),
    ];
    let error = dag_engine.add_network_vertex(vertex).unwrap_err();
    assert!(
        error.to_string().contains("Missing parent")
            || error.to_string().contains("missing parents")
    );
}
