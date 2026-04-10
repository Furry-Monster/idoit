use anyhow::Result;

use super::types::{CompletionRequest, CompletionResponse};

#[allow(dead_code)]
pub trait AiProvider: Send + Sync {
    fn name(&self) -> &str;
    fn complete(
        &self,
        request: &CompletionRequest,
    ) -> impl std::future::Future<Output = Result<CompletionResponse>> + Send;
}
