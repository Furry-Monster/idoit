use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use tokio_util::sync::CancellationToken;

use crate::cli::spinner::Spinner;
use crate::config::settings::{AiProviderId, Settings};

use super::provider::AiProvider;
use super::providers::anthropic::AnthropicProvider;
use super::providers::gemini::GeminiProvider;
use super::providers::ollama::OllamaProvider;
use super::providers::openai::OpenAiProvider;
use super::retry::{self, RetryConfig};
use super::types::{AiCommandResponse, CompletionRequest, CompletionResponse};

/// Cap streamed diagnostic text to limit memory use in the TUI.
pub const MAX_FREEFORM_STREAM_BYTES: usize = 256 * 1024;

pub struct AiClient {
    inner: AiClientInner,
    max_retries: u32,
}

enum AiClientInner {
    OpenAi(OpenAiProvider),
    Anthropic(AnthropicProvider),
    Gemini(GeminiProvider),
    Ollama(OllamaProvider),
}

pub struct AskResult {
    pub response: AiCommandResponse,
    pub elapsed: Duration,
}

impl AiClient {
    pub fn from_settings(
        settings: &Settings,
        provider_override: Option<AiProviderId>,
    ) -> Result<Self> {
        let id = provider_override.unwrap_or(settings.ai.provider);
        let timeout = Duration::from_secs(settings.ai.timeout_secs);
        let max_retries = settings.ai.max_retries;

        let inner = match id {
            AiProviderId::OpenAi => {
                let cfg = &settings.ai.openai;
                let api_key = resolve_api_key(&cfg.api_key, &cfg.api_key_env)?;
                let base_url = if cfg.base_url.is_empty() {
                    None
                } else {
                    Some(cfg.base_url.clone())
                };
                AiClientInner::OpenAi(OpenAiProvider::new(api_key, base_url, timeout))
            }
            AiProviderId::Anthropic => {
                let api_key = resolve_api_key(
                    &settings.ai.anthropic.api_key,
                    &settings.ai.anthropic.api_key_env,
                )?;
                AiClientInner::Anthropic(AnthropicProvider::new(api_key, timeout))
            }
            AiProviderId::Gemini => {
                let api_key =
                    resolve_api_key(&settings.ai.gemini.api_key, &settings.ai.gemini.api_key_env)?;
                AiClientInner::Gemini(GeminiProvider::new(api_key, timeout))
            }
            AiProviderId::Ollama => AiClientInner::Ollama(OllamaProvider::new(
                settings.ai.ollama.host.clone(),
                timeout,
            )),
        };

        Ok(Self { inner, max_retries })
    }

    pub fn model_name(&self, settings: &Settings) -> String {
        settings.ai.active_model().to_string()
    }

    pub fn provider_name(&self) -> &str {
        match &self.inner {
            AiClientInner::OpenAi(p) => p.name(),
            AiClientInner::Anthropic(p) => p.name(),
            AiClientInner::Gemini(p) => p.name(),
            AiClientInner::Ollama(p) => p.name(),
        }
    }

    async fn complete(
        &self,
        request: &CompletionRequest,
        cancel: Option<&CancellationToken>,
    ) -> Result<CompletionResponse> {
        match &self.inner {
            AiClientInner::OpenAi(p) => p.complete(request, cancel).await,
            AiClientInner::Anthropic(p) => p.complete(request, cancel).await,
            AiClientInner::Gemini(p) => p.complete(request, cancel).await,
            AiClientInner::Ollama(p) => p.complete(request, cancel).await,
        }
    }

    pub async fn ask_command(
        &self,
        system: &str,
        user_message: &str,
        model: &str,
        settings: &Settings,
        spinner: Option<&Spinner>,
        cancel: Option<&CancellationToken>,
    ) -> Result<AskResult> {
        let request = CompletionRequest {
            system: system.to_string(),
            user_message: user_message.to_string(),
            model: model.to_string(),
            temperature: settings.ai.temperature,
            max_tokens: settings.ai.max_tokens,
        };

        let spinner_clone = spinner.cloned();
        let max_retries = self.max_retries;
        let retry_config = RetryConfig {
            max_retries,
            on_retry: Some(Box::new(move |attempt, delay| {
                if let Some(ref s) = spinner_clone {
                    s.set_message(&format!(
                        "retrying ({}/{})... waiting {:.0}s",
                        attempt,
                        max_retries,
                        delay.as_secs_f32()
                    ));
                }
            })),
        };

        let start = Instant::now();
        let response =
            retry::with_retry(&retry_config, cancel, || self.complete(&request, cancel)).await?;
        let elapsed = start.elapsed();

        let parsed = parse_command_response(&response.content)?;
        Ok(AskResult {
            response: parsed,
            elapsed,
        })
    }

    pub async fn ask_freeform(
        &self,
        system: &str,
        user_message: &str,
        model: &str,
        settings: &Settings,
        cancel: Option<&CancellationToken>,
    ) -> Result<(String, Duration)> {
        let request = CompletionRequest {
            system: system.to_string(),
            user_message: user_message.to_string(),
            model: model.to_string(),
            temperature: settings.ai.temperature,
            max_tokens: settings.ai.max_tokens,
        };

        let retry_config = RetryConfig {
            max_retries: self.max_retries,
            on_retry: None,
        };

        let start = Instant::now();
        let response =
            retry::with_retry(&retry_config, cancel, || self.complete(&request, cancel)).await?;
        Ok((response.content, start.elapsed()))
    }

