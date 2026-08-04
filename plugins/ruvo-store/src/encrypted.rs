//! AEAD-at-rest wrapper for [`KvStore`](crate::KvStore).

use crate::{BoxFuture, KvStore};
use bytes::Bytes;
use chacha20poly1305::aead::Aead;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::{Sha256, Sha512};
use std::time::Duration;

type HmacSha256 = Hmac<Sha256>;

const ENVELOPE_V1: &str = "v1";
const HKDF_SALT: &[u8] = b"ruvo-store-v1";

/// Application secret used to derive per-namespace keys.
#[derive(Clone)]
pub struct AppKey {
    bytes: Vec<u8>,
}

impl AppKey {
    pub fn from_env(name: &'static str) -> Result<Self, String> {
        let v = std::env::var(name).map_err(|_| format!("missing env `{name}`"))?;
        if v.is_empty() {
            return Err(format!("env `{name}` is empty"));
        }
        Ok(Self { bytes: v.into_bytes() })
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    fn derive_namespace_key(&self, namespace: &str) -> [u8; 32] {
        let hk = Hkdf::<Sha512>::new(Some(HKDF_SALT), &self.bytes);
        let mut okm = [0u8; 32];
        hk.expand(namespace.as_bytes(), &mut okm)
            .expect("hkdf expand");
        okm
    }
}

fn derive_mac_key(namespace: &str) -> [u8; 32] {
    let hk = Hkdf::<Sha512>::new(Some(b"ruvo-store-mac-v1"), namespace.as_bytes());
    let mut okm = [0u8; 32];
    hk.expand(b"storage-key", &mut okm).expect("hkdf expand");
    okm
}

const INCR_PREFIX: &str = "__incr:";

/// Encrypted [`KvStore`] decorator (values only; `incr` bypasses encryption).
pub struct Encrypted<S> {
    inner: S,
    previous: Vec<AppKey>,
    namespace: String,
    ns_key: [u8; 32],
    mac_key: [u8; 32],
}

impl<S> Encrypted<S> {
    pub fn new(inner: S, key: AppKey, namespace: impl Into<String>) -> Self {
        let namespace = namespace.into();
        let ns_key = key.derive_namespace_key(&namespace);
        let mac_key = derive_mac_key(&namespace);
        Self {
            inner,
            previous: Vec::new(),
            namespace,
            ns_key,
            mac_key,
        }
    }

    pub fn with_previous_keys(mut self, keys: Vec<AppKey>) -> Self {
        self.previous = keys;
        self
    }

    fn storage_key(&self, logical: &str) -> String {
        if logical.contains(':') {
            let (prefix, tail) = logical.split_at(logical.rfind(':').unwrap() + 1);
            format!("{prefix}{}", hex_hmac(&self.mac_key, tail))
        } else {
            hex_hmac(&self.mac_key, logical)
        }
    }

    fn encrypt(&self, plain: &[u8]) -> Bytes {
        let cipher = XChaCha20Poly1305::new_from_slice(&self.ns_key).expect("cipher key");
        let mut nonce = [0u8; 24];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);
        let ct = cipher
            .encrypt(XNonce::from_slice(&nonce), plain)
            .expect("encrypt");
        let mut out = format!("{ENVELOPE_V1}:").into_bytes();
        out.extend_from_slice(&nonce);
        out.push(b':');
        out.extend_from_slice(&ct);
        Bytes::from(out)
    }

    fn decrypt(&self, blob: &[u8]) -> Option<Bytes> {
        self.try_decrypt_with(&self.ns_key, blob)
            .or_else(|| {
                for old in &self.previous {
                    let k = old.derive_namespace_key(&self.namespace);
                    if let Some(b) = self.try_decrypt_with(&k, blob) {
                        return Some(b);
                    }
                }
                None
            })
    }

    fn try_decrypt_with(&self, key: &[u8; 32], blob: &[u8]) -> Option<Bytes> {
        let rest = blob.strip_prefix(format!("{ENVELOPE_V1}:").as_bytes())?;
        if rest.len() < 25 {
            return None;
        }
        let (nonce, ct) = rest.split_at(24);
        let ct = ct.strip_prefix(b":")?;
        let cipher = XChaCha20Poly1305::new_from_slice(key).expect("cipher key");
        cipher
            .decrypt(XNonce::from_slice(nonce), ct)
            .ok()
            .map(Bytes::from)
    }
}

