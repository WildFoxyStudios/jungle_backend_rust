use sqlx::PgPool;
use std::time::Duration;
use tracing::{info, error};

use crate::cron;

pub async fn run(pool: PgPool) {
    let check_every = Duration::from_secs(15 * 60);
    loop {
        let now = time::OffsetDateTime::now_utc();
        // Run daily at 03:00 UTC
        if now.hour() == 3 && now.minute() < 15 {
            cron::tracked(&pool, "pagerank", || async {
                compute_all(&pool).await.map_err(|e| e.to_string())?;
                Ok("recommendations computed".into())
            })
            .await;
            // Sleep an hour to avoid re-triggering within the same window
            tokio::time::sleep(Duration::from_secs(3600)).await;
            continue;
        }
        tokio::time::sleep(check_every).await;
    }
}

async fn compute_all(pool: &PgPool) -> Result<(), sqlx::Error> {
    info!("Starting PageRank recommendation computation");

    // 1. Compute PYMK (People You May Known -- friends of friends
    if let Err(e) = compute_pymk(pool).await {
        error!(error = %e, "Failed to compute PYMK recommendations");
    }

    // 2. Compute Page recommendations
    if let Err(e) = compute_page_recs(pool).await {
        error!(error = %e, "Failed to compute page recommendations");
    }

    // 3. Compute Group recommendations
    if let Err(e) = compute_group_recs(pool).await {
        error!(error = %e, "Failed to compute group recommendations");
    }

    info!("PageRank recommendation computation complete");
    Ok(())
}

async fn compute_pymk(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO recommendation_snapshots (user_id, kind, target_id, score, reason, generated_at)
           SELECT
               f1.user_id,
               'pymk',
               f3.friend_id AS target_id,
               COUNT(DISTINCT f2.friend_id)::REAL AS score,
               'mutual_friends',
               NOW()
           FROM friends f1
           JOIN friends f2 ON f2.user_id = f1.friend_id
           JOIN friends f3 ON f3.user_id = f2.friend_id AND f3.friend_id != f1.user_id
           WHERE f1.status = 'accepted'
             AND f2.status = 'accepted'
             AND f3.status = 'accepted'
             AND f3.friend_id NOT IN (
                 SELECT friend_id FROM friends WHERE user_id = f1.user_id
             )
             AND f3.friend_id NOT IN (
                 SELECT dismissed.target_id FROM recommendation_snapshots dismissed
                 WHERE dismissed.user_id = f1.user_id AND dismissed.kind = 'pymk' AND dismissed.dismissed = true
             )
           GROUP BY f1.user_id, f3.friend_id
           HAVING COUNT(DISTINCT f2.friend_id) >= 2
           ON CONFLICT (user_id, kind, target_id)
           DO UPDATE SET score = EXCLUDED.score, reason = EXCLUDED.reason, generated_at = EXCLUDED.generated_at, dismissed = FALSE"#
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn compute_page_recs(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO recommendation_snapshots (user_id, kind, target_id, score, reason, generated_at)
           SELECT
               f.user_id,
               'pages',
               pl.page_id AS target_id,
               COUNT(DISTINCT f.friend_id)::REAL AS score,
               'friends_liked',
               NOW()
           FROM friends f
           JOIN page_likes pl ON pl.user_id = f.friend_id
           WHERE f.status = 'accepted'
             AND pl.page_id NOT IN (
                 SELECT page_id FROM page_likes WHERE user_id = f.user_id
             )
           GROUP BY f.user_id, pl.page_id
           HAVING COUNT(DISTINCT f.friend_id) >= 1
           ON CONFLICT (user_id, kind, target_id)
           DO UPDATE SET score = EXCLUDED.score, reason = EXCLUDED.reason, generated_at = EXCLUDED.generated_at, dismissed = FALSE"#
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn compute_group_recs(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO recommendation_snapshots (user_id, kind, target_id, score, reason, generated_at)
           SELECT
               f.user_id,
               'groups',
               gm.group_id AS target_id,
               COUNT(DISTINCT f.friend_id)::REAL AS score,
               'friends_joined',
               NOW()
           FROM friends f
           JOIN group_members gm ON gm.user_id = f.friend_id
           WHERE f.status = 'accepted'
             AND gm.group_id NOT IN (
                 SELECT group_id FROM group_members WHERE user_id = f.user_id
             )
           GROUP BY f.user_id, gm.group_id
           HAVING COUNT(DISTINCT f.friend_id) >= 1
           ON CONFLICT (user_id, kind, target_id)
           DO UPDATE SET score = EXCLUDED.score, reason = EXCLUDED.reason, generated_at = EXCLUDED.generated_at, dismissed = FALSE"#
    )
    .execute(pool)
    .await?;
    Ok(())
}
