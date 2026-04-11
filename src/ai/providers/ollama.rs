use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::ai::provider::AiProvider;
use crate::ai::types::{CompletionRequest, CompletionResponse};

pub struct OllamaProvider {
    client: reqwest::Client,
    host: String,
}

#[derive(Serialize)]
struct GenerateRequest {
    model: String,
    system: String,
    prompt: String,
    stream: bool,
    options: GenerateOptions,
}

#[derive(Serialize)]
struct GenerateOptions {
    temperature: f64,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

impl OllamaProvider {
    pub fn new(host: String, timeout: Duration) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap_or_default(),
            host: host.trim_end_matches('/').to_string(),
        }
    }
}

fn check_cancel(cancel: Option<&CancellationToken>) -> Result<()> {
    if cancel.map(|c| c.is_cancelled()).unwrap_or(false) {
        bail!("cancelled");
    }
    Ok(())
}

impl OllamaProvider {
    /// Streaming path falls back to one-shot completion and invokes `on_delta` once.
    pub async fn stream_complete<F>(
        &self,
        request: &CompletionRequest,
        cancel: Option<&CancellationToken>,
        mut on_delta: F,
    ) -> Result<CompletionResponse>
    where
        F: FnMut(&str) + Send,
    {
        let r = self.complete(request, cancel).await?;
        if !r.content.is_empty() {
            on_delta(&r.content);
        }
        Ok(r)
    }
}

impl AiProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn complete(
        &self,
        request: &CompletionRequest,
        cancel: Option<&CancellationToken>,
    ) -> Result<CompletionResponse> {
        let url = format!("{}/api/generate", self.host);

        let body = GenerateRequest {
            model: request.model.clone(),
            system: request.system.clone(),
            prompt: request.user_message.clone(),
            stream: false,
            options: GenerateOptions {
                temperature: request.temperature,
            },
        };

        check_cancel(cancel)?;
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("failed to reach Ollama at {}", self.host))?;

        check_cancel(cancel)?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("Ollama returned {status}: {text}");
        }

        let gen_resp: GenerateResponse = resp
            .json()
            .await
            .with_context(|| "failed to parse Ollama response")?;

        Ok(CompletionResponse {
            content: gen_resp.response,
        })
    }
}
