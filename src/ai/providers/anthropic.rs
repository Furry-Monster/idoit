use std::time::Duration;

use anyhow::{bail, Context, Result};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::ai::provider::AiProvider;
use crate::ai::stream::extract_anthropic_delta;
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
    temperature: f64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    stream: bool,
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
    pub fn new(api_key: String, timeout: Duration) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap_or_default(),
            api_key,
        }
    }
}

fn check_cancel(cancel: Option<&CancellationToken>) -> Result<()> {
    if cancel.map(|c| c.is_cancelled()).unwrap_or(false) {
        bail!("cancelled");
    }
    Ok(())
}

impl AnthropicProvider {
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
        let body = MessagesRequest {
            model: request.model.clone(),
            max_tokens: request.max_tokens,
            system: request.system.clone(),
            messages: vec![Message {
                role: "user".into(),
                content: request.user_message.clone(),
            }],
            temperature: request.temperature,
            stream: true,
        };

        check_cancel(cancel)?;
        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .with_context(|| "failed to reach Anthropic API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("Anthropic API returned {status}: {text}");
        }

        check_cancel(cancel)?;
        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        let mut full = String::new();

        while let Some(chunk) = stream.next().await {
            check_cancel(cancel)?;
            let chunk = chunk.with_context(|| "Anthropic stream read failed")?;
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
                    if let Some(d) = extract_anthropic_delta(data) {
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

impl AiProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn complete(
        &self,
        request: &CompletionRequest,
        cancel: Option<&CancellationToken>,
    ) -> Result<CompletionResponse> {
        let body = MessagesRequest {
            model: request.model.clone(),
            max_tokens: request.max_tokens,
            system: request.system.clone(),
            messages: vec![Message {
                role: "user".into(),
                content: request.user_message.clone(),
            }],
            temperature: request.temperature,
            stream: false,
        };

        check_cancel(cancel)?;
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

        check_cancel(cancel)?;
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