    /// Incremental freeform completion (SSE). Retries are not applied here to avoid duplicate deltas.
    pub async fn ask_freeform_stream<F>(
        &self,
        system: &str,
        user_message: &str,
        model: &str,
        settings: &Settings,
        cancel: Option<&CancellationToken>,
        mut on_delta: F,
    ) -> Result<(String, Duration)>
    where
        F: FnMut(&str) + Send,
    {
        let request = CompletionRequest {
            system: system.to_string(),
            user_message: user_message.to_string(),
            model: model.to_string(),
            temperature: settings.ai.temperature,
            max_tokens: settings.ai.max_tokens,
        };

        let start = Instant::now();
        let response = match &self.inner {
            AiClientInner::OpenAi(p) => {
                p.stream_complete(&request, cancel, MAX_FREEFORM_STREAM_BYTES, &mut on_delta)
                    .await?
            }
            AiClientInner::Anthropic(p) => {
                p.stream_complete(&request, cancel, MAX_FREEFORM_STREAM_BYTES, &mut on_delta)
                    .await?
            }
            AiClientInner::Gemini(p) => {
                p.stream_complete(&request, cancel, MAX_FREEFORM_STREAM_BYTES, &mut on_delta)
                    .await?
            }
            AiClientInner::Ollama(p) => p.stream_complete(&request, cancel, &mut on_delta).await?,
        };

        Ok((response.content, start.elapsed()))
    }
}

fn resolve_api_key(config_value: &str, env_var: &str) -> Result<String> {
    let from_config = config_value.trim();
    if !from_config.is_empty() {
        return Ok(from_config.to_string());
    }

    std::env::var(env_var).with_context(|| {
        format!(
            "environment variable {env_var} not set.\n\
             Set it with: export {env_var}=\"your-key\"\n\
             Or save API key in config via: idoit setup"
        )
    })
}

fn normalize_ai_command_response(mut resp: AiCommandResponse) -> Result<AiCommandResponse> {
    resp.command = resp.command.trim().to_string();
    resp.alternates = resp
        .alternates
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if resp.command.is_empty() && resp.alternates.is_empty() {
        anyhow::bail!(
            "model returned an empty command with no alternates; try another model or rephrase"
        );
    }

    Ok(resp)
}

fn parse_command_response(raw: &str) -> Result<AiCommandResponse> {
    let trimmed = raw.trim();

    let parsed = if let Ok(resp) = serde_json::from_str::<AiCommandResponse>(trimmed) {
        Some(resp)
    } else if trimmed.contains("```") {
        let inner = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        serde_json::from_str::<AiCommandResponse>(inner).ok()
    } else if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        let candidate = &trimmed[start..=end];
        serde_json::from_str::<AiCommandResponse>(candidate).ok()
    } else {
        None
    };

    match parsed {
        Some(resp) => normalize_ai_command_response(resp).with_context(|| {
            format!("after parsing JSON from the model.\nRaw response:\n{raw}")
        }),
        None => anyhow::bail!(
            "AI returned unexpected response (not valid JSON).\nTip: try a different model or provider with -p.\n\nRaw response:\n{raw}"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_direct_json() {
        let raw = r#"{"command": "rg 'Good Morning' ./", "explanation": "search for text", "missing_tools": [], "confidence": 0.95}"#;
        let resp = parse_command_response(raw).unwrap();
        assert_eq!(resp.command, "rg 'Good Morning' ./");
        assert!(resp.missing_tools.is_empty());
    }

    #[test]
    fn test_parse_json_in_code_fence() {
        let raw = "```json\n{\"command\": \"ls -la\", \"explanation\": \"list files\", \"missing_tools\": [], \"confidence\": 0.9}\n```";
        let resp = parse_command_response(raw).unwrap();
        assert_eq!(resp.command, "ls -la");
    }

    #[test]
    fn test_parse_json_with_prose() {
        let raw = "Here is the command:\n{\"command\": \"find . -name '*.rs'\", \"explanation\": \"find rust files\", \"missing_tools\": [], \"confidence\": 0.9}\nHope this helps!";
        let resp = parse_command_response(raw).unwrap();
        assert_eq!(resp.command, "find . -name '*.rs'");
    }

    #[test]
    fn test_parse_json_with_teaching() {
        let raw = r#"{"command": "grep -r 'TODO' src/", "explanation": "search for TODOs", "missing_tools": [], "confidence": 0.9, "teaching": "grep searches file contents.\n-r means recursive."}"#;
        let resp = parse_command_response(raw).unwrap();
        assert_eq!(
            resp.teaching.unwrap(),
            "grep searches file contents.\n-r means recursive."
        );
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(parse_command_response("not json").is_err());
    }

    #[test]
    fn test_parse_json_missing_optional_fields() {
        let raw = r#"{"command": "ls", "explanation": "list"}"#;
        let resp = parse_command_response(raw).unwrap();
        assert_eq!(resp.command, "ls");
        assert!(resp.missing_tools.is_empty());
        assert!(resp.teaching.is_none());
    }

    #[test]
    fn test_parse_json_with_alternates() {
        let raw = r#"{"command": "rg foo .", "explanation": "search", "missing_tools": [], "confidence": 0.9, "alternates": ["grep -r foo ."]}"#;
        let resp = parse_command_response(raw).unwrap();
        assert_eq!(resp.alternates.len(), 1);
        assert_eq!(resp.alternates[0], "grep -r foo .");
    }

    #[test]
    fn test_parse_allows_whitespace_only_command_when_alternates_present() {
        let raw = r#"{"command":"   ","explanation":"e","missing_tools":[],"alternates":["ls"]}"#;
        let resp = parse_command_response(raw).unwrap();
        assert!(resp.command.is_empty());
        assert_eq!(resp.alternates, vec!["ls".to_string()]);
    }

    #[test]
    fn test_parse_rejects_empty_command_and_no_alternates() {
        let raw = r#"{"command":"","explanation":"e","missing_tools":[]}"#;
        assert!(parse_command_response(raw).is_err());
    }
}
