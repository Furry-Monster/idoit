use std::time::Duration;

use anyhow::{bail, Context, Result};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::ai::provider::AiProvider;
use crate::ai::stream::extract_gemini_delta;
use crate::ai::types::{CompletionRequest, CompletionResponse};

pub struct GeminiProvider {
    client: reqwest::Client,
    api_key: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateRequest {
    system_instruction: Option<SystemInstruction>,
    contents: Vec<Content>,
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Serialize, Deserialize)]
struct Content {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<Part>,
}

#[derive(Serialize, Deserialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    temperature: f64,
    max_output_tokens: u32,
}

#[derive(Deserialize)]
struct GenerateResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Content,
}

impl GeminiProvider {
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

impl GeminiProvider {
    fn build_body(request: &CompletionRequest) -> GenerateRequest {
        GenerateRequest {
            system_instruction: Some(SystemInstruction {
                parts: vec![Part {
                    text: request.system.clone(),
                }],
            }),
            contents: vec![Content {
                role: Some("user".into()),
                parts: vec![Part {
                    text: request.user_message.clone(),
                }],
            }],
            generation_config: GenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
            },
        }
    }

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
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            request.model, self.api_key
        );
        let body = Self::build_body(request);

        check_cancel(cancel)?;
        let resp = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .with_context(|| "failed to reach Gemini API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("Gemini API returned {status}: {text}");
        }

        check_cancel(cancel)?;
        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        let mut full = String::new();

        while let Some(chunk) = stream.next().await {
            check_cancel(cancel)?;
            let chunk = chunk.with_context(|| "Gemini stream read failed")?;
            buf.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buf.find("\n\n") {
                let frame: String = buf[..pos].to_string();
                buf.drain(..=pos + 1);

                for line in frame.lines() {
                    let line = line.trim();
                    let Some(data) = line.strip_prefix("data: ") else {
                        continue;
                    };
                    if let Some(d) = extract_gemini_delta(data) {
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

impl AiProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    async fn complete(
        &self,
        request: &CompletionRequest,
        cancel: Option<&CancellationToken>,
    ) -> Result<CompletionResponse> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            request.model, self.api_key
        );

        let body = Self::build_body(request);

        check_cancel(cancel)?;
        let resp = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .with_context(|| "failed to reach Gemini API")?;

        check_cancel(cancel)?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("Gemini API returned {status}: {text}");
        }

        let gen_resp: GenerateResponse = resp
            .json()
            .await
            .with_context(|| "failed to parse Gemini response")?;

        let content = gen_resp
            .candidates
            .into_iter()
            .next()
            .map(|c| {
                c.content
                    .parts
                    .into_iter()
                    .map(|p| p.text)
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        Ok(CompletionResponse { content })
    }
}
