use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};
use sqlx::Row;

#[derive(Debug, Serialize)]
pub struct FriendListResponse {
    pub id: i64,
    pub name: String,
    pub list_type: String,
    pub member_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateListRequest {
    pub name: String,
    #[serde(default = "default_list_type")]
    pub list_type: String,
}

fn default_list_type() -> String {
    "custom".to_string()
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub friend_id: i64,
}

/// GET /v1/users/me/friend-lists
pub async fn list_friend_lists(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = sqlx::query(
        r#"SELECT fl.id, fl.name, fl.list_type, COUNT(flm.friend_id)::BIGINT AS member_count
           FROM friend_lists fl
           LEFT JOIN friend_list_members flm ON flm.list_id = fl.id
           WHERE fl.user_id = $1
           GROUP BY fl.id
           ORDER BY fl.list_type = 'close_friends' DESC,
                    fl.list_type = 'restricted' DESC,
                    fl.name"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to list friend lists");
        ApiError::Internal("Database error".into())
    })?;

    let lists: Vec<FriendListResponse> = rows
        .iter()
        .map(|r| FriendListResponse {
            id: r.get("id"),
            name: r.get("name"),
            list_type: r.get("list_type"),
            member_count: r.get("member_count"),
        })
        .collect();

    Ok(Json(serde_json::json!({ "data": lists })))
}

/// POST /v1/users/me/friend-lists
pub async fn create_friend_list(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateListRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let name = body.name.trim().to_string();
    if name.is_empty() || name.len() > 128 {
        return Err(ApiError::BadRequest(
            "Name must be 1-128 characters".into(),
        ));
    }

    let row = sqlx::query(
        r#"INSERT INTO friend_lists (user_id, name, list_type)
           VALUES ($1, $2, $3)
           ON CONFLICT (user_id, name) DO UPDATE SET list_type = EXCLUDED.list_type
           RETURNING id, name, list_type"#,
    )
    .bind(auth.user_id)
    .bind(&name)
    .bind(&body.list_type)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to create friend list");
        ApiError::Internal("Database error".into())
    })?;

    let response = FriendListResponse {
        id: row.get("id"),
        name: row.get("name"),
        list_type: row.get("list_type"),
        member_count: 0,
    };

    Ok(Json(serde_json::json!({ "data": response })))
}

/// DELETE /v1/users/me/friend-lists/:id
pub async fn delete_friend_list(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(list_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "DELETE FROM friend_lists WHERE id = $1 AND user_id = $2 AND list_type = 'custom'",
    )
    .bind(list_id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("Database error".into())
    })?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(
            "List not found or cannot be deleted".into(),
        ));
    }
    Ok(Json(serde_json::json!({ "data": null })))
}

/// POST /v1/users/me/friend-lists/:id/members
pub async fn add_friend_to_list(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(list_id): Path<i64>,
    Json(body): Json<AddMemberRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Verify the list belongs to the authenticated user
    let list_owner: Option<i64> = sqlx::query_scalar(
        "SELECT user_id FROM friend_lists WHERE id = $1",
    )
    .bind(list_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("Database error".into())
    })?;

    match list_owner {
        Some(uid) if uid == auth.user_id => {}
        Some(_) => return Err(ApiError::NotFound("List not found".into())),
        None => return Err(ApiError::NotFound("List not found".into())),
    }

    let _ = sqlx::query(
        "INSERT INTO friend_list_members (list_id, friend_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(list_id)
    .bind(body.friend_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("Database error".into())
    })?;

    Ok(Json(serde_json::json!({ "data": null })))
}

/// DELETE /v1/users/me/friend-lists/:id/members/:friend_id
pub async fn remove_friend_from_list(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((list_id, friend_id)): Path<(i64, i64)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Verify the list belongs to the authenticated user
    let list_owner: Option<i64> = sqlx::query_scalar(
        "SELECT user_id FROM friend_lists WHERE id = $1",
    )
    .bind(list_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("Database error".into())
    })?;

    match list_owner {
        Some(uid) if uid == auth.user_id => {}
        Some(_) => return Err(ApiError::NotFound("List not found".into())),
        None => return Err(ApiError::NotFound("List not found".into())),
    }

    let _ = sqlx::query(
        "DELETE FROM friend_list_members WHERE list_id = $1 AND friend_id = $2",
    )
    .bind(list_id)
    .bind(friend_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("Database error".into())
    })?;

    Ok(Json(serde_json::json!({ "data": null })))
}
