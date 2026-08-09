//! `req.ai()` fluent generate / stream helpers.

use crate::client::AiClient;
use crate::error::AiError;
use crate::stream::stream_to_response;
use aisdk::core::{GenerateTextResponse, LanguageModelRequest, Messages};
use sova_core::{Request, Response};
use std::sync::Arc;

/// `req.ai()` — bound to the installed [`AiClient`].
pub trait AiExt {
    fn ai(&self) -> AiBound;
}

impl AiExt for Request {
    fn ai(&self) -> AiBound {
        let client = self.state::<AiClient>();
        AiBound::new(client)
    }
}

/// Fluent call builder (`prompt` / `system` / `generate` / `stream_response`).
pub struct AiBound {
    client: Arc<AiClient>,
    system: Option<String>,
    prompt: Option<String>,
    messages: Option<Messages>,
}

impl AiBound {
    pub(crate) fn new(client: Arc<AiClient>) -> Self {
        let system = client.default_system().map(str::to_owned);
        Self {
            client,
            system,
            prompt: None,
            messages: None,
        }
    }

    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    pub fn messages(mut self, messages: Messages) -> Self {
        self.messages = Some(messages);
        self
    }

    /// Full AISDK builder (model already set).
    pub fn builder(
        self,
    ) -> aisdk::core::language_model::request::LanguageModelRequestBuilder<
        crate::SharedModel,
        aisdk::core::language_model::request::SystemStage,
    > {
        self.client.builder()
    }

    fn build_request(self) -> Result<LanguageModelRequest<crate::SharedModel>, AiError> {
        let AiBound {
            client,
            system,
            prompt,
            messages,
        } = self;

        let b = LanguageModelRequest::builder().model(client.model());

        Ok(match (system, prompt, messages) {
            (Some(sys), Some(p), None) => b.system(sys).prompt(p).build(),
            (Some(sys), None, Some(m)) => b.system(sys).messages(m).build(),
            (None, Some(p), None) => b.prompt(p).build(),
            (None, None, Some(m)) => b.messages(m).build(),
            _ => return Err(AiError::EmptyPrompt),
        })
    }

    /// Non-streaming generation.
    pub async fn generate(self) -> Result<GenerateTextResponse, AiError> {
        let mut req = self.build_request()?;
        Ok(req.generate_text().await?)
    }

    /// Convenience: generate and return assistant text (empty string if none).
    pub async fn text(self) -> Result<String, AiError> {
        let out = self.generate().await?;
        Ok(out.text().unwrap_or_default())
    }

    /// Stream model output as `text/event-stream` [`Response`].
    pub async fn stream_response(self) -> Result<Response, AiError> {
        let mut req = self.build_request()?;
        let stream = req.stream_text().await?;
        Ok(stream_to_response(stream))
    }
}
