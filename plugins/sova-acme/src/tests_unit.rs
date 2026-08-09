//! Unit tests (storage / challenges / reload).

use crate::http01::ChallengeMap;
use crate::storage::{needs_renew, AcmeStorage, CertMeta};
use sova_core::Tls;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn challenge_map_roundtrip() {
    let m = ChallengeMap::new();
    m.insert("tok", "auth.val");
    assert_eq!(m.get("tok").as_deref(), Some("auth.val"));
    m.remove("tok");
    assert!(m.get("tok").is_none());
}

#[test]
fn storage_meta_and_pem() {
    let dir = tempfile::tempdir().unwrap();
    let storage = AcmeStorage::new(dir.path());
    storage.ensure_dir().unwrap();
    assert!(!storage.has_cert());

    let cert = rcgen::generate_simple_self_signed(vec!["acme.test".into()]).unwrap();
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();
    storage.write_pem(&cert_pem, &key_pem).unwrap();
    assert!(storage.has_cert());

    let meta = CertMeta {
        domains: vec!["acme.test".into()],
        not_after_unix: 9_999_999_999,
        staging: true,
    };
    storage.save_meta(&meta).unwrap();
    let loaded = storage.load_meta().unwrap();
    assert_eq!(loaded.domains, meta.domains);
    assert!(loaded.staging);
}

#[test]
fn needs_renew_threshold() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let soon = CertMeta {
        domains: vec!["x".into()],
        not_after_unix: now + 2 * 24 * 3600,
        staging: true,
    };
    assert!(needs_renew(&soon, 30));
    let far = CertMeta {
        domains: vec!["x".into()],
        not_after_unix: now + 90 * 24 * 3600,
        staging: true,
    };
    assert!(!needs_renew(&far, 30));
}

#[test]
fn tls_reload_pem_from_acme_paths() {
    let dir = tempfile::tempdir().unwrap();
    let storage = AcmeStorage::new(dir.path());
    storage.ensure_dir().unwrap();
    let a = rcgen::generate_simple_self_signed(vec!["a.test".into()]).unwrap();
    storage
        .write_pem(&a.cert.pem(), &a.key_pair.serialize_pem())
        .unwrap();
    let tls = Tls::from_pem(storage.cert_path(), storage.key_path()).unwrap();

    let b = rcgen::generate_simple_self_signed(vec!["b.test".into()]).unwrap();
    let cert_pem = b.cert.pem();
    let key_pem = b.key_pair.serialize_pem();
    tls.reload_pem(&cert_pem, &key_pem).unwrap();
    assert_eq!(
        std::fs::read_to_string(storage.cert_path()).unwrap(),
        cert_pem
    );
}
