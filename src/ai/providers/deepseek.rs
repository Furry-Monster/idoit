//! DeepSeek — OpenAI-compatible Chat Completions (`https://api.deepseek.com/v1`).

use std::time::Duration;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use crate::ai::provider::AiProvider;
use crate::ai::providers::openai::OpenAiProvider;
use crate::ai::types::{CompletionRequest, CompletionResponse};

const DEFAULT_BASE_URL: &str = "https://api.deepseek.com/v1";

/// Wraps [`OpenAiProvider`] with DeepSeek defaults and `provider_name` `"deepseek"`.
pub struct DeepSeekProvider {
    inner: OpenAiProvider,
}

impl DeepSeekProvider {
    pub fn new(api_key: String, base_url: Option<String>, timeout: Duration) -> Self {
        let url = base_url
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        Self {
            inner: OpenAiProvider::new(api_key, Some(url), timeout),
        }
    }

    pub async fn stream_complete<F>(
        &self,
        request: &CompletionRequest,
        cancel: Option<&CancellationToken>,
        max_stream_bytes: usize,
        on_delta: F,
    ) -> Result<CompletionResponse>
    where
        F: FnMut(&str) + Send,
    {
        self.inner
            .stream_complete(request, cancel, max_stream_bytes, on_delta)
            .await
    }
}

impl AiProvider for DeepSeekProvider {
    fn name(&self) -> &str {
        "deepseek"
    }

    async fn complete(
        &self,
        request: &CompletionRequest,
        cancel: Option<&CancellationToken>,
    ) -> Result<CompletionResponse> {
        self.inner.complete(request, cancel).await
    }
}