fn hex_hmac(key: &[u8; 32], msg: &str) -> String {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key).expect("hmac key");
    mac.update(msg.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

pub fn encrypted<S>(inner: S, key: AppKey) -> Encrypted<S> {
    Encrypted::new(inner, key, "default")
}

pub fn encrypted_ns<S>(inner: S, key: AppKey, namespace: impl Into<String>) -> Encrypted<S> {
    Encrypted::new(inner, key, namespace)
}

impl<S: KvStore> KvStore for Encrypted<S> {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Option<Bytes>> {
        let incr_key = format!("{INCR_PREFIX}{key}");
        let sk = self.storage_key(key);
        Box::pin(async move {
            if let Some(raw) = self.inner.get(&incr_key).await {
                return Some(raw);
            }
            let blob = self.inner.get(&sk).await?;
            self.decrypt(blob.as_ref())
        })
    }

    fn set<'a>(&'a self, key: &'a str, val: Bytes, ttl: Option<Duration>) -> BoxFuture<'a, ()> {
        let sk = self.storage_key(key);
        let enc = self.encrypt(val.as_ref());
        Box::pin(async move { self.inner.set(&sk, enc, ttl).await })
    }

    fn remove<'a>(&'a self, key: &'a str) -> BoxFuture<'a, ()> {
        let sk = self.storage_key(key);
        Box::pin(async move { self.inner.remove(&sk).await })
    }

    /// Counter values are not secret — stored plaintext (documented bypass).
    fn incr<'a>(&'a self, key: &'a str, by: i64, ttl: Option<Duration>) -> BoxFuture<'a, u64> {
        let ik = format!("{INCR_PREFIX}{key}");
        Box::pin(async move { self.inner.incr(&ik, by, ttl).await })
    }

    fn clear_prefix<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, u64> {
        Box::pin(async move { self.inner.clear_prefix(prefix).await })
    }
}

#[cfg(all(test, feature = "store-crypto"))]
mod tests {
    use super::*;
    use crate::MemoryStore;
    use std::sync::Arc;

    #[tokio::test]
    async fn encrypted_roundtrip_and_plaintext_not_leaked() {
        let inner = MemoryStore::new();
        let sk;
        {
            let store = Encrypted::new(inner.clone(), AppKey::from_bytes(b"test-app-key-material!!".to_vec()), "sess");
            store
                .set("user:1", Bytes::from_static(b"secret"), None)
                .await;
            let got = store.get("user:1").await.unwrap();
            assert_eq!(got.as_ref(), b"secret");
            sk = store.storage_key("user:1");
            store.remove("user:1").await;
            assert!(store.get("user:1").await.is_none());
        }
        let raw = inner.get(&sk).await;
        assert!(raw.is_none() || !raw.unwrap().windows(6).any(|w| w == b"secret"));
    }

    #[tokio::test]
    async fn encrypted_conformance() {
        let enc = Arc::new(Encrypted::new(
            MemoryStore::new(),
            AppKey::from_bytes(b"conformance-key!!".to_vec()),
            "default",
        )) as Arc<dyn KvStore>;
        crate::conformance::run(enc).await;
    }

    #[tokio::test]
    async fn rotation_decrypts_old_records() {
        let inner = MemoryStore::new();
        let old_key = AppKey::from_bytes(b"old-key-material-32bytes!!".to_vec());
        let new_key = AppKey::from_bytes(b"new-key-material-32bytes!!".to_vec());
        {
            let store = Encrypted::new(inner.clone(), old_key.clone(), "data");
            store.set("k", Bytes::from_static(b"v"), None).await;
        }
        let rotated = Encrypted::new(inner, new_key, "data").with_previous_keys(vec![old_key]);
        assert_eq!(rotated.get("k").await.unwrap().as_ref(), b"v");
    }

    #[tokio::test]
    async fn incr_bypasses_encryption() {
        let inner = MemoryStore::new();
        let store = Encrypted::new(inner.clone(), AppKey::from_bytes(b"incr-key!!".to_vec()), "c");
        assert_eq!(store.incr("hits", 1, None).await, 1);
        assert_eq!(store.incr("hits", 2, None).await, 3);
        assert_eq!(store.get("hits").await.unwrap().as_ref(), b"3");
    }
}
