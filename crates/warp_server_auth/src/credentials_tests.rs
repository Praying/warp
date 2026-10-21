use super::{AuthToken, Credentials};

#[test]
fn local_credentials_send_unauthenticated_requests() {
    let credentials = Credentials::Local;
    assert!(matches!(credentials.bearer_token(), AuthToken::NoAuth));
    assert!(credentials.login_token().is_none());
    assert!(!credentials.is_externally_managed());
    assert!(credentials.as_firebase().is_none());
    assert!(credentials.as_api_key().is_none());
    assert!(credentials.refresh_token().is_none());
}
