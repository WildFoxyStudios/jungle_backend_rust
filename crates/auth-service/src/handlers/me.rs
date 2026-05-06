use axum::{Json, extract::State};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    models::{AuthUserResponse, User},
};

pub async fn me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user =
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1 AND deleted_at IS NULL")
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or(ApiError::NotFound("User not found".into()))?;

    let resp = AuthUserResponse::from(&user);
    let mut data = serde_json::to_value(&resp).map_err(|e| ApiError::Internal(e.to_string()))?;
    if let Some(obj) = data.as_object_mut() {
        obj.insert(
            "can_access_admin".to_string(),
            serde_json::json!(auth.can_access_admin_panel()),
        );
    }

    Ok(Json(serde_json::json!({ "data": data })))
}
