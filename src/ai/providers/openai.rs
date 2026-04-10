use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::ai::provider::AiProvider;
use crate::ai::types::{CompletionRequest, CompletionResponse};

pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f64,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

impl OpenAiProvider {
    pub fn new(api_key: String, base_url: Option<String>, timeout: Duration) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap_or_default(),
            api_key,
            base_url: base_url
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "https://api.openai.com/v1".into()),
        }
    }
}

impl AiProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let body = ChatRequest {
            model: request.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: request.system.clone(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: request.user_message.clone(),
                },
            ],
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .with_context(|| "failed to reach AI provider")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("AI provider returned {status}: {text}");
        }

        let chat_resp: ChatResponse = resp
            .json()
            .await
            .with_context(|| "failed to parse AI response")?;

        let content = chat_resp
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();

        Ok(CompletionResponse { content })
    }
}
