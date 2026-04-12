use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;

fn require_admin(auth: &AuthUser) -> Result<(), ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }
    Ok(())
}

// ── Genders ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct GenderRow {
    pub id: i64,
    pub gender_id: String,
    pub name: String,
    pub image: String,
}

pub async fn list_genders(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = sqlx::query_as::<_, GenderRow>(
        "SELECT id, gender_id, name, image FROM genders ORDER BY id ASC",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!({ "data": rows })))
}

#[derive(Debug, Deserialize)]
pub struct GenderRequest {
    pub gender_id: String,
    pub name: String,
    #[serde(default)]
    pub image: String,
}

pub async fn create_gender(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<GenderRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let row = sqlx::query_as::<_, GenderRow>(
        "INSERT INTO genders (gender_id, name, image) VALUES ($1, $2, $3) RETURNING id, gender_id, name, image",
    )
    .bind(req.gender_id.trim())
    .bind(req.name.trim())
    .bind(req.image.trim())
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({ "data": row })))
}

pub async fn update_gender(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<GenderRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let result = sqlx::query(
        "UPDATE genders SET gender_id = $2, name = $3, image = $4 WHERE id = $1",
    )
    .bind(id)
    .bind(req.gender_id.trim())
    .bind(req.name.trim())
    .bind(req.image.trim())
    .execute(&state.db)
    .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Gender not found".into()));
    }
    Ok(Json(json!({ "data": { "updated": true } })))
}

pub async fn delete_gender(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let result = sqlx::query("DELETE FROM genders WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Gender not found".into()));
    }
    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Sub-Categories ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct SubCategoryRow {
    pub id: i64,
    pub category_id: i64,
    pub lang_key: String,
    #[sqlx(rename = "type")]
    pub sub_type: String,
    pub created_at: OffsetDateTime,
}

pub async fn list_sub_categories(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<SubCategoryFilter>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = if let Some(cat_id) = params.category_id {
        sqlx::query_as::<_, SubCategoryRow>(
            "SELECT id, category_id, lang_key, type, created_at FROM sub_categories WHERE category_id = $1 ORDER BY id ASC",
        )
        .bind(cat_id)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, SubCategoryRow>(
            "SELECT id, category_id, lang_key, type, created_at FROM sub_categories ORDER BY id ASC",
        )
        .fetch_all(&state.db)
        .await?
    };
    Ok(Json(json!({ "data": rows })))
}

#[derive(Debug, Deserialize)]
pub struct SubCategoryFilter {
    pub category_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SubCategoryRequest {
    pub category_id: i64,
    pub lang_key: String,
    #[serde(rename = "type", default)]
    pub sub_type: String,
}

pub async fn create_sub_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SubCategoryRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let row = sqlx::query_as::<_, SubCategoryRow>(
        "INSERT INTO sub_categories (category_id, lang_key, type) VALUES ($1, $2, $3) RETURNING id, category_id, lang_key, type, created_at",
    )
    .bind(req.category_id)
    .bind(req.lang_key.trim())
    .bind(req.sub_type.trim())
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({ "data": row })))
}

pub async fn update_sub_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<SubCategoryRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let result = sqlx::query(
        "UPDATE sub_categories SET category_id = $2, lang_key = $3, type = $4 WHERE id = $1",
    )
    .bind(id)
    .bind(req.category_id)
    .bind(req.lang_key.trim())
    .bind(req.sub_type.trim())
    .execute(&state.db)
    .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Sub-category not found".into()));
    }
    Ok(Json(json!({ "data": { "updated": true } })))
}

pub async fn delete_sub_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let result = sqlx::query("DELETE FROM sub_categories WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Sub-category not found".into()));
    }
    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Terms / Legal Pages ────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct TermsPageRow {
    pub id: i64,
    #[sqlx(rename = "type")]
    pub page_type: String,
    pub text: String,
}

pub async fn list_terms_pages(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = sqlx::query_as::<_, TermsPageRow>(
        "SELECT id, type, text FROM terms_pages ORDER BY id ASC",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!({ "data": rows })))
}

#[derive(Debug, Deserialize)]
pub struct TermsPageUpdate {
    pub text: String,
}

pub async fn update_terms_page(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<TermsPageUpdate>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let result = sqlx::query("UPDATE terms_pages SET text = $2 WHERE id = $1")
        .bind(id)
        .bind(&req.text)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Terms page not found".into()));
    }
    Ok(Json(json!({ "data": { "updated": true } })))
}

// ── Movies Admin ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct MovieAdminRow {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub cover: String,
    pub is_approved: bool,
    pub admin_featured: bool,
    pub created_at: OffsetDateTime,
}

pub async fn list_movies(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, MovieAdminRow>(
        "SELECT id, user_id, name, cover, is_approved, admin_featured, created_at FROM movies WHERE id < $1 ORDER BY id DESC LIMIT $2",
    )
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let has_more = rows.len() as i64 > limit;
    let data: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|r| r.id.to_string());
    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

