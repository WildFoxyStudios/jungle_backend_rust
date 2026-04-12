use axum::{extract::State, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{
    auth::{encode_access_token, AppState, AuthUser},
    errors::ApiError,
};

#[derive(Debug, Deserialize)]
pub struct SwitchAccountRequest {
    pub target_user_id: i64,
}

/// POST /v1/auth/switch-account
/// Switch to another account the user manages (e.g., page admin accounts).
/// The user must be an admin or the target must be a linked sub-account.
pub async fn switch_account(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SwitchAccountRequest>,
) -> Result<Json<Value>, ApiError> {
    // Verify the user has permission to switch to target account.
    // Allowed if: (a) current user is admin, or (b) target account is
    // in the same "account family" (linked via parent_user_id).
    let target = sqlx::query_as::<_, (i64, uuid::Uuid, bool)>(
        r#"SELECT id, uuid, is_admin FROM users WHERE id = $1 AND is_active = TRUE"#,
    )
    .bind(req.target_user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Target account not found".into()))?;

    let (target_id, target_uuid, target_is_admin) = target;

    // Permission check: admin can switch to anyone.
    // Regular users can only switch to accounts they administer (e.g., page admin).
    if !auth.is_admin {
        // Check if the current user is an admin of a page owned by the target,
        // or if target is the same user (no-op but harmless).
        let is_page_admin = sqlx::query_scalar::<_, bool>(
            r#"SELECT EXISTS(
                SELECT 1 FROM page_admins pa
                JOIN pages p ON p.id = pa.page_id
                WHERE pa.user_id = $1 AND p.user_id = $2
            )"#,
        )
        .bind(auth.user_id)
        .bind(target_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(false);

        if !is_page_admin && target_id != auth.user_id {
            return Err(ApiError::Forbidden("Not authorized to switch to this account".into()));
        }
    }

    let access_token = encode_access_token(
        target_id,
        target_uuid,
        target_is_admin,
        &state.config.jwt_secret,
    )?;

    // Generate a new refresh token
    let refresh_token = uuid::Uuid::new_v4().to_string();
    let refresh_hash = shared::auth::hash_token(&refresh_token);

    sqlx::query(
        r#"INSERT INTO sessions (user_id, refresh_token_hash, ip_address, user_agent, expires_at)
           VALUES ($1, $2, 'switch', 'switch-account', NOW() + INTERVAL '30 days')"#,
    )
    .bind(target_id)
    .bind(&refresh_hash)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({
        "data": {
            "access_token": access_token,
            "refresh_token": refresh_token,
            "user_id": target_id,
            "is_admin": target_is_admin,
        }
    })))
}
