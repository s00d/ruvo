//! Type-erased [`LanguageModel`](aisdk::core::LanguageModel) handle stored in [`crate::AiClient`].

use aisdk::core::capabilities::{
    ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport,
    ToolCallSupport,
};
use aisdk::core::language_model::{
    LanguageModel, LanguageModelOptions, LanguageModelResponse, LanguageModelStreamChunk,
};
use aisdk::Result as AisdkResult;
use async_trait::async_trait;
use futures_util::Stream;
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

type ProviderStream =
    Pin<Box<dyn Stream<Item = AisdkResult<Vec<LanguageModelStreamChunk>>> + Send>>;

#[async_trait]
trait DynModel: Send + Sync + fmt::Debug {
    fn name(&self) -> String;
    async fn generate_text(&self, options: LanguageModelOptions)
        -> AisdkResult<LanguageModelResponse>;
    async fn stream_text(&self, options: LanguageModelOptions) -> AisdkResult<ProviderStream>;
}

struct Holding<M: LanguageModel> {
    name: String,
    inner: Mutex<M>,
}

impl<M: LanguageModel> fmt::Debug for Holding<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Holding").field("name", &self.name).finish()
    }
}

#[async_trait]
impl<M: LanguageModel> DynModel for Holding<M> {
    fn name(&self) -> String {
        self.name.clone()
    }

    async fn generate_text(
        &self,
        options: LanguageModelOptions,
    ) -> AisdkResult<LanguageModelResponse> {
        self.inner.lock().await.generate_text(options).await
    }

    async fn stream_text(&self, options: LanguageModelOptions) -> AisdkResult<ProviderStream> {
        self.inner.lock().await.stream_text(options).await
    }
}

/// Cloneable model handle used as the default for `LanguageModelRequest`.
///
/// Implements AISDK capability markers so the typed builder accepts prompts/tools
/// (runtime model selection — same idea as aisdk `DynamicModel`).
#[derive(Clone)]
pub struct SharedModel {
    inner: Arc<dyn DynModel>,
}

impl fmt::Debug for SharedModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedModel")
            .field("name", &self.inner.name())
            .finish()
    }
}

impl SharedModel {
    /// Wrap any AISDK [`LanguageModel`] (provider or [`crate::FakeAi`]).
    pub fn wrap<M: LanguageModel>(model: M) -> Self {
        let name = model.name();
        Self {
            inner: Arc::new(Holding {
                name,
                inner: Mutex::new(model),
            }),
        }
    }
}

impl TextInputSupport for SharedModel {}
impl TextOutputSupport for SharedModel {}
impl ToolCallSupport for SharedModel {}
impl StructuredOutputSupport for SharedModel {}
impl ReasoningSupport for SharedModel {}

#[async_trait]
impl LanguageModel for SharedModel {
    fn name(&self) -> String {
        self.inner.name()
    }

    async fn generate_text(
        &mut self,
        options: LanguageModelOptions,
    ) -> AisdkResult<LanguageModelResponse> {
        self.inner.generate_text(options).await
    }

    async fn stream_text(&mut self, options: LanguageModelOptions) -> AisdkResult<ProviderStream> {
        self.inner.stream_text(options).await
    }
}
