use crate::ValidationError;
use ruvo_core::Request;
use serde_json::{Map, Value};
use vld::schema::VldParse;

/// Validate request body / query / path params with `vld` `schema!` types.
pub trait ValidationExt {
    fn validate<T: VldParse>(
        &mut self,
    ) -> impl std::future::Future<Output = Result<T, ValidationError>> + Send;

    fn validate_query<T: VldParse>(&self) -> Result<T, ValidationError>;

    fn validate_params<T: VldParse>(&self) -> Result<T, ValidationError>;
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
        let mut map = Map::new();
        for (k, v) in &self.query {
            map.insert(k.clone(), Value::String(v.clone()));
        }
        T::vld_parse_value(&Value::Object(map)).map_err(ValidationError::from)
    }

    fn validate_params<T: VldParse>(&self) -> Result<T, ValidationError> {
        let mut map = Map::new();
        for (k, v) in &self.params {
            map.insert(k.clone(), Value::String(v.clone()));
        }
        T::vld_parse_value(&Value::Object(map)).map_err(ValidationError::from)
    }
}
