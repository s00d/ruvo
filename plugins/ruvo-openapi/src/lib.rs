//! OpenAPI 3.1 document building and Scalar docs UI.
//!
//! Does **not** depend on `vld` — schemas are plain [`serde_json::Value`].

mod build;
mod doc;
mod plugin;
mod undocumented;

pub use build::{build_document, validation_error_schema, BuildOptions};
pub use doc::Doc;
pub use plugin::{OpenApi, OpenApiDocExt};
pub use undocumented::undocumented;
