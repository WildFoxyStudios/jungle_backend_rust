use redis::AsyncCommands;

/// Token-bucket rate limiter backed by Redis.
pub struct RateLimiter {
    redis: redis::aio::ConnectionManager,
}

impl RateLimiter {
    pub fn new(redis: redis::aio::ConnectionManager) -> Self {
        Self { redis }
    }

    /// Check if a request is allowed. Returns Ok(remaining) or Err(retry_after_secs).
    pub async fn check(
        &self,
        key: &str,
        max_requests: u64,
        window_secs: u64,
    ) -> Result<u64, u64> {
        let mut conn = self.redis.clone();

        let current: u64 = conn.incr(key, 1u64).await.unwrap_or(1);

        if current == 1 {
            let _: Result<(), _> = conn.expire(key, window_secs as i64).await;
        }

        if current > max_requests {
            let ttl: i64 = conn.ttl(key).await.unwrap_or(window_secs as i64);
            Err(ttl as u64)
        } else {
            Ok(max_requests - current)
        }
    }

    /// Get rate limit config based on path prefix
    pub fn config_for_path(path: &str) -> (u64, u64) {
        if path.starts_with("/v1/auth/login") || path.starts_with("/v1/auth/register") {
            (10, 900) // 10 per 15 min
        } else if path.starts_with("/v1/auth/refresh") {
            (30, 60) // 30 per min
        } else if path.starts_with("/v1/auth") {
            (15, 900) // 15 per 15 min
        } else if path.starts_with("/v1/media/upload") {
            (20, 60) // 20 per min
        } else if path.contains("/search") {
            (30, 60) // 30 per min
        } else if path.starts_with("/v1/messages") || path.starts_with("/v1/conversations") {
            (60, 60) // 60 per min
        } else {
            (100, 60) // 100 per min default
        }
    }
}
