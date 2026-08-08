//! Object storage via `Storage::from_env` (memory fallback / S3·MinIO).
//!
//! ```bash
//! # memory (default when SOVA_STORAGE unset → local; here we prefer memory for demo)
//! SOVA_STORAGE=memory cargo run -p storage_demo
//!
//! # MinIO — see README.md
//! SOVA_STORAGE=s3 \
//!   SOVA_STORAGE_BUCKET=sova \
//!   SOVA_STORAGE_ENDPOINT=http://127.0.0.1:9000 \
//!   AWS_ACCESS_KEY_ID=minioadmin \
//!   AWS_SECRET_ACCESS_KEY=minioadmin \
//!   cargo run -p storage_demo
//! ```

use bytes::Bytes;
use sova::prelude::*;
use sova::{PutOpts, Storage, StorageExt};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct PutBody {
    key: String,
    #[serde(default)]
    data: String,
    #[serde(default)]
    content_type: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = sova::sova_env::load();

    // Prefer explicit memory when unset so the demo runs without a disk tree.
    if std::env::var("SOVA_STORAGE").is_err() {
        std::env::set_var("SOVA_STORAGE", "memory");
    }

    let mut app = App::new();
    app.install(Storage::from_env().map_err(|e| Error::Internal(e.to_string()))?);

    app.get("/", || async {
        Json(json!({
            "ok": true,
            "hint": "POST /put · GET /get?key= · GET /exists?key= · GET /list?prefix= · GET /temporary-url?key= (cloud) · DELETE /delete?key="
        }))
    });

    app.post("/put", |mut req: Request| async move {
        let body: PutBody = req.json().await?;
        let storage = req.storage().clone();
        let opts = PutOpts {
            content_type: body.content_type,
            ..Default::default()
        };
        storage
            .put(&body.key, Bytes::from(body.data.into_bytes()), opts)
            .await?;
        let url = storage.url(&body.key);
        Ok::<_, Error>(Json(json!({ "ok": true, "key": body.key, "url": url })))
    });

    app.get("/get", |req: Request| async move {
        let key = req
            .query("key")
            .ok_or_else(|| Error::BadRequest("query key required".into()))?;
        let storage = req.storage();
        match storage.get(key).await? {
            Some(bytes) => {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                Ok::<_, Error>(Json(json!({ "key": key, "data": text })))
            }
            None => Err(Error::NotFound),
        }
    });

    app.get("/exists", |req: Request| async move {
        let key = req
            .query("key")
            .ok_or_else(|| Error::BadRequest("query key required".into()))?;
        let ok = req.storage().exists(key).await?;
        Ok::<_, Error>(Json(json!({ "key": key, "exists": ok })))
    });

    app.get("/list", |req: Request| async move {
        let prefix = req.query("prefix").unwrap_or("");
        let keys = req.storage().list(prefix).await?;
        Ok::<_, Error>(Json(json!({ "prefix": prefix, "keys": keys })))
    });

    app.get("/temporary-url", |req: Request| async move {
        let key = req
            .query("key")
            .ok_or_else(|| Error::BadRequest("query key required".into()))?;
        let secs: u64 = req
            .query("expires")
            .and_then(|s| s.parse().ok())
            .unwrap_or(300);
        let url = req
            .storage()
            .temporary_url(key, std::time::Duration::from_secs(secs))
            .await
            .map_err(|e| Error::BadRequest(e.to_string()))?;
        Ok::<_, Error>(Json(json!({ "key": key, "url": url, "expires": secs })))
    });

    app.delete("/delete", |req: Request| async move {
        let key = req
            .query("key")
            .ok_or_else(|| Error::BadRequest("query key required".into()))?;
        req.storage().delete(key).await?;
        Ok::<_, Error>(Json(json!({ "ok": true, "key": key })))
    });

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3030);
    app.listen(port).await
}
