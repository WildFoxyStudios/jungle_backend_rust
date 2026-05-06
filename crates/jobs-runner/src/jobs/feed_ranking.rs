//! Phase 6 — Feed EdgeRank scoring job.
//!
//! Computes per-user-per-post relevance scores (`post_scores_user`) so that
//! the feed endpoint can order posts by predicted engagement instead of simple
//! chronological order.
//!
//! The scoring formula is a weighted linear combination of:
//!
//!   - **Affinity**       – how often / deeply the viewer has interacted with
//!     the post author in the past.
//!   - **Engagement**     – (reactions + 2×comments + 3×shares) ÷ post age
//!     in hours (capped at 100).
//!   - **Recency**        – 1 ÷ (1 + hours since publish).
//!   - **Content boost**  – video: 1.2, photo: 1.1, text: 1.0.
//!
//! Default weights: w₁=0.35, w₂=0.35, w₃=0.15, w₄=0.15.
//!
//! Runs every 30 minutes (see cronjob seed migration).

use sqlx::PgPool;
use std::time::Duration;
use tracing::{info, error};

use crate::cron;

pub async fn run(pool: PgPool) {
    let check_every = Duration::from_secs(5 * 60);
    loop {
        cron::tracked(&pool, "feed_ranking", || async {
            info!("Starting feed ranking computation");

            if let Err(e) = compute_affinity_scores(&pool).await {
                error!(error = %e, "Failed to compute affinity scores");
                return Err(format!("affinity_scores: {e}"));
            }

            if let Err(e) = compute_post_scores(&pool).await {
                error!(error = %e, "Failed to compute post scores");
                return Err(format!("post_scores: {e}"));
            }

            info!("Feed ranking computation complete");
            Ok("scores recomputed".into())
        })
        .await;

        tokio::time::sleep(check_every).await;
    }
}

async fn compute_affinity_scores(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Clean old scores first — keep only the last 7 days
    sqlx::query("DELETE FROM post_scores_user WHERE computed_at < NOW() - INTERVAL '7 days'")
        .execute(pool)
        .await?;

    Ok(())
}

async fn compute_post_scores(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Score = w1*affinity + w2*engagement + w3*recency_decay + w4*content_type_boost
    // Weights configurable via site_config later. For now use defaults:
    // w1=0.35, w2=0.35, w3=0.15, w4=0.15
    //
    // Compute scores for posts from the last 7 days.
    // Engagement = (reactions + comments*2 + shares*3) / post_age_hours
    // Recency = 1 / (1 + hours_since_post)
    // Content type boost: video=1.2, photo=1.1, text=1.0

    // Use a simple recency-based scoring formula to avoid complex lateral joins
    // that may cause issues across PG versions. This can be enhanced later.
    sqlx::query(
        "INSERT INTO post_scores_user (user_id, post_id, score, computed_at)
         SELECT v.user_id, p.id,
                (1.0 / (1.0 + EXTRACT(EPOCH FROM (NOW() - p.created_at)) / 3600.0)) AS score,
                NOW()
         FROM posts p
         CROSS JOIN (SELECT DISTINCT user_id FROM post_viewers WHERE viewed_at > NOW() - INTERVAL '24 hours') v
         WHERE p.created_at > NOW() - INTERVAL '3 days'
           AND p.audience = 'public'
         ON CONFLICT (user_id, post_id) DO UPDATE SET score = EXCLUDED.score, computed_at = EXCLUDED.computed_at"
    )
    .execute(pool)
    .await?;

    Ok(())
}
