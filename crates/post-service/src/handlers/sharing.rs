use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

#[derive(Debug, Deserialize)]
pub struct SharePostRequest {
    pub content: Option<String>,
    pub privacy: Option<String>,
    pub page_id: Option<i64>,
    pub group_id: Option<i64>,
}

pub async fn share_post(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<SharePostRequest>,
) -> Result<Json<Value>, ApiError> {
    // Verify original post exists and is public/friends
    let original = sqlx::query_as::<_, (i64, String)>(
        "SELECT user_id, privacy FROM posts WHERE id = $1 AND deleted_at IS NULL AND is_approved = TRUE",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Post not found".into()))?;

    if original.1 == "only_me" && original.0 != auth.user_id {
        return Err(ApiError::Forbidden("".into()));
    }

    let content = req.content.unwrap_or_default();
    let privacy = req.privacy.as_deref().unwrap_or("everyone");

    // Create a new post with parent_id pointing to the original
    // If page_id or group_id is provided, share onto that page/group's timeline
    let new_id = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO posts (user_id, parent_id, content, post_type, privacy, page_id, group_id)
        VALUES ($1, $2, $3, 'share', $4, $5, $6) RETURNING id"#,
    )
    .bind(auth.user_id)
    .bind(id)
    .bind(&content)
    .bind(privacy)
    .bind(req.page_id)
    .bind(req.group_id)
    .fetch_one(&state.db)
    .await?;

    // Increment share count on original
    sqlx::query("UPDATE posts SET share_count = share_count + 1 WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "id": new_id, "shared_post_id": id } })))
}

pub async fn ad_click(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(ad_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    // Record the click
    sqlx::query(
        "INSERT INTO ad_clicks (ad_id, user_id) VALUES ($1, $2)",
    )
    .bind(ad_id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    // Increment click count on the ad
    sqlx::query(
        "UPDATE user_ads SET clicks = clicks + 1, budget = GREATEST(budget - 0.01, 0) WHERE id = $1",
    )
    .bind(ad_id)
    .execute(&state.db)
    .await?;

    // Get the target id (post/page being advertised)
    let target_id = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT target_id FROM user_ads WHERE id = $1",
    )
    .bind(ad_id)
    .fetch_optional(&state.db)
    .await?
    .flatten();

    Ok(Json(json!({ "data": { "clicked": true, "target_id": target_id } })))
}
