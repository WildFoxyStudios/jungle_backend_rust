use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};

use crate::errors::ApiError;

/// Circuit breaker for external service calls.
///
/// Opens after `threshold` consecutive failures and stays open for
/// `timeout_secs` before transitioning to half-open (allowing one probe).
pub struct CircuitBreaker {
    failure_count: AtomicU32,
    last_failure: AtomicI64,
    threshold: u32,
    timeout_secs: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, timeout_secs: i64) -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            last_failure: AtomicI64::new(0),
            threshold,
            timeout_secs,
        }
    }

    pub fn state(&self) -> CircuitState {
        let failures = self.failure_count.load(Ordering::Relaxed);
        if failures < self.threshold {
            return CircuitState::Closed;
        }
        let last = self.last_failure.load(Ordering::Relaxed);
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        if now - last >= self.timeout_secs {
            CircuitState::HalfOpen
        } else {
            CircuitState::Open
        }
    }

    pub fn is_open(&self) -> bool {
        self.state() == CircuitState::Open
    }

    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::Relaxed);
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        self.last_failure.store(now, Ordering::Relaxed);
    }

    /// Execute a fallible async operation through the circuit breaker.
    ///
    /// Returns `ApiError::Internal` if the circuit is open.
    pub async fn call<F, T>(&self, f: F) -> Result<T, ApiError>
    where
        F: Future<Output = Result<T, ApiError>>,
    {
        if self.is_open() {
            return Err(ApiError::Internal(
                "Circuit breaker open — service unavailable".into(),
            ));
        }

        match f.await {
            Ok(val) => {
                self.record_success();
                Ok(val)
            }
            Err(e) => {
                self.record_failure();
                Err(e)
            }
        }
    }
}

/// Retry an async closure with exponential backoff.
///
/// `max_retries` — how many *retries* (total attempts = max_retries + 1).
/// Base delay starts at 100 ms and doubles each retry.
pub async fn retry_with_backoff<F, T, E, Fut>(mut f: F, max_retries: u32) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut retries = 0u32;
    loop {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) if retries < max_retries => {
                retries += 1;
                let delay = std::time::Duration::from_millis(100 * 2u64.pow(retries));
                tracing::warn!(?e, retries, "Retrying after {:?}", delay);
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}

/// Publish a NATS message with retry and Dead Letter Queue fallback.
///
/// Attempts up to 3 publishes; on total failure the message is routed to
/// `dlq.<original_subject>`.
pub async fn publish_with_retry(nats: &async_nats::Client, subject: &str, payload: &[u8]) {
    for attempt in 0u32..3 {
        if nats
            .publish(subject.to_string(), payload.to_vec().into())
            .await
            .is_ok()
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100 * 2u64.pow(attempt))).await;
    }
    let dlq_subject = format!("dlq.{}", subject);
    let _ = nats.publish(dlq_subject, payload.to_vec().into()).await;
    tracing::error!(subject, "Message sent to DLQ after 3 failures");
}

/// Generic retry wrapper that accepts a boxed future factory.
///
/// Useful when the closure cannot implement `FnMut` easily (e.g. captures
/// mutable state or needs type-erased futures).
pub async fn retry_boxed<T, E>(
    mut f: impl FnMut() -> Pin<Box<dyn Future<Output = Result<T, E>> + Send>>,
    max_retries: u32,
) -> Result<T, E>
where
    E: std::fmt::Debug,
{
    let mut retries = 0u32;
    loop {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) if retries < max_retries => {
                retries += 1;
                let delay = std::time::Duration::from_millis(100 * 2u64.pow(retries));
                tracing::warn!(?e, retries, "Retrying (boxed) after {:?}", delay);
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_starts_closed() {
        let cb = CircuitBreaker::new(3, 30);
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(!cb.is_open());
    }

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let cb = CircuitBreaker::new(3, 30);
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(cb.is_open());
    }

    #[test]
    fn test_circuit_breaker_resets_on_success() {
        let cb = CircuitBreaker::new(3, 30);
        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_retry_with_backoff_succeeds_first_try() {
        let result: Result<i32, String> = retry_with_backoff(|| async { Ok(42) }, 3).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_with_backoff_fails_then_succeeds() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let c = counter.clone();
        let result: Result<i32, String> = retry_with_backoff(
            move || {
                let c = c.clone();
                async move {
                    let n = c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if n < 2 {
                        Err("fail".to_string())
                    } else {
                        Ok(42)
                    }
                }
            },
            3,
        )
        .await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);
    }
}
