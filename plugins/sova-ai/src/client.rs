//! [`Ai`] builder and [`AiClient`] app state.

use crate::error::AiError;
use crate::fake::FakeAi;
use crate::model::SharedModel;
use aisdk::core::{LanguageModel, LanguageModelRequest};

/// Plugin builder — install with `app.install(Ai::…)`.
#[derive(Clone)]
pub struct Ai {
    model: Option<SharedModel>,
    fake: Option<FakeAi>,
    default_system: Option<String>,
}

impl Default for Ai {
    fn default() -> Self {
        Self::new()
    }
}

impl Ai {
    pub fn new() -> Self {
        Self {
            model: None,
            fake: None,
            default_system: None,
        }
    }

    /// Default language model (OpenAI, Anthropic, [`FakeAi`], …).
    pub fn model<M: LanguageModel>(mut self, model: M) -> Self {
        self.model = Some(SharedModel::wrap(model));
        self.fake = None;
        self
    }

    /// Install a recording fake (tests). Same as `.model(fake)` plus `client.fake()`.
    pub fn fake(fake: FakeAi) -> Self {
        Self {
            model: Some(SharedModel::wrap(fake.clone())),
            fake: Some(fake),
            default_system: None,
        }
    }

    /// Default system prompt prepended when a bound call does not set `.system()`.
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.default_system = Some(system.into());
        self
    }

    pub(crate) fn default_system(&self) -> Option<&str> {
        self.default_system.as_deref()
    }

    pub fn into_client(self) -> Result<AiClient, AiError> {
        let model = self.model.ok_or(AiError::NoModel)?;
        Ok(AiClient {
            model,
            fake: self.fake,
            default_system: self.default_system,
        })
    }

    /// Build the client without installing (tests / manual `app.state`).
    pub fn client(self) -> Result<AiClient, AiError> {
        self.into_client()
    }
}

/// Shared AI client in app state (`req.state::<AiClient>()` / `req.ai()`).
#[derive(Clone)]
pub struct AiClient {
    model: SharedModel,
    fake: Option<FakeAi>,
    default_system: Option<String>,
}

impl AiClient {
    pub fn model(&self) -> SharedModel {
        self.model.clone()
    }

    pub fn fake(&self) -> Option<&FakeAi> {
        self.fake.as_ref()
    }

    pub fn default_system(&self) -> Option<&str> {
        self.default_system.as_deref()
    }

    /// Escape hatch: full AISDK request builder pre-seeded with the installed model.
    pub fn builder(
        &self,
    ) -> aisdk::core::language_model::request::LanguageModelRequestBuilder<
        SharedModel,
        aisdk::core::language_model::request::SystemStage,
    > {
        LanguageModelRequest::builder().model(self.model.clone())
    }
}
