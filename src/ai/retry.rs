use std::future::Future;
use std::time::Duration;

use anyhow::Result;
use rand::Rng;

pub struct RetryConfig {
    pub max_retries: u32,
    pub on_retry: Option<Box<dyn Fn(u32, Duration) + Send + Sync>>,
}

pub async fn with_retry<F, Fut, T>(config: &RetryConfig, mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 0;
    loop {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                attempt += 1;
                if attempt > config.max_retries || !is_retryable(&e) {
                    return Err(e);
                }

                let delay = backoff_duration(attempt);
                if let Some(ref cb) = config.on_retry {
                    cb(attempt, delay);
                }
                tokio::time::sleep(delay).await;
            }
        }
    }
}

fn is_retryable(err: &anyhow::Error) -> bool {
    let msg = format!("{err:#}");
    let msg = msg.to_lowercase();

    // HTTP status codes that are retryable
    for code in ["429", "500", "502", "503", "529"] {
        if msg.contains(&format!("returned {code}"))
            || msg.contains(&format!("status: {code}"))
            || msg.contains(&format!("{code} "))
        {
            return true;
        }
    }

    // Connection / network errors
    if msg.contains("connection")
        || msg.contains("timed out")
        || msg.contains("timeout")
        || msg.contains("temporarily")
        || msg.contains("overloaded")
        || msg.contains("unavailable")
        || msg.contains("high demand")
        || msg.contains("dns")
        || msg.contains("reset by peer")
    {
        return true;
    }

    false
}

fn backoff_duration(attempt: u32) -> Duration {
    let base_ms = 1000u64 * (1 << (attempt - 1).min(4));
    let jitter_ms = rand::rng().random_range(0..=500);
    Duration::from_millis(base_ms + jitter_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retryable_503() {
        let err = anyhow::anyhow!("Gemini API returned 503 Service Unavailable: high demand");
        assert!(is_retryable(&err));
    }

    #[test]
    fn test_retryable_429() {
        let err = anyhow::anyhow!("AI provider returned 429 Too Many Requests");
        assert!(is_retryable(&err));
    }

    #[test]
    fn test_retryable_connection() {
        let err = anyhow::anyhow!("failed to reach AI provider: connection refused");
        assert!(is_retryable(&err));
    }

    #[test]
    fn test_not_retryable_400() {
        let err = anyhow::anyhow!("AI provider returned 400 Bad Request");
        assert!(!is_retryable(&err));
    }

    #[test]
    fn test_not_retryable_401() {
        let err = anyhow::anyhow!("AI provider returned 401 Unauthorized");
        assert!(!is_retryable(&err));
    }

    #[test]
    fn test_backoff_increases() {
        let d1 = backoff_duration(1);
        let d2 = backoff_duration(2);
        // d2 base (2s) should generally be larger than d1 base (1s) even with jitter
        assert!(d2.as_millis() > 1000);
        assert!(d1.as_millis() >= 1000);
    }
}
