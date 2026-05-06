use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    events::DomainEvent,
};

#[derive(Debug, Deserialize)]
pub struct ReactRequest {
    pub reaction_type: Option<String>,
}

pub async fn react_to_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<i64>,
    Json(req): Json<ReactRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let reaction_type = req.reaction_type.as_deref().unwrap_or("like");
    let valid = ["like", "love", "haha", "wow", "sad", "angry"];
    if !valid.contains(&reaction_type) {
        return Err(ApiError::BadRequest("Invalid reaction type".into()));
    }

    sqlx::query(
        r#"INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
           VALUES ($1, 'post', $2, $3)
           ON CONFLICT (user_id, target_type, target_id)
           DO UPDATE SET reaction_type = $3"#,
    )
    .bind(auth.user_id)
    .bind(post_id)
    .bind(reaction_type)
    .execute(&state.db)
    .await?;

    // Update denormalized count
    sqlx::query(
        "UPDATE posts SET like_count = (SELECT COUNT(*) FROM reactions WHERE target_type = 'post' AND target_id = $1) WHERE id = $1",
    )
    .bind(post_id)
    .execute(&state.db)
    .await?;

    // Fetch author for event
    let author_id: Option<i64> = sqlx::query_scalar("SELECT user_id FROM posts WHERE id = $1")
        .bind(post_id)
        .fetch_optional(&state.db)
        .await?
        .flatten();

    if let Some(aid) = author_id {
        let _ = state
            .event_bus
            .publish(&DomainEvent::PostLiked {
                post_id,
                user_id: auth.user_id,
                author_id: aid,
                reaction_type: reaction_type.to_string(),
            })
            .await;
    }

    // Granular signal for live UI updates (badge counters, animations).
    let _ = state
        .event_bus
        .publish(&DomainEvent::ReactionRegistered {
            post_id,
            user_id: auth.user_id,
            reaction: reaction_type.to_string(),
        })
        .await;

    Ok(Json(serde_json::json!({
        "data": { "reaction_type": reaction_type, "post_id": post_id }
    })))
}

pub async fn unreact_to_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(post_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query(
        "DELETE FROM reactions WHERE user_id = $1 AND target_type = 'post' AND target_id = $2",
    )
    .bind(auth.user_id)
    .bind(post_id)
    .execute(&state.db)
    .await?;

    sqlx::query(
        "UPDATE posts SET like_count = (SELECT COUNT(*) FROM reactions WHERE target_type = 'post' AND target_id = $1) WHERE id = $1",
    )
    .bind(post_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "data": { "message": "Reaction removed" }
    })))
}

pub async fn react_to_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(comment_id): Path<i64>,
    Json(req): Json<ReactRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let reaction_type = req.reaction_type.as_deref().unwrap_or("like");

    sqlx::query(
        r#"INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
           VALUES ($1, 'comment', $2, $3)
           ON CONFLICT (user_id, target_type, target_id)
           DO UPDATE SET reaction_type = $3"#,
    )
    .bind(auth.user_id)
    .bind(comment_id)
    .bind(reaction_type)
    .execute(&state.db)
    .await?;

    sqlx::query(
        "UPDATE comments SET like_count = (SELECT COUNT(*) FROM reactions WHERE target_type = 'comment' AND target_id = $1) WHERE id = $1",
    )
    .bind(comment_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "data": { "reaction_type": reaction_type, "comment_id": comment_id }
    })))
}

pub async fn unreact_to_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(comment_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query(
        "DELETE FROM reactions WHERE user_id = $1 AND target_type = 'comment' AND target_id = $2",
    )
    .bind(auth.user_id)
    .bind(comment_id)
    .execute(&state.db)
    .await?;

    sqlx::query(
        "UPDATE comments SET like_count = (SELECT COUNT(*) FROM reactions WHERE target_type = 'comment' AND target_id = $1) WHERE id = $1",
    )
    .bind(comment_id)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({
        "data": { "message": "Reaction removed" }
    })))
}
