use anyhow::Result;

use super::types::{CompletionRequest, CompletionResponse};

pub trait AiProvider: Send + Sync {
    fn name(&self) -> &str;

    fn complete(
        &self,
        request: &CompletionRequest,
    ) -> impl std::future::Future<Output = Result<CompletionResponse>> + Send;

    /// Whether this provider supports streaming. Override to enable.
    #[allow(dead_code)]
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Stream tokens, calling `on_token` for each text fragment received.
    #[allow(dead_code)]
    /// Default implementation falls back to non-streaming `complete`.
    fn stream_complete(
        &self,
        request: &CompletionRequest,
        on_token: &(dyn Fn(&str) + Send + Sync),
    ) -> impl std::future::Future<Output = Result<CompletionResponse>> + Send {
        async move {
            let resp = self.complete(request).await?;
            on_token(&resp.content);
            Ok(resp)
        }
    }
}
