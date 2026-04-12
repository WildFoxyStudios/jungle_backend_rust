use axum::{
    extract::State,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

// ─── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub notification_settings: Value,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

pub async fn get_preferences(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let settings = sqlx::query_scalar::<_, Value>(
        "SELECT COALESCE(notification_settings, '{}'::jsonb) FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": settings })))
}

pub async fn update_preferences(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdatePreferencesRequest>,
) -> Result<Json<Value>, ApiError> {
    // Validate that the settings object only contains known keys
    if let Some(obj) = req.notification_settings.as_object() {
        let allowed_keys = [
            "email_on_follow",
            "email_on_comment",
            "email_on_like",
            "email_on_mention",
            "email_on_message",
            "push_on_follow",
            "push_on_comment",
            "push_on_like",
            "push_on_mention",
            "push_on_message",
            "push_on_group_invite",
            "push_on_page_invite",
            "push_on_event_invite",
            "push_on_birthday",
            "push_on_memory",
            "push_on_live",
            "push_on_story_reply",
            "push_on_funding",
            "push_on_order",
        ];

        for key in obj.keys() {
            if !allowed_keys.contains(&key.as_str()) {
                return Err(ApiError::BadRequest(format!("Unknown preference key: {}", key)));
            }
        }

        // Validate all values are booleans
        for (key, val) in obj {
            if !val.is_boolean() {
                return Err(ApiError::BadRequest(format!(
                    "Preference '{}' must be a boolean",
                    key
                )));
            }
        }
    } else {
        return Err(ApiError::BadRequest("notification_settings must be a JSON object".into()));
    }

    // Merge with existing settings (JSONB || operator)
    sqlx::query(
        r#"
        UPDATE users
        SET notification_settings = COALESCE(notification_settings, '{}'::jsonb) || $1::jsonb,
            updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(&req.notification_settings)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    let updated = sqlx::query_scalar::<_, Value>(
        "SELECT COALESCE(notification_settings, '{}'::jsonb) FROM users WHERE id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": updated })))
}
