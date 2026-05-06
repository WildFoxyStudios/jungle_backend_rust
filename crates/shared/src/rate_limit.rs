use redis::AsyncCommands;
use crate::errors::ApiError;

/// Simple Redis-backed rate limiter. Tracks request counts per key in a
/// fixed window using INCR + EXPIRE.
pub struct RateLimiter;

impl RateLimiter {
    /// Check rate limit. Returns `Ok(())` if the request is allowed, or
    /// `Err(ApiError::RateLimited)` if the limit has been exceeded.
    ///
    /// `max_requests`: max allowed in the window (e.g., 5)
    /// `window_secs`: window duration in seconds (e.g., 60)
    pub async fn check<C>(
        conn: &mut C,
        key: &str,
        max_requests: u32,
        window_secs: u32,
    ) -> Result<(), ApiError>
    where
        C: AsyncCommands + Send + Sync,
    {
        let count: u32 = conn
            .incr(key, 1u32)
            .await
            .unwrap_or(0);

        // Set expiry on first request in the window
        if count == 1 {
            let _: Result<(), _> = conn.expire(key, window_secs as i64).await;
        }

        if count > max_requests {
            return Err(ApiError::RateLimited);
        }

        Ok(())
    }
}
