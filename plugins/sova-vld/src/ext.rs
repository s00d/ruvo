use crate::validate::{read_params_value, read_query_value};
use crate::ValidationError;
use sova_core::Request;
use serde_json::Value;
use vld::schema::VldParse;

/// Validate request body / query / path params with `vld` `schema!` types.
pub trait ValidationExt {
    fn validate<T: VldParse>(
        &mut self,
    ) -> impl std::future::Future<Output = Result<T, ValidationError>> + Send;

    fn validate_query<T: VldParse>(&self) -> Result<T, ValidationError>;

    fn validate_params<T: VldParse>(&self) -> Result<T, ValidationError>;

    /// Parse `application/x-www-form-urlencoded` or `multipart/form-data` (feature `form`).
    #[cfg(feature = "form")]
    fn validate_form<T: VldParse>(
        &mut self,
    ) -> impl std::future::Future<Output = Result<T, ValidationError>> + Send;
}

impl ValidationExt for Request {
    async fn validate<T: VldParse>(&mut self) -> Result<T, ValidationError> {
        let bytes = self.body().await?;
        let value: Value = serde_json::from_slice(&bytes).map_err(|e| {
            ValidationError(vld::error::VldError::single(
                vld::error::IssueCode::ParseError,
                format!("Invalid JSON: {e}"),
            ))
        })?;
        T::vld_parse_value(&value).map_err(ValidationError::from)
    }

    fn validate_query<T: VldParse>(&self) -> Result<T, ValidationError> {
        let value = read_query_value(self);
        T::vld_parse_value(&value).map_err(ValidationError::from)
    }

    fn validate_params<T: VldParse>(&self) -> Result<T, ValidationError> {
        let value = read_params_value(self);
        T::vld_parse_value(&value).map_err(ValidationError::from)
    }

    #[cfg(feature = "form")]
    async fn validate_form<T: VldParse>(&mut self) -> Result<T, ValidationError> {
        let value = crate::validate::read_form_value(self).await?;
        T::vld_parse_value(&value).map_err(ValidationError::from)
    }
}
