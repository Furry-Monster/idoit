use anyhow::{bail, Context, Result};

use crate::config::settings::Settings;

use super::providers::anthropic::AnthropicProvider;
use super::providers::ollama::OllamaProvider;
use super::providers::openai::OpenAiProvider;
use super::provider::AiProvider;
use super::types::{AiCommandResponse, CompletionRequest, CompletionResponse};

pub enum AiClient {
    OpenAi(OpenAiProvider),
    Anthropic(AnthropicProvider),
    Ollama(OllamaProvider),
}

impl AiClient {
    pub fn from_settings(settings: &Settings, provider_override: Option<&str>) -> Result<Self> {
        let provider_name = provider_override.unwrap_or(&settings.ai.provider);

        match provider_name {
            "openai" => {
                let api_key = std::env::var(&settings.ai.api_key_env).with_context(|| {
                    format!(
                        "environment variable {} not set — configure your API key",
                        settings.ai.api_key_env
                    )
                })?;
                let base_url = if settings.ai.base_url.is_empty() {
                    None
                } else {
                    Some(settings.ai.base_url.clone())
                };
                Ok(Self::OpenAi(OpenAiProvider::new(api_key, base_url)))
            }
            "anthropic" => {
                let key_env = if settings.ai.api_key_env == "OPENAI_API_KEY" {
                    "ANTHROPIC_API_KEY"
                } else {
                    &settings.ai.api_key_env
                };
                let api_key = std::env::var(key_env).with_context(|| {
                    format!("environment variable {key_env} not set — configure your Anthropic API key")
                })?;
                Ok(Self::Anthropic(AnthropicProvider::new(api_key)))
            }
            "ollama" => Ok(Self::Ollama(OllamaProvider::new(
                settings.ai.ollama.host.clone(),
            ))),
            other => bail!("unknown AI provider: {other} (expected openai, anthropic, or ollama)"),
        }
    }

    pub fn model_name(&self, settings: &Settings) -> String {
        match self {
            Self::Ollama(_) => settings.ai.ollama.model.clone(),
            _ => settings.ai.model.clone(),
        }
    }

    pub async fn complete(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        match self {
            Self::OpenAi(p) => p.complete(request).await,
            Self::Anthropic(p) => p.complete(request).await,
            Self::Ollama(p) => p.complete(request).await,
        }
    }

    #[allow(dead_code)]
    pub fn provider_name(&self) -> &str {
        match self {
            Self::OpenAi(p) => p.name(),
            Self::Anthropic(p) => p.name(),
            Self::Ollama(p) => p.name(),
        }
    }

    pub async fn ask_command(
        &self,
        system: &str,
        user_message: &str,
        model: &str,
    ) -> Result<AiCommandResponse> {
        let request = CompletionRequest {
            system: system.to_string(),
            user_message: user_message.to_string(),
            model: model.to_string(),
            temperature: 0.1,
        };

        let response = self.complete(&request).await?;
        parse_command_response(&response.content)
    }
}

fn parse_command_response(raw: &str) -> Result<AiCommandResponse> {
    let trimmed = raw.trim();

    // Strategy 1: direct parse
    if let Ok(resp) = serde_json::from_str::<AiCommandResponse>(trimmed) {
        return Ok(resp);
    }

    // Strategy 2: strip markdown code fences
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

    // Strategy 3: find first { ... } block in the response
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            let candidate = &trimmed[start..=end];
            if let Ok(resp) = serde_json::from_str::<AiCommandResponse>(candidate) {
                return Ok(resp);
            }
        }
    }

    anyhow::bail!("AI returned invalid JSON:\n{raw}")
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
        assert_eq!(resp.teaching.unwrap(), "grep searches file contents.\n-r means recursive.");
    }

    #[test]
    fn test_parse_invalid_json() {
        let raw = "this is not json at all";
        assert!(parse_command_response(raw).is_err());
    }

    #[test]
    fn test_parse_json_missing_optional_fields() {
        let raw = r#"{"command": "ls", "explanation": "list"}"#;
        let resp = parse_command_response(raw).unwrap();
        assert_eq!(resp.command, "ls");
        assert!(resp.missing_tools.is_empty());
        assert_eq!(resp.confidence, 0.0);
        assert!(resp.teaching.is_none());
    }
}
