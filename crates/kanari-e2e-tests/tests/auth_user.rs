//! Authentication system: register / login / session / invalid credentials

use kanari_auth::AuthManager;

const EMAIL: &str = "e2e@example.com";
const PASSWORD: &str = "SecurePassword123!";

#[test]
fn register_login_logout_roundtrip() {
    let mut auth = AuthManager::new();
    let wallet = auth
        .register_user(EMAIL, PASSWORD, None)
        .expect("registration should succeed");
    assert!(!wallet.address.to_hex_literal().is_empty());

    let session = auth
        .login(EMAIL, PASSWORD, None)
        .expect("login should succeed");
    let validated = auth
        .validate_session(&session.session_id)
        .expect("session should be valid");
    assert_eq!(validated.session_id, session.session_id);

    auth.logout(&session.session_id)
        .expect("logout should succeed");
    assert!(
        auth.validate_session(&session.session_id).is_err(),
        "logged out session should be invalid"
    );
}

#[test]
fn duplicate_registration_is_rejected() {
    let mut auth = AuthManager::new();
    auth.register_user("dup@example.com", PASSWORD, None)
        .unwrap();
    let err = auth.register_user("dup@example.com", PASSWORD, None);
    assert!(err.is_err(), "duplicate registration should be rejected");
}

#[test]
fn wrong_password_is_rejected() {
    let mut auth = AuthManager::new();
    auth.register_user("wrong@example.com", PASSWORD, None)
        .unwrap();
    let err = auth.login("wrong@example.com", "NotThePassword1!", None);
    assert!(err.is_err(), "wrong password should be rejected");
}

#[test]
fn unknown_user_login_is_rejected() {
    let mut auth = AuthManager::new();
    let err = auth.login("nobody@example.com", PASSWORD, None);
    assert!(err.is_err(), "unknown user login should be rejected");
}

#[test]
fn persistent_auth_manager_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("auth.sqlite");
    let mut auth = AuthManager::with_persistence(db_path.clone()).unwrap();
    auth.register_user("persist@example.com", PASSWORD, None)
        .unwrap();
    drop(auth);

    let mut reopened = AuthManager::with_persistence(db_path).unwrap();
    reopened
        .login("persist@example.com", PASSWORD, None)
        .expect("login should work after reopening database");
}
