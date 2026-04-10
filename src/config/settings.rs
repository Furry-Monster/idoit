use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub ai: AiSettings,
    #[serde(default)]
    pub behavior: BehaviorSettings,
    #[serde(default)]
    pub ui: UiSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettings {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub ollama: OllamaSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaSettings {
    #[serde(default = "default_ollama_host")]
    pub host: String,
    #[serde(default = "default_ollama_model")]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSettings {
    #[serde(default)]
    pub auto_confirm: bool,
    #[serde(default)]
    pub learn_by_default: bool,
    #[serde(default)]
    pub shell: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    #[serde(default = "default_true")]
    pub color: bool,
    #[serde(default)]
    pub verbose: bool,
}

fn default_provider() -> String {
    "openai".into()
}
fn default_model() -> String {
    "gpt-4o-mini".into()
}
fn default_api_key_env() -> String {
    "OPENAI_API_KEY".into()
}
fn default_ollama_host() -> String {
    "http://localhost:11434".into()
}
fn default_ollama_model() -> String {
    "llama3".into()
}
fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ai: AiSettings::default(),
            behavior: BehaviorSettings::default(),
            ui: UiSettings::default(),
        }
    }
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            api_key_env: default_api_key_env(),
            base_url: String::new(),
            ollama: OllamaSettings::default(),
        }
    }
}

impl Default for OllamaSettings {
    fn default() -> Self {
        Self {
            host: default_ollama_host(),
            model: default_ollama_model(),
        }
    }
}

impl Default for BehaviorSettings {
    fn default() -> Self {
        Self {
            auto_confirm: false,
            learn_by_default: false,
            shell: String::new(),
        }
    }
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            color: true,
            verbose: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let s = Settings::default();
        assert_eq!(s.ai.provider, "openai");
        assert_eq!(s.ai.model, "gpt-4o-mini");
        assert_eq!(s.ai.api_key_env, "OPENAI_API_KEY");
        assert!(!s.behavior.auto_confirm);
        assert!(s.ui.color);
    }

    #[test]
    fn test_parse_partial_toml() {
        let toml_str = r#"
[ai]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
"#;
        let s: Settings = toml::from_str(toml_str).unwrap();
        assert_eq!(s.ai.provider, "anthropic");
        assert_eq!(s.ai.model, "claude-sonnet-4-20250514");
        assert_eq!(s.ai.api_key_env, "OPENAI_API_KEY");
        assert!(s.ui.color);
    }

    #[test]
    fn test_parse_ollama_config() {
        let toml_str = r#"
[ai]
provider = "ollama"

[ai.ollama]
host = "http://myhost:11434"
model = "codellama"
"#;
        let s: Settings = toml::from_str(toml_str).unwrap();
        assert_eq!(s.ai.provider, "ollama");
        assert_eq!(s.ai.ollama.host, "http://myhost:11434");
        assert_eq!(s.ai.ollama.model, "codellama");
    }

    #[test]
    fn test_roundtrip_serialization() {
        let s = Settings::default();
        let toml_str = toml::to_string_pretty(&s).unwrap();
        let s2: Settings = toml::from_str(&toml_str).unwrap();
        assert_eq!(s.ai.provider, s2.ai.provider);
        assert_eq!(s.ai.model, s2.ai.model);
        assert_eq!(s.behavior.auto_confirm, s2.behavior.auto_confirm);
    }

    #[test]
    fn test_empty_toml() {
        let s: Settings = toml::from_str("").unwrap();
        assert_eq!(s.ai.provider, "openai");
        assert_eq!(s.ai.model, "gpt-4o-mini");
    }
}
