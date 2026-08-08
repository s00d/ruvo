//! Flash `errors` / `old` / `status` providers for MiniJinja (feature `flash-templates`).

use minijinja::Value;
use ruvo_core::Request;
use ruvo_session::{SessionExt, FLASH_ERRORS, FLASH_OLD, FLASH_STATUS};
use ruvo_templates::MiniJinjaTemplatesBuilder;

/// Attach `errors`, `old`, and `status` per-request template variables from session flash.
pub fn with_flash(builder: MiniJinjaTemplatesBuilder) -> MiniJinjaTemplatesBuilder {
    builder
        .per_request("errors", |req: &Request| take_json(req, FLASH_ERRORS))
        .per_request("old", |req: &Request| take_json(req, FLASH_OLD))
        .per_request("status", |req: &Request| take_status(req))
}

/// Alias of [`with_flash`] (historical name).
pub fn with_validation_flash(builder: MiniJinjaTemplatesBuilder) -> MiniJinjaTemplatesBuilder {
    with_flash(builder)
}

fn take_json(req: &Request, key: &str) -> Value {
    let parsed = req.session().take_json(key);
    Value::from_serialize(&parsed)
}

fn take_status(req: &Request) -> Value {
    let raw = req.session().take(FLASH_STATUS);
    if raw.is_empty() {
        Value::from(())
    } else {
        Value::from(raw)
    }
}
