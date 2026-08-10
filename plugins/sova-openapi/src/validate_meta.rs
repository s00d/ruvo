//! Request-side OpenAPI schemas attached via `sova_vld` validate sugar.

use serde_json::Value;
use sova_core::extend::RouteValue;

/// Neutral request schemas for OpenAPI (filled by `ValidateRouteExt`).
#[derive(Debug, Clone, Default)]
pub struct OpenApiValidate {
    pub body: Option<Value>,
    pub query: Option<Value>,
    pub params: Option<Value>,
}

impl RouteValue for OpenApiValidate {
    fn label(&self) -> std::borrow::Cow<'static, str> {
        let mut parts = Vec::new();
        if self.body.is_some() {
            parts.push("body");
        }
        if self.query.is_some() {
            parts.push("query");
        }
        if self.params.is_some() {
            parts.push("params");
        }
        if parts.is_empty() {
            std::borrow::Cow::Borrowed("OpenApiValidate")
        } else {
            std::borrow::Cow::Owned(format!("OpenApiValidate({})", parts.join(",")))
        }
    }
}
