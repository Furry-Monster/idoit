//! `idoit config` — show, get, set, and list keys for `~/.config/idoit/config.toml`.

use anyhow::{bail, Context, Result};

use crate::config;
use crate::config::settings::{AiProviderId, Settings};
use crate::parser::ConfigCommand;

pub fn run(cmd: Option<&ConfigCommand>) -> Result<()> {
    let mut settings = config::load().unwrap_or_else(|_| Settings::default());

    match cmd.unwrap_or(&ConfigCommand::Show) {
        ConfigCommand::Show => config::show_config(&settings),
        ConfigCommand::Keys => {
            println!("{}", RECOGNIZED_KEYS.trim());
            Ok(())
        }
        ConfigCommand::Get { key } => {
            let v = get_key(&settings, key)?;
            println!("{v}");
            Ok(())
        }
        ConfigCommand::Set { key, value } => {
            if value.is_empty() {
                bail!("missing value (see: idoit config keys)");
            }
            let joined = value.join(" ");
            apply_set(&mut settings, key, &joined)?;
            config::save(&settings)?;
            println!("Updated {}. Saved to {}", key, config::config_path().display());
            Ok(())
        }
    }
}

fn parse_provider(s: &str) -> Result<AiProviderId> {
    match s.trim().to_lowercase().as_str() {
        "openai" => Ok(AiProviderId::OpenAi),
        "anthropic" => Ok(AiProviderId::Anthropic),
        "gemini" => Ok(AiProviderId::Gemini),
        "deepseek" => Ok(AiProviderId::DeepSeek),
        "ollama" => Ok(AiProviderId::Ollama),
        _ => bail!(
            "unknown provider {s:?} (expected: openai, anthropic, gemini, deepseek, ollama)"
        ),
    }
}

fn parse_bool(s: &str) -> Result<bool> {
    match s.trim().to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => bail!("expected boolean (true/false, yes/no, 1/0), got {s:?}"),
    }
}

fn norm_key(key: &str) -> String {
    key.trim().to_lowercase()
}

pub fn get_key(settings: &Settings, key: &str) -> Result<String> {
    match norm_key(key).as_str() {
        "ai.provider" => Ok(settings.ai.provider.to_string()),
        "ai.timeout_secs" => Ok(settings.ai.timeout_secs.to_string()),
        "ai.temperature" => Ok(settings.ai.temperature.to_string()),
        "ai.max_tokens" => Ok(settings.ai.max_tokens.to_string()),
        "ai.max_retries" => Ok(settings.ai.max_retries.to_string()),
        "ai.openai.model" => Ok(settings.ai.openai.model.clone()),
        "ai.openai.api_key" => Ok(settings.ai.openai.api_key.clone()),
        "ai.openai.api_key_env" => Ok(settings.ai.openai.api_key_env.clone()),
        "ai.openai.base_url" => Ok(settings.ai.openai.base_url.clone()),
        "ai.anthropic.model" => Ok(settings.ai.anthropic.model.clone()),
        "ai.anthropic.api_key" => Ok(settings.ai.anthropic.api_key.clone()),
        "ai.anthropic.api_key_env" => Ok(settings.ai.anthropic.api_key_env.clone()),
        "ai.gemini.model" => Ok(settings.ai.gemini.model.clone()),
        "ai.gemini.api_key" => Ok(settings.ai.gemini.api_key.clone()),
        "ai.gemini.api_key_env" => Ok(settings.ai.gemini.api_key_env.clone()),
        "ai.deepseek.model" => Ok(settings.ai.deepseek.model.clone()),
        "ai.deepseek.api_key" => Ok(settings.ai.deepseek.api_key.clone()),
        "ai.deepseek.api_key_env" => Ok(settings.ai.deepseek.api_key_env.clone()),
        "ai.deepseek.base_url" => Ok(settings.ai.deepseek.base_url.clone()),
        "ai.ollama.model" => Ok(settings.ai.ollama.model.clone()),
        "ai.ollama.host" => Ok(settings.ai.ollama.host.clone()),
        "behavior.auto_confirm" => Ok(settings.behavior.auto_confirm.to_string()),
        "behavior.learn_by_default" => Ok(settings.behavior.learn_by_default.to_string()),
        "behavior.shell" => Ok(settings.behavior.shell.clone()),
        "behavior.history_path" => Ok(settings.behavior.history_path.clone()),
        "ui.color" => Ok(settings.ui.color.to_string()),
        "ui.verbose" => Ok(settings.ui.verbose.to_string()),
        "ui.tui_debounce_ms" => Ok(settings.ui.tui_debounce_ms.to_string()),
        _ => bail!("unknown key — run `idoit config keys` for recognized dot-paths"),
    }
}

