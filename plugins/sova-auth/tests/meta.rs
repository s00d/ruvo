//! Fortify form / token unit tests.

#[cfg(feature = "vld")]
mod vld_forms {
    use sova_auth::{LoginForm, RegisterForm};
    use vld::schema::VldParse;

    #[test]
    fn register_rejects_short_password() {
        let v = serde_json::json!({
            "name": "Ada",
            "email": "ada@example.com",
            "password": "short"
        });
        assert!(RegisterForm::vld_parse_value(&v).is_err());
    }

    #[test]
    fn register_ok() {
        let v = serde_json::json!({
            "name": "Ada",
            "email": "ada@example.com",
            "password": "secret123"
        });
        let f = RegisterForm::vld_parse_value(&v).unwrap();
        assert_eq!(f.email, "ada@example.com");
    }

    #[test]
    fn login_rejects_bad_email() {
        let v = serde_json::json!({
            "email": "not-an-email",
            "password": "x"
        });
        assert!(LoginForm::vld_parse_value(&v).is_err());
    }
}

#[test]
fn verify_token_roundtrip() {
    use sova_auth::{make_verify_token, parse_verify_token};
    let secret = "test-secret";
    let tok = make_verify_token(secret, 42);
    assert_eq!(parse_verify_token(secret, &tok).unwrap(), 42);
}