pub async fn approve_movie(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("UPDATE movies SET is_approved = TRUE WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "approved": true } })))
}

pub async fn feature_movie(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("UPDATE movies SET admin_featured = NOT admin_featured WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "toggled": true } })))
}

pub async fn delete_movie(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM movies WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Games Admin ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct GameAdminRow {
    pub id: i64,
    pub name: String,
    pub game_link: String,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
}

pub async fn list_games(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = sqlx::query_as::<_, GameAdminRow>(
        "SELECT id, name, game_link, is_active, created_at FROM games ORDER BY id DESC",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!({ "data": rows })))
}

#[derive(Debug, Deserialize)]
pub struct GameRequest {
    pub name: String,
    pub game_link: String,
    pub description: Option<String>,
    pub cover: Option<String>,
}

pub async fn create_game(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<GameRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO games (name, game_link, description, cover) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(req.name.trim())
    .bind(req.game_link.trim())
    .bind(req.description.as_deref().unwrap_or(""))
    .bind(req.cover.as_deref().unwrap_or(""))
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({ "data": { "id": id } })))
}

pub async fn toggle_game(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("UPDATE games SET is_active = NOT is_active WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "toggled": true } })))
}

pub async fn delete_game(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM games WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Bank Receipts Admin ────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct BankReceiptRow {
    pub id: i64,
    pub user_id: i64,
    pub description: String,
    pub price: rust_decimal::Decimal,
    pub mode: String,
    pub approved: bool,
    pub receipt_file: String,
    pub created_at: OffsetDateTime,
}

pub async fn list_bank_receipts(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, BankReceiptRow>(
        "SELECT id, user_id, description, price, mode, approved, receipt_file, created_at FROM bank_receipts WHERE id < $1 ORDER BY id DESC LIMIT $2",
    )
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let has_more = rows.len() as i64 > limit;
    let data: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|r| r.id.to_string());
    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

pub async fn approve_bank_receipt(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let receipt = sqlx::query_as::<_, BankReceiptRow>(
        "SELECT id, user_id, description, price, mode, approved, receipt_file, created_at FROM bank_receipts WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("Receipt not found".into()))?;

    if receipt.approved {
        return Err(ApiError::BadRequest("Receipt already approved".into()));
    }

    let mut tx = state.db.begin().await?;

    sqlx::query("UPDATE bank_receipts SET approved = TRUE, approved_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // Credit the user's wallet
    sqlx::query("UPDATE users SET wallet = wallet + $2 WHERE id = $1")
        .bind(receipt.user_id)
        .bind(receipt.price)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(json!({ "data": { "approved": true, "amount": receipt.price.to_string() } })))
}

pub async fn reject_bank_receipt(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let result = sqlx::query("DELETE FROM bank_receipts WHERE id = $1 AND approved = FALSE")
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Receipt not found or already approved".into()));
    }
    Ok(Json(json!({ "data": { "rejected": true } })))
}

// ── Currencies Admin ───────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct CurrencyRow {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub symbol: String,
    pub format: String,
    pub is_active: bool,
}

pub async fn list_currencies(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = sqlx::query_as::<_, CurrencyRow>(
        "SELECT id, code, name, symbol, format, is_active FROM currencies ORDER BY code ASC",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!({ "data": rows })))
}

#[derive(Debug, Deserialize)]
pub struct CurrencyRequest {
    pub code: String,
    pub name: String,
    pub symbol: String,
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "{symbol}{amount}".to_string()
}

pub async fn create_currency(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CurrencyRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let row = sqlx::query_as::<_, CurrencyRow>(
        "INSERT INTO currencies (code, name, symbol, format) VALUES ($1, $2, $3, $4) RETURNING id, code, name, symbol, format, is_active",
    )
    .bind(req.code.trim().to_uppercase())
    .bind(req.name.trim())
    .bind(req.symbol.trim())
    .bind(req.format.trim())
    .fetch_one(&state.db)
    .await?;
    Ok(Json(json!({ "data": row })))
}

pub async fn update_currency(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<CurrencyRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let result = sqlx::query(
        "UPDATE currencies SET code = $2, name = $3, symbol = $4, format = $5 WHERE id = $1",
    )
    .bind(id)
    .bind(req.code.trim().to_uppercase())
    .bind(req.name.trim())
    .bind(req.symbol.trim())
    .bind(req.format.trim())
    .execute(&state.db)
    .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Currency not found".into()));
    }
    Ok(Json(json!({ "data": { "updated": true } })))
}

pub async fn toggle_currency(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("UPDATE currencies SET is_active = NOT is_active WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "toggled": true } })))
}

pub async fn delete_currency(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM currencies WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}
