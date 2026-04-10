use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

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
    temperature: f32,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

impl OllamaProvider {
    pub fn new(host: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            host: host.trim_end_matches('/').to_string(),
        }
    }
}

impl AiProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
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

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("failed to reach Ollama at {}", self.host))?;

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
