use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub system: String,
    pub user_message: String,
    pub model: String,
    pub temperature: f32,
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
}
