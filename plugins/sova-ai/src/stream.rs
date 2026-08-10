//! Map AISDK text streams onto Sova [`Response::sse`].

use aisdk::core::language_model::LanguageModelStreamChunkType;
use aisdk::core::StreamTextResponse;
use futures_util::StreamExt;
use sova_core::Response;

/// Convert [`StreamTextResponse`] into an SSE HTTP response (`data:` lines = text deltas).
pub fn stream_to_response(response: StreamTextResponse) -> Response {
    let stream = response.stream;
    let mapped = stream.filter_map(|chunk| async move {
        match chunk {
            LanguageModelStreamChunkType::Text(t) | LanguageModelStreamChunkType::Reasoning(t) => {
                Some(Ok::<_, std::convert::Infallible>(t))
            }
            LanguageModelStreamChunkType::End(_) => {
                Some(Ok::<_, std::convert::Infallible>("[DONE]".into()))
            }
            LanguageModelStreamChunkType::Failed(e)
            | LanguageModelStreamChunkType::Incomplete(e)
            | LanguageModelStreamChunkType::NotSupported(e) => {
                Some(Ok::<_, std::convert::Infallible>(format!("error: {e}")))
            }
            LanguageModelStreamChunkType::ToolCall(t) => Some(Ok::<_, std::convert::Infallible>(t)),
            LanguageModelStreamChunkType::Start => None,
        }
    });
    Response::sse(mapped)
}
