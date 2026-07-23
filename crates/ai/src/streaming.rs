//! Streaming AI provider trait — prepared for future streaming support.
//!
//! This trait is additive: providers MAY implement it in addition to
//! `AiProvider`. PrintDoctor does not use it yet; it exists so Phase 2.4
//! local model support can add streaming without redesign.

use std::pin::Pin;

use async_trait::async_trait;
use futures_util::Stream;

use layermind_reasoning::provider::{AiError, AiRequest};

/// A chunk of a streaming AI response.
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// Delta text content for this chunk.
    pub content: String,
    /// Whether this is the final chunk.
    pub done: bool,
}

/// Optional trait for providers that support token-by-token streaming.
///
/// Implementations should return a `Stream` of `StreamChunk` values.
/// The stream ends after the final `done: true` chunk.
#[async_trait]
pub trait StreamingAiProvider: Send + Sync {
    /// Stream a completion request, yielding text chunks as they arrive.
    async fn stream_complete(
        &self,
        request: AiRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, AiError>> + Send>>, AiError>;
}
