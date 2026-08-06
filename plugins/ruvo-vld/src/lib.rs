//! Request validation via [`vld`].

mod coerce;
mod error;
mod ext;
mod validate;

pub use error::ValidationError;
pub use ext::ValidationExt;
pub use validate::{
    missing_validate_routes, ValidExt, Validate, ValidateHook, ValidateRouteExt, ValidateSchema,
    ValidateSource, Validated, Vld,
};

#[cfg(feature = "openapi")]
mod doc_ext;
#[cfg(feature = "openapi")]
pub use doc_ext::{DocVldExt, VldDocSchema};

#[cfg(feature = "i18n")]
mod i18n_msg;

#[cfg(feature = "flash-templates")]
mod flash_templates;
#[cfg(feature = "flash-templates")]
pub use flash_templates::with_validation_flash;