fn apply_set(settings: &mut Settings, key: &str, val: &str) -> Result<()> {
    match norm_key(key).as_str() {
        "ai.provider" => settings.ai.provider = parse_provider(val)?,
        "ai.timeout_secs" => {
            settings.ai.timeout_secs = val.trim().parse().with_context(|| {
                format!("invalid u64 for ai.timeout_secs: {val:?}")
            })?;
        }
        "ai.temperature" => {
            settings.ai.temperature = val.trim().parse().with_context(|| {
                format!("invalid float for ai.temperature: {val:?}")
            })?;
        }
        "ai.max_tokens" => {
            settings.ai.max_tokens = val.trim().parse().with_context(|| {
                format!("invalid u32 for ai.max_tokens: {val:?}")
            })?;
        }
        "ai.max_retries" => {
            settings.ai.max_retries = val.trim().parse().with_context(|| {
                format!("invalid u32 for ai.max_retries: {val:?}")
            })?;
        }
        "ai.openai.model" => settings.ai.openai.model = val.to_string(),
        "ai.openai.api_key" => settings.ai.openai.api_key = val.to_string(),
        "ai.openai.api_key_env" => settings.ai.openai.api_key_env = val.to_string(),
        "ai.openai.base_url" => settings.ai.openai.base_url = val.to_string(),
        "ai.anthropic.model" => settings.ai.anthropic.model = val.to_string(),
        "ai.anthropic.api_key" => settings.ai.anthropic.api_key = val.to_string(),
        "ai.anthropic.api_key_env" => settings.ai.anthropic.api_key_env = val.to_string(),
        "ai.gemini.model" => settings.ai.gemini.model = val.to_string(),
        "ai.gemini.api_key" => settings.ai.gemini.api_key = val.to_string(),
        "ai.gemini.api_key_env" => settings.ai.gemini.api_key_env = val.to_string(),
        "ai.deepseek.model" => settings.ai.deepseek.model = val.to_string(),
        "ai.deepseek.api_key" => settings.ai.deepseek.api_key = val.to_string(),
        "ai.deepseek.api_key_env" => settings.ai.deepseek.api_key_env = val.to_string(),
        "ai.deepseek.base_url" => settings.ai.deepseek.base_url = val.to_string(),
        "ai.ollama.model" => settings.ai.ollama.model = val.to_string(),
        "ai.ollama.host" => settings.ai.ollama.host = val.to_string(),
        "behavior.auto_confirm" => settings.behavior.auto_confirm = parse_bool(val)?,
        "behavior.learn_by_default" => settings.behavior.learn_by_default = parse_bool(val)?,
        "behavior.shell" => settings.behavior.shell = val.to_string(),
        "behavior.history_path" => settings.behavior.history_path = val.to_string(),
        "ui.color" => settings.ui.color = parse_bool(val)?,
        "ui.verbose" => settings.ui.verbose = parse_bool(val)?,
        "ui.tui_debounce_ms" => {
            settings.ui.tui_debounce_ms = val.trim().parse().with_context(|| {
                format!("invalid u64 for ui.tui_debounce_ms: {val:?}")
            })?;
        }
        _ => bail!("unknown key — run `idoit config keys` for recognized dot-paths"),
    }
    Ok(())
}

const RECOGNIZED_KEYS: &str = r"
Recognized keys (dot paths for get/set):

  ai.provider
  ai.timeout_secs
  ai.temperature
  ai.max_tokens
  ai.max_retries
  ai.openai.model
  ai.openai.api_key
  ai.openai.api_key_env
  ai.openai.base_url
  ai.anthropic.model
  ai.anthropic.api_key
  ai.anthropic.api_key_env
  ai.gemini.model
  ai.gemini.api_key
  ai.gemini.api_key_env
  ai.deepseek.model
  ai.deepseek.api_key
  ai.deepseek.api_key_env
  ai.deepseek.base_url
  ai.ollama.model
  ai.ollama.host
  behavior.auto_confirm
  behavior.learn_by_default
  behavior.shell
  behavior.history_path
  ui.color
  ui.verbose
  ui.tui_debounce_ms
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_roundtrip() {
        let mut s = Settings::default();
        apply_set(&mut s, "ai.provider", "deepseek").unwrap();
        assert_eq!(get_key(&s, "ai.provider").unwrap(), "deepseek");
        apply_set(&mut s, "ui.tui_debounce_ms", "500").unwrap();
        assert_eq!(get_key(&s, "ui.tui_debounce_ms").unwrap(), "500");
    }

    #[test]
    fn unknown_key_errors() {
        let mut s = Settings::default();
        assert!(apply_set(&mut s, "ai.unknown", "x").is_err());
        assert!(get_key(&s, "nope").is_err());
    }
}
