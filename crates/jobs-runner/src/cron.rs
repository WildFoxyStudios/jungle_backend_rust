//! Helpers that wrap a single iteration of a recurring job and persist the
//! outcome into the `cronjob_runs` table (consumed by
//! `GET /v1/admin/cronjobs/status` and the admin UI's Scheduled jobs page).
//!
//! Usage from a `jobs/<name>.rs` module:
//!
//! ```ignore
//! loop {
//!     cron::tracked(&pool, "story_cleanup", || async {
//!         let r = sqlx::query("DELETE FROM stories WHERE expires_at < NOW()")
//!             .execute(&pool).await.map_err(|e| e.to_string())?;
//!         Ok(format!("deleted {}", r.rows_affected()))
//!     }).await;
//!     tokio::time::sleep(interval).await;
//! }
//! ```
//!
//! The wrapper:
//! - measures wall-clock duration
//! - converts `Ok(_)` to status `healthy`, `Err(_)` to `error`
//! - logs the failure via `tracing::error!`
//! - inserts one row in `cronjob_runs` (best-effort; never panics)
//!
//! Job names must match the spawn names in `main.rs` so the admin UI catalog
//! lines up with the latest_run lookup keyed on `name`.

use sqlx::PgPool;
use std::future::Future;
use std::time::Instant;

const STATUS_HEALTHY: &str = "healthy";
const STATUS_ERROR: &str = "error";
const STATUS_SKIPPED: &str = "warning";

/// Persist a single execution record. Failure to write is logged but never
/// propagated — observability must not break the job itself.
pub async fn record(
    pool: &PgPool,
    name: &str,
    status: &str,
    message: Option<String>,
    duration_ms: i32,
) {
    let res = sqlx::query(
        r#"INSERT INTO cronjob_runs (name, status, message, duration_ms, ran_at)
           VALUES ($1, $2, $3, $4, NOW())"#,
    )
    .bind(name)
    .bind(status)
    .bind(message.as_deref())
    .bind(duration_ms)
    .execute(pool)
    .await;

    if let Err(e) = res {
        tracing::warn!(job = name, error = %e, "cronjob_runs insert failed");
    }
}

/// Run `body` once, time it, and persist the outcome.
///
/// `body` returns `Result<String, String>` — on `Ok` the string is stored as
/// the run message (e.g. "deleted 42"); on `Err` the string is the error
/// description and the row is marked `error`.
pub async fn tracked<F, Fut>(pool: &PgPool, name: &'static str, body: F)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<String, String>>,
{
    let start = Instant::now();
    let (status, message) = match body().await {
        Ok(msg) => (STATUS_HEALTHY, Some(msg)),
        Err(e) => {
            tracing::error!(job = name, error = %e, "cron job failed");
            (STATUS_ERROR, Some(e))
        }
    };
    let duration_ms = i32::try_from(start.elapsed().as_millis()).unwrap_or(i32::MAX);
    record(pool, name, status, message, duration_ms).await;
}

/// Record a skipped iteration (e.g. retention disabled, condition not met).
/// Stored as `status = 'warning'` with a short reason so admins can see the
/// job is alive but intentionally idle.
pub async fn skipped(pool: &PgPool, name: &'static str, reason: &str) {
    record(pool, name, STATUS_SKIPPED, Some(reason.to_string()), 0).await;
}
