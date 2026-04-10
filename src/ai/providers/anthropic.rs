use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::ai::provider::AiProvider;
use crate::ai::types::{CompletionRequest, CompletionResponse};

pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
}

#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
    temperature: f32,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }
}

impl AiProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        let body = MessagesRequest {
            model: request.model.clone(),
            max_tokens: 1024,
            system: request.system.clone(),
            messages: vec![Message {
                role: "user".into(),
                content: request.user_message.clone(),
            }],
            temperature: request.temperature,
        };

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .with_context(|| "failed to reach Anthropic API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("Anthropic API returned {status}: {text}");
        }

        let msg_resp: MessagesResponse = resp
            .json()
            .await
            .with_context(|| "failed to parse Anthropic response")?;

        let content = msg_resp
            .content
            .into_iter()
            .filter_map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(CompletionResponse { content })
    }
}
