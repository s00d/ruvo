//! In-memory fake [`LanguageModel`] for tests.

use aisdk::core::capabilities::{
    ReasoningSupport, StructuredOutputSupport, TextInputSupport, TextOutputSupport, ToolCallSupport,
};
use aisdk::core::language_model::{
    LanguageModel, LanguageModelOptions, LanguageModelResponse, LanguageModelStreamChunk,
    LanguageModelStreamChunkType,
};
use aisdk::core::Message;
use aisdk::Result as AisdkResult;
use async_trait::async_trait;
use futures_util::stream;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// Recorded call snapshot (system + user prompts extracted from options).
#[derive(Debug, Clone)]
pub struct FakeCall {
    /// System prompt, if any.
    pub system: Option<String>,
    /// Flattened user/text prompts from the conversation.
    pub prompts: Vec<String>,
}

#[derive(Debug, Default)]
struct FakeInner {
    stubs: Vec<String>,
    default: String,
    calls: Vec<FakeCall>,
}

/// Laravel-style fake model: stub replies, assert prompts.
#[derive(Clone, Debug, Default)]
pub struct FakeAi {
    inner: Arc<Mutex<FakeInner>>,
}

impl FakeAi {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a one-shot stub (FIFO). Falls back to [`Self::stub_text`] default.
    pub fn push_text(self, text: impl Into<String>) -> Self {
        self.inner.lock().unwrap().stubs.push(text.into());
        self
    }

    /// Default reply when the stub queue is empty (also sets initial queue empty).
    pub fn stub_text(self, text: impl Into<String>) -> Self {
        self.inner.lock().unwrap().default = text.into();
        self
    }

    fn next_text(&self) -> String {
        let mut g = self.inner.lock().unwrap();
        if !g.stubs.is_empty() {
            g.stubs.remove(0)
        } else {
            g.default.clone()
        }
    }

    fn record(&self, options: &LanguageModelOptions) {
        let prompts = options
            .messages()
            .into_iter()
            .filter_map(|m| match m {
                Message::User(u) => Some(u.content),
                Message::System(s) => Some(s.content),
                _ => None,
            })
            .collect();
        self.inner.lock().unwrap().calls.push(FakeCall {
            system: options.system.clone(),
            prompts,
        });
    }

    /// All recorded calls (oldest first).
    pub fn calls(&self) -> Vec<FakeCall> {
        self.inner.lock().unwrap().calls.clone()
    }

    /// Flattened user/system prompt strings across calls.
    pub fn prompts(&self) -> Vec<String> {
        self.calls().into_iter().flat_map(|c| c.prompts).collect()
    }

    pub fn call_count(&self) -> usize {
        self.inner.lock().unwrap().calls.len()
    }

    pub fn assert_called(&self) {
        assert!(
            self.call_count() > 0,
            "FakeAi: expected at least one generate/stream call"
        );
    }

    pub fn assert_called_times(&self, n: usize) {
        assert_eq!(
            self.call_count(),
            n,
            "FakeAi: expected {n} calls, got {}",
            self.call_count()
        );
    }
}

impl TextInputSupport for FakeAi {}
impl TextOutputSupport for FakeAi {}
impl ToolCallSupport for FakeAi {}
impl StructuredOutputSupport for FakeAi {}
impl ReasoningSupport for FakeAi {}

#[async_trait]
impl LanguageModel for FakeAi {
    fn name(&self) -> String {
        "fake".into()
    }

    async fn generate_text(
        &mut self,
        options: LanguageModelOptions,
    ) -> AisdkResult<LanguageModelResponse> {
        self.record(&options);
        Ok(LanguageModelResponse::new(self.next_text()))
    }

    async fn stream_text(
        &mut self,
        options: LanguageModelOptions,
    ) -> AisdkResult<
        Pin<
            Box<dyn futures_util::Stream<Item = AisdkResult<Vec<LanguageModelStreamChunk>>> + Send>,
        >,
    > {
        self.record(&options);
        let text = self.next_text();
        let chunks = vec![
            LanguageModelStreamChunk::Delta(LanguageModelStreamChunkType::Start),
            LanguageModelStreamChunk::Delta(LanguageModelStreamChunkType::Text(text)),
            LanguageModelStreamChunk::Delta(LanguageModelStreamChunkType::End(Default::default())),
        ];
        Ok(Box::pin(stream::once(async move { Ok(chunks) })))
    }
}
