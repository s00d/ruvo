//! Flash `errors` / `old` providers for MiniJinja (feature `flash-templates`).

use minijinja::Value;
use ruvo_core::Request;
use ruvo_session::SessionExt;
use ruvo_templates::MiniJinjaTemplatesBuilder;

/// Attach `errors` and `old` per-request template variables from session flash.
pub fn with_validation_flash(builder: MiniJinjaTemplatesBuilder) -> MiniJinjaTemplatesBuilder {
    builder
        .per_request("errors", |req: &Request| take_json(req, "flash_errors"))
        .per_request("old", |req: &Request| take_json(req, "flash_old"))
}

fn take_json(req: &Request, key: &str) -> Value {
    let session = req.session();
    let raw = session.get(key).unwrap_or_default();
    if !raw.is_empty() {
        session.set(key, "");
    }
    let parsed: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}));
    Value::from_serialize(&parsed)
}
