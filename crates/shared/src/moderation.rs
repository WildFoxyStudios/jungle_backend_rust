use sqlx::{PgPool, Row};

/// Enqueue content for AI moderation review.
/// Called by post-service, media-service, commerce-service, etc.
/// after a new content item is created or updated.
pub async fn enqueue_moderation(
    db: &PgPool,
    target_type: &str,
    target_id: i64,
    submitted_by_user_id: i64,
    content_text: &str,
) -> Result<i64, crate::errors::ApiError> {
    let row = sqlx::query(
        "INSERT INTO moderation_queue (target_type, target_id, submitted_by_user_id, content_text, status, created_at)
         VALUES ($1, $2, $3, $4, 'pending', NOW())
         ON CONFLICT (target_type, target_id)
         DO UPDATE SET content_text = EXCLUDED.content_text, status = 'pending'
         RETURNING id",
    )
    .bind(target_type)
    .bind(target_id)
    .bind(submitted_by_user_id)
    .bind(content_text)
    .fetch_one(db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, target_type, target_id, "Failed to enqueue moderation");
        crate::errors::ApiError::Internal("Moderation enqueue failed".into())
    })?;

    let id: i64 = row.get("id");
    tracing::info!(id, target_type, target_id, "Content enqueued for moderation");
    Ok(id)
}

/// Decide moderation action based on OpenAI scores and configurable thresholds.
pub fn decide_action(flagged: bool, max_score: f64) -> &'static str {
    if !flagged {
        return "auto_approve";
    }
    if max_score > 0.85 {
        "auto_block"
    } else if max_score > 0.5 {
        "human_review"
    } else {
        "auto_approve"
    }
}

/// Apply moderation action to content (hide flagged posts, etc.)
pub async fn apply_moderation_action(
    db: &PgPool,
    target_type: &str,
    target_id: i64,
    action: &str,
) -> Result<(), crate::errors::ApiError> {
    match action {
        "auto_block" | "auto_approve" => {
            // Mark the moderation item as resolved
            sqlx::query(
                "UPDATE moderation_queue SET status = $1, auto_action = $2, resolved_at = NOW() WHERE target_type = $3 AND target_id = $4",
            )
            .bind(action)
            .bind(action)
            .bind(target_type)
            .bind(target_id)
            .execute(db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to resolve moderation item");
                crate::errors::ApiError::Internal("Moderation resolution failed".into())
            })?;

            tracing::info!(target_type, target_id, action, "Moderation action applied");
        }
        _ => {
            // human_review — stays in queue, no auto-action
            sqlx::query(
                "UPDATE moderation_queue SET auto_action = $1 WHERE target_type = $2 AND target_id = $3",
            )
            .bind(action)
            .bind(target_type)
            .bind(target_id)
            .execute(db)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to set moderation action");
                crate::errors::ApiError::Internal("Moderation update failed".into())
            })?;
        }
    }

    Ok(())
}
