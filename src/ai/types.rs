use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub system: String,
    pub user_message: String,
    pub model: String,
    pub temperature: f64,
    pub max_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiCommandResponse {
    pub command: String,
    pub explanation: String,
    #[serde(default)]
    pub missing_tools: Vec<String>,
    #[serde(default)]
    pub confidence: f32,
    #[serde(default)]
    pub teaching: Option<String>,
    /// Additional command suggestions (TUI / advanced UX)
    #[serde(default)]
    pub alternates: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::AiCommandResponse;

    #[test]
    fn ai_command_response_defaults_from_minimal_json() {
        let j = r#"{"command":"ls","explanation":"list files"}"#;
        let r: AiCommandResponse = serde_json::from_str(j).unwrap();
        assert_eq!(r.command, "ls");
        assert_eq!(r.explanation, "list files");
        assert!(r.missing_tools.is_empty());
        assert_eq!(r.confidence, 0.0);
        assert!(r.teaching.is_none());
        assert!(r.alternates.is_empty());
    }
}
