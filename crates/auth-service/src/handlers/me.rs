use axum::{extract::State, Json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    models::{AuthUserResponse, User},
};

pub async fn me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("User not found".into()))?;

    let resp = AuthUserResponse::from(&user);

    Ok(Json(serde_json::json!({ "data": resp })))
}
