//! Request validation via [`vld`].

mod error;
mod ext;

pub use error::ValidationError;
pub use ext::ValidationExt;

#[cfg(feature = "openapi")]
mod doc_ext;
#[cfg(feature = "openapi")]
pub use doc_ext::{DocVldExt, VldDocSchema};
