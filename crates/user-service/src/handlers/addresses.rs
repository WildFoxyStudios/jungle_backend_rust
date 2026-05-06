use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Serialize, FromRow)]
pub struct AddressRow {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub phone: String,
    pub country: String,
    pub city: String,
    pub zip: String,
    pub address: String,
    pub is_default: bool,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateAddressRequest {
    pub name: String,
    pub phone: String,
    pub country: String,
    pub city: String,
    pub zip: String,
    pub address: String,
    #[serde(default)]
    pub is_default: bool,
}

/// GET /v1/users/me/addresses
pub async fn list_addresses(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let rows = sqlx::query_as::<_, AddressRow>(
        r#"SELECT id, user_id, name, phone, country, city, zip, address, is_default, created_at
           FROM user_addresses
           WHERE user_id = $1 AND id < $2
           ORDER BY id DESC LIMIT $3"#,
    )
    .bind(auth.user_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let data: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|r| r.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

/// GET /v1/users/me/addresses/{id}
pub async fn get_address(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query_as::<_, AddressRow>(
        "SELECT id, user_id, name, phone, country, city, zip, address, is_default, created_at FROM user_addresses WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Address not found".into()))?;

    Ok(Json(json!({ "data": row })))
}

/// POST /v1/users/me/addresses
pub async fn create_address(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateAddressRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("Name is required".into()));
    }

    let mut tx = state.db.begin().await?;

    // If setting as default, unset all other defaults
    if req.is_default {
        sqlx::query("UPDATE user_addresses SET is_default = FALSE WHERE user_id = $1")
            .bind(auth.user_id)
            .execute(&mut *tx)
            .await?;
    }

    let row = sqlx::query_as::<_, AddressRow>(
        r#"INSERT INTO user_addresses (user_id, name, phone, country, city, zip, address, is_default)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id, user_id, name, phone, country, city, zip, address, is_default, created_at"#,
    )
    .bind(auth.user_id)
    .bind(req.name.trim())
    .bind(req.phone.trim())
    .bind(req.country.trim())
    .bind(req.city.trim())
    .bind(req.zip.trim())
    .bind(req.address.trim())
    .bind(req.is_default)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(json!({ "data": row })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateAddressRequest {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub zip: Option<String>,
    pub address: Option<String>,
    pub is_default: Option<bool>,
}

/// PUT /v1/users/me/addresses/{id}
pub async fn update_address(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateAddressRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Verify ownership
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_addresses WHERE id = $1 AND user_id = $2)",
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if !exists {
        return Err(ApiError::NotFound("Address not found".into()));
    }

    let mut tx = state.db.begin().await?;

    if req.is_default == Some(true) {
        sqlx::query("UPDATE user_addresses SET is_default = FALSE WHERE user_id = $1")
            .bind(auth.user_id)
            .execute(&mut *tx)
            .await?;
    }

    let row = sqlx::query_as::<_, AddressRow>(
        r#"UPDATE user_addresses SET
           name = COALESCE($3, name),
           phone = COALESCE($4, phone),
           country = COALESCE($5, country),
           city = COALESCE($6, city),
           zip = COALESCE($7, zip),
           address = COALESCE($8, address),
           is_default = COALESCE($9, is_default)
           WHERE id = $1 AND user_id = $2
           RETURNING id, user_id, name, phone, country, city, zip, address, is_default, created_at"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(req.name.as_deref().map(str::trim))
    .bind(req.phone.as_deref().map(str::trim))
    .bind(req.country.as_deref().map(str::trim))
    .bind(req.city.as_deref().map(str::trim))
    .bind(req.zip.as_deref().map(str::trim))
    .bind(req.address.as_deref().map(str::trim))
    .bind(req.is_default)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(json!({ "data": row })))
}

/// DELETE /v1/users/me/addresses/{id}
pub async fn delete_address(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query("DELETE FROM user_addresses WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Address not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}
