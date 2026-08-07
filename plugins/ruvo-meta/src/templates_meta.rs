//! Template `meta()` helper (feature `templates`).

use crate::html::render_html;
use crate::resolve_meta;
use minijinja::Value;
use ruvo_core::Request;
use ruvo_templates::MiniJinjaTemplatesBuilder;

/// Attach per-request `meta` function that renders the HTML head fragment.
///
/// Prefer installing [`crate::Meta`] with feature `templates` — `meta` is
/// registered automatically. This wrapper remains for explicit builder chaining.
pub fn with_meta(builder: MiniJinjaTemplatesBuilder) -> MiniJinjaTemplatesBuilder {
    builder.per_request("meta", |req: &Request| {
        let html = render_html(&resolve_meta(req));
        Value::from_function(move || html.clone())
    })
}
