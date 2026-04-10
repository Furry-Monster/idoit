use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub ai: AiSettings,
    #[serde(default)]
    pub behavior: BehaviorSettings,
    #[serde(default)]
    pub ui: UiSettings,
    #[serde(default)]
    pub cache: CacheSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSettings {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default)]
    pub openai: OpenAiSettings,
    #[serde(default)]
    pub anthropic: AnthropicSettings,
    #[serde(default)]
    pub gemini: GeminiSettings,
    #[serde(default)]
    pub ollama: OllamaSettings,
}

/// Shared interface — every provider settings struct has at least model + api_key_env.
impl AiSettings {
    pub fn active_model(&self) -> &str {
        match self.provider.as_str() {
            "anthropic" => &self.anthropic.model,
            "gemini" => &self.gemini.model,
            "ollama" => &self.ollama.model,
            _ => &self.openai.model,
        }
    }

    #[allow(dead_code)]
    pub fn active_api_key_env(&self) -> &str {
        match self.provider.as_str() {
            "anthropic" => &self.anthropic.api_key_env,
            "gemini" => &self.gemini.api_key_env,
            _ => &self.openai.api_key_env,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiSettings {
    #[serde(default = "default_openai_model")]
    pub model: String,
    #[serde(default = "default_openai_api_key_env")]
    pub api_key_env: String,
    #[serde(default)]
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicSettings {
    #[serde(default = "default_anthropic_model")]
    pub model: String,
    #[serde(default = "default_anthropic_api_key_env")]
    pub api_key_env: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiSettings {
    #[serde(default = "default_gemini_model")]
    pub model: String,
    #[serde(default = "default_gemini_api_key_env")]
    pub api_key_env: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaSettings {
    #[serde(default = "default_ollama_model")]
    pub model: String,
    #[serde(default = "default_ollama_host")]
    pub host: String,
}

// --- defaults ---

fn default_provider() -> String { "openai".into() }
fn default_timeout() -> u64 { 30 }
fn default_temperature() -> f64 { 0.1 }
fn default_max_tokens() -> u32 { 2048 }
fn default_max_retries() -> u32 { 3 }

fn default_openai_model() -> String { "gpt-4o-mini".into() }
fn default_openai_api_key_env() -> String { "OPENAI_API_KEY".into() }
fn default_anthropic_model() -> String { "claude-sonnet-4-20250514".into() }
fn default_anthropic_api_key_env() -> String { "ANTHROPIC_API_KEY".into() }
fn default_gemini_model() -> String { "gemini-2.5-flash".into() }
fn default_gemini_api_key_env() -> String { "GEMINI_API_KEY".into() }
fn default_ollama_model() -> String { "llama3".into() }
fn default_ollama_host() -> String { "http://localhost:11434".into() }

fn default_true() -> bool { true }
fn default_cache_ttl() -> u64 { 3600 }

// --- Default impls ---

impl Default for Settings {
    fn default() -> Self {
        Self {
            ai: AiSettings::default(),
            behavior: BehaviorSettings::default(),
            ui: UiSettings::default(),
            cache: CacheSettings::default(),
        }
    }
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            timeout_secs: default_timeout(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            max_retries: default_max_retries(),
            openai: OpenAiSettings::default(),
            anthropic: AnthropicSettings::default(),
            gemini: GeminiSettings::default(),
            ollama: OllamaSettings::default(),
        }
    }
}

impl Default for OpenAiSettings {
    fn default() -> Self {
        Self {
            model: default_openai_model(),
            api_key_env: default_openai_api_key_env(),
            base_url: String::new(),
        }
    }
}

impl Default for AnthropicSettings {
    fn default() -> Self {
        Self {
            model: default_anthropic_model(),
            api_key_env: default_anthropic_api_key_env(),
        }
    }
}

impl Default for GeminiSettings {
    fn default() -> Self {
        Self {
            model: default_gemini_model(),
            api_key_env: default_gemini_api_key_env(),
        }
    }
}

impl Default for OllamaSettings {
    fn default() -> Self {
        Self {
            model: default_ollama_model(),
            host: default_ollama_host(),
        }
    }
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

impl Default for BehaviorSettings {
    fn default() -> Self {
        Self {
            auto_confirm: false,
            learn_by_default: false,
            shell: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    #[serde(default = "default_true")]
    pub color: bool,
    #[serde(default)]
    pub verbose: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self { color: true, verbose: false }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_cache_ttl")]
    pub ttl_secs: u64,
}

impl Default for CacheSettings {
    fn default() -> Self {
        Self { enabled: false, ttl_secs: default_cache_ttl() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let s = Settings::default();
        assert_eq!(s.ai.provider, "openai");
        assert_eq!(s.ai.openai.model, "gpt-4o-mini");
        assert_eq!(s.ai.anthropic.model, "claude-sonnet-4-20250514");
        assert_eq!(s.ai.gemini.model, "gemini-2.5-flash");
        assert_eq!(s.ai.ollama.model, "llama3");
        assert_eq!(s.ai.timeout_secs, 30);
        assert!((s.ai.temperature - 0.1).abs() < f64::EPSILON);
        assert_eq!(s.ai.max_tokens, 2048);
    }

    #[test]
    fn test_active_model() {
        let mut s = Settings::default();
        assert_eq!(s.ai.active_model(), "gpt-4o-mini");

        s.ai.provider = "gemini".into();
        assert_eq!(s.ai.active_model(), "gemini-2.5-flash");

        s.ai.provider = "anthropic".into();
        assert_eq!(s.ai.active_model(), "claude-sonnet-4-20250514");

        s.ai.provider = "ollama".into();
        assert_eq!(s.ai.active_model(), "llama3");
    }

    #[test]
    fn test_active_api_key_env() {
        let mut s = Settings::default();
        assert_eq!(s.ai.active_api_key_env(), "OPENAI_API_KEY");

        s.ai.provider = "gemini".into();
        assert_eq!(s.ai.active_api_key_env(), "GEMINI_API_KEY");

        s.ai.provider = "anthropic".into();
        assert_eq!(s.ai.active_api_key_env(), "ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_parse_provider_config() {
        let toml_str = r#"
[ai]
provider = "gemini"

[ai.gemini]
model = "gemini-2.5-pro"
"#;
        let s: Settings = toml::from_str(toml_str).unwrap();
        assert_eq!(s.ai.provider, "gemini");
        assert_eq!(s.ai.gemini.model, "gemini-2.5-pro");
        assert_eq!(s.ai.active_model(), "gemini-2.5-pro");
    }

    #[test]
    fn test_parse_openai_with_base_url() {
        let toml_str = r#"
[ai]
provider = "openai"
temperature = 0.3

[ai.openai]
model = "gpt-4o"
base_url = "https://my-proxy.example.com/v1"
"#;
        let s: Settings = toml::from_str(toml_str).unwrap();
        assert_eq!(s.ai.openai.model, "gpt-4o");
        assert_eq!(s.ai.openai.base_url, "https://my-proxy.example.com/v1");
        assert!((s.ai.temperature - 0.3).abs() < f64::EPSILON);
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
    fn test_parse_cache_config() {
        let toml_str = r#"
[cache]
enabled = true
ttl_secs = 7200
"#;
        let s: Settings = toml::from_str(toml_str).unwrap();
        assert!(s.cache.enabled);
        assert_eq!(s.cache.ttl_secs, 7200);
    }

    #[test]
    fn test_roundtrip_serialization() {
        let s = Settings::default();
        let toml_str = toml::to_string_pretty(&s).unwrap();
        let s2: Settings = toml::from_str(&toml_str).unwrap();
        assert_eq!(s.ai.provider, s2.ai.provider);
        assert_eq!(s.ai.openai.model, s2.ai.openai.model);
        assert_eq!(s.ai.gemini.model, s2.ai.gemini.model);
    }

    #[test]
    fn test_empty_toml() {
        let s: Settings = toml::from_str("").unwrap();
        assert_eq!(s.ai.provider, "openai");
        assert_eq!(s.ai.active_model(), "gpt-4o-mini");
    }
}
