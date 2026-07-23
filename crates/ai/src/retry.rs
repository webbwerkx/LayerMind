//! Retrying provider — wraps any `AiProvider` with retry logic.
//!
//! Retries on: HTTP errors (timeouts, connection failures) and 429
//! rate limits. Does NOT retry: authentication failures (401/403),
//! invalid requests (400), or unknown model errors.

use std::sync::Arc;

use async_trait::async_trait;

use layermind_reasoning::provider::{AiError, AiProvider, AiRequest, AiResponse};

/// Wraps an `AiProvider` with automatic retry for transient failures.
pub struct RetryingProvider {
    inner: Arc<dyn AiProvider>,
    max_retries: u32,
}

impl RetryingProvider {
    pub fn new(provider: Arc<dyn AiProvider>, max_retries: u32) -> Self {
        Self {
            inner: provider,
            max_retries,
        }
    }
}

#[async_trait]
impl AiProvider for RetryingProvider {
    async fn complete(&self, request: AiRequest) -> Result<AiResponse, AiError> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let delay_ms = 2u64.pow(attempt) * 100; // 100ms, 200ms, 400ms...
                tracing::info!(
                    provider = %self.inner.name(),
                    attempt,
                    delay_ms,
                    "retrying AI provider request"
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }

            match self.inner.complete(request.clone()).await {
                Ok(response) => {
                    if attempt > 0 {
                        tracing::info!(
                            provider = %self.inner.name(),
                            attempt,
                            "AI provider request succeeded after retry"
                        );
                    }
                    return Ok(response);
                }
                Err(e) => {
                    let retryable = is_retryable(&e);
                    tracing::warn!(
                        provider = %self.inner.name(),
                        error = %e,
                        attempt,
                        retryable,
                        "AI provider request failed"
                    );
                    if !retryable {
                        return Err(e);
                    }
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AiError::Http("max retries exhausted".into())))
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    fn supports_structured_output(&self) -> bool {
        self.inner.supports_structured_output()
    }
}

fn is_retryable(error: &AiError) -> bool {
    matches!(error, AiError::Http(_) | AiError::RateLimited { .. })
}
