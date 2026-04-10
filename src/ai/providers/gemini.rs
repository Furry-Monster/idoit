use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::ai::provider::AiProvider;
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

impl AiProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            request.model, self.api_key
        );

        let body = GenerateRequest {
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
        };

        let resp = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .with_context(|| "failed to reach Gemini API")?;

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
