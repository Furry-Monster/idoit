use std::time::Duration;

use anyhow::{bail, Context, Result};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::ai::provider::AiProvider;
use crate::ai::stream::extract_openai_delta;
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
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    stream: bool,
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

fn check_cancel(cancel: Option<&CancellationToken>) -> Result<()> {
    if cancel.map(|c| c.is_cancelled()).unwrap_or(false) {
        bail!("cancelled");
    }
    Ok(())
}

impl OpenAiProvider {
    pub async fn stream_complete<F>(
        &self,
        request: &CompletionRequest,
        cancel: Option<&CancellationToken>,
        max_stream_bytes: usize,
        mut on_delta: F,
    ) -> Result<CompletionResponse>
    where
        F: FnMut(&str) + Send,
    {
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
            stream: true,
        };

        check_cancel(cancel)?;
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .with_context(|| "failed to reach AI provider")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("AI provider returned {status}: {text}");
        }

        check_cancel(cancel)?;
        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        let mut full = String::new();

        while let Some(chunk) = stream.next().await {
            check_cancel(cancel)?;
            let chunk = chunk.with_context(|| "OpenAI stream read failed")?;
            buf.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buf.find("\n\n") {
                let frame: String = buf[..pos].to_string();
                buf.drain(..=pos + 1);

                for line in frame.lines() {
                    let line = line.trim();
                    let Some(data) = line.strip_prefix("data: ") else {
                        continue;
                    };
                    if data == "[DONE]" {
                        continue;
                    }
                    if let Some(d) = extract_openai_delta(data) {
                        if full.len() + d.len() > max_stream_bytes {
                            bail!("stream exceeded max size ({max_stream_bytes} bytes)");
                        }
                        full.push_str(&d);
                        on_delta(&d);
                    }
                }
            }
        }

        Ok(CompletionResponse { content: full })
    }
}

impl AiProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn complete(
        &self,
        request: &CompletionRequest,
        cancel: Option<&CancellationToken>,
    ) -> Result<CompletionResponse> {
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
            stream: false,
        };

        check_cancel(cancel)?;
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .with_context(|| "failed to reach AI provider")?;

        check_cancel(cancel)?;
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
