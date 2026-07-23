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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use layermind_reasoning::provider::{AiError, TokenUsage};
    use std::sync::atomic::{AtomicU32, Ordering};

    struct FailingThenSuccessProvider {
        name: String,
        model: String,
        fail_count: AtomicU32,
        max_fails: u32,
        success_response: String,
    }

    #[async_trait]
    impl AiProvider for FailingThenSuccessProvider {
        async fn complete(&self, _request: AiRequest) -> Result<AiResponse, AiError> {
            let attempts = self.fail_count.fetch_add(1, Ordering::SeqCst);
            if attempts < self.max_fails {
                Err(AiError::Http("transient failure".into()))
            } else {
                Ok(AiResponse {
                    content: self.success_response.clone(),
                    usage: TokenUsage {
                        prompt_tokens: 10,
                        completion_tokens: 20,
                        total_tokens: 30,
                    },
                    model: self.model.clone(),
                })
            }
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn model(&self) -> &str {
            &self.model
        }
    }

    #[test]
    fn retries_transient_http_failures() {
        let inner = FailingThenSuccessProvider {
            name: "test".into(),
            model: "m".into(),
            fail_count: AtomicU32::new(0),
            max_fails: 2,
            success_response: "success".into(),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let provider = RetryingProvider::new(Arc::new(inner), 3);
        let request = AiRequest {
            system_prompt: "s".into(),
            user_prompt: "u".into(),
            max_tokens: 100,
            temperature: 0.3,
        };

        let result = rt.block_on(provider.complete(request));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "success");
    }

    #[test]
    fn does_not_retry_unauthorized() {
        assert!(!is_retryable(&AiError::Unauthorized("bad key".into())));
    }

    #[test]
    fn does_not_retry_invalid_requests() {
        assert!(!is_retryable(&AiError::ApiError {
            status: 400,
            body: "bad request".into()
        }));
    }

    #[test]
    fn retries_rate_limited() {
        assert!(is_retryable(&AiError::RateLimited {
            retry_after: Some("5".into())
        }));
    }
}
