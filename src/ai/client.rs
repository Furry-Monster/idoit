use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use crate::config::settings::Settings;
use crate::ui::spinner::Spinner;

use super::provider::AiProvider;
use super::providers::anthropic::AnthropicProvider;
use super::providers::gemini::GeminiProvider;
use super::providers::ollama::OllamaProvider;
use super::providers::openai::OpenAiProvider;
use super::retry::{self, RetryConfig};
use super::types::{AiCommandResponse, CompletionRequest, CompletionResponse};

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
    pub fn from_settings(settings: &Settings, provider_override: Option<&str>) -> Result<Self> {
        let provider_name = provider_override.unwrap_or(&settings.ai.provider);
        let timeout = Duration::from_secs(settings.ai.timeout_secs);
        let max_retries = settings.ai.max_retries;

        let inner = match provider_name {
            "openai" => {
                let cfg = &settings.ai.openai;
                let api_key = resolve_api_key(&cfg.api_key, &cfg.api_key_env)?;
                let base_url = if cfg.base_url.is_empty() {
                    None
                } else {
                    Some(cfg.base_url.clone())
                };
                AiClientInner::OpenAi(OpenAiProvider::new(api_key, base_url, timeout))
            }
            "anthropic" => {
                let api_key = resolve_api_key(
                    &settings.ai.anthropic.api_key,
                    &settings.ai.anthropic.api_key_env,
                )?;
                AiClientInner::Anthropic(AnthropicProvider::new(api_key, timeout))
            }
            "gemini" => {
                let api_key =
                    resolve_api_key(&settings.ai.gemini.api_key, &settings.ai.gemini.api_key_env)?;
                AiClientInner::Gemini(GeminiProvider::new(api_key, timeout))
            }
            "ollama" => AiClientInner::Ollama(OllamaProvider::new(
                settings.ai.ollama.host.clone(),
                timeout,
            )),
            other => bail!(
                "unknown AI provider: {other} (expected openai, anthropic, gemini, or ollama)"
            ),
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

    async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        match &self.inner {
            AiClientInner::OpenAi(p) => p.complete(request).await,
            AiClientInner::Anthropic(p) => p.complete(request).await,
            AiClientInner::Gemini(p) => p.complete(request).await,
            AiClientInner::Ollama(p) => p.complete(request).await,
        }
    }

    pub async fn ask_command(
        &self,
        system: &str,
        user_message: &str,
        model: &str,
        settings: &Settings,
        spinner: Option<&Spinner>,
    ) -> Result<AskResult> {
        let request = CompletionRequest {
            system: system.to_string(),
            user_message: user_message.to_string(),
            model: model.to_string(),
            temperature: settings.ai.temperature,
            max_tokens: settings.ai.max_tokens,
        };

        let spinner_clone = spinner.cloned();
        let retry_config = RetryConfig {
            max_retries: self.max_retries,
            on_retry: Some(Box::new(move |attempt, delay| {
                if let Some(ref s) = spinner_clone {
                    s.set_message(&format!(
                        "retrying ({}/{})... waiting {:.0}s",
                        attempt,
                        3,
                        delay.as_secs_f32()
                    ));
                }
            })),
        };

        let start = Instant::now();
        let response = retry::with_retry(&retry_config, || self.complete(&request)).await?;
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
        let response = retry::with_retry(&retry_config, || self.complete(&request)).await?;
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

fn parse_command_response(raw: &str) -> Result<AiCommandResponse> {
    let trimmed = raw.trim();

    if let Ok(resp) = serde_json::from_str::<AiCommandResponse>(trimmed) {
        return Ok(resp);
    }

    if trimmed.contains("```") {
        let inner = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        if let Ok(resp) = serde_json::from_str::<AiCommandResponse>(inner) {
            return Ok(resp);
        }
    }

    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            let candidate = &trimmed[start..=end];
            if let Ok(resp) = serde_json::from_str::<AiCommandResponse>(candidate) {
                return Ok(resp);
            }
        }
    }

    anyhow::bail!("AI returned unexpected response (not valid JSON).\nTip: try a different model or provider with -p.\n\nRaw response:\n{raw}")
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
}
