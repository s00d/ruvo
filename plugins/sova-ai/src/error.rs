//! Errors for the AI plugin.

use sova_core::{IntoResponse, Response};
use thiserror::Error;

/// Plugin / call errors (maps AISDK failures and missing install config).
#[derive(Debug, Error)]
pub enum AiError {
    /// No default model was configured on [`crate::Ai`] / [`crate::AiClient`].
    #[error("ai: no model configured (use Ai::model(...) or Ai::fake(...))")]
    NoModel,
    /// Missing prompt/messages on a bound call.
    #[error("ai: prompt or messages required")]
    EmptyPrompt,
    /// Upstream AISDK error.
    #[error(transparent)]
    Sdk(#[from] aisdk::Error),
}

impl IntoResponse for AiError {
    fn into_response(self) -> Response {
        let status = match &self {
            AiError::NoModel | AiError::EmptyPrompt => 500,
            AiError::Sdk(_) => 502,
        };
        Response::text(self.to_string()).status(status)
    }
}

impl From<AiError> for sova_core::Error {
    fn from(value: AiError) -> Self {
        sova_core::Error::Internal(value.to_string())
    }
}
