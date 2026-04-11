use anyhow::Result;
use tokio_util::sync::CancellationToken;

use super::types::{CompletionRequest, CompletionResponse};

pub trait AiProvider: Send + Sync {
    fn name(&self) -> &str;

    fn complete(
        &self,
        request: &CompletionRequest,
        cancel: Option<&CancellationToken>,
    ) -> impl std::future::Future<Output = Result<CompletionResponse>> + Send;
}
