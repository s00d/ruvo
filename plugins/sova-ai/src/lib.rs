//! AISDK language models for Sova — `app.install(Ai::…)` / `req.ai()`.
//!
//! Thin shell over [`aisdk`](https://crates.io/crates/aisdk): install a default
//! model into app state, call it from handlers, or drop down to the full
//! `LanguageModelRequest` builder. [`FakeAi`] records prompts for tests.
//!
//! ```ignore
//! use sova_ai::{Ai, AiExt, FakeAi};
//!
//! let fake = FakeAi::new().stub_text("pong");
//! app.install(Ai::fake(fake.clone()));
//!
//! // in a handler:
//! let out = req.ai().prompt("ping").generate().await?;
//! assert_eq!(out.text().as_deref(), Some("pong"));
//! ```

mod bound;
mod client;
mod error;
mod fake;
mod model;
mod stream;

pub use bound::{AiBound, AiExt};
pub use client::{Ai, AiClient};
pub use error::AiError;
pub use fake::FakeAi;
pub use model::SharedModel;
pub use stream::stream_to_response;

/// Re-export the upstream SDK so apps can use providers / tools without a second dep.
pub use aisdk;

/// Common AISDK imports for handlers and agents.
pub mod prelude {
    pub use aisdk::core::{
        utils::step_count_is, GenerateTextResponse, LanguageModel, LanguageModelRequest,
        LanguageModelStreamChunkType, Message, Messages, Tool,
    };
    pub use aisdk::macros::tool;
    pub use aisdk::{Error as AisdkError, Result as AisdkResult};

    pub use crate::{Ai, AiBound, AiClient, AiError, AiExt, FakeAi, SharedModel};
}

use sova_core::{App, Plugin};

impl Plugin for Ai {
    fn id(&self) -> &'static str {
        "ai"
    }

    fn meta(&self) -> sova_core::PluginMeta {
        sova_core::PluginMeta::new("Ai")
            .description("AISDK language models (chat, tools, stream, fake)")
            .version(env!("CARGO_PKG_VERSION"))
    }

    fn install(self, app: &mut App) {
        let mut ai = self;
        if ai.default_system().is_none() {
            if let Some(doc) = app.config_doc() {
                if let Some(section) = doc.section("ai") {
                    if let Some(system) = section.get("system").and_then(|v| v.as_str()) {
                        ai = ai.system(system);
                    }
                }
            }
        }
        let client = match ai.into_client() {
            Ok(c) => c,
            Err(err) => {
                tracing::error!(error = %err, "ai plugin install failed");
                panic!("ai install failed: {err}");
            }
        };
        app.state(client);
    }
}
