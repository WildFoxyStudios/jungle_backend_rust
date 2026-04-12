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
        return Err(ApiError::Forbidden("Admin access required".into()));
    }
    Ok(())
}

// ── Auto Settings ───────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct AutoFollowRow {
    pub id: i64,
    pub user_id: i64,
    pub is_active: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AutoJoinRow {
    pub id: i64,
    pub group_id: i64,
    pub is_active: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AutoLikeRow {
    pub id: i64,
    pub page_id: i64,
    pub is_active: bool,
}

pub async fn get_auto_settings(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let auto_friends = sqlx::query_as::<_, AutoFollowRow>(
        "SELECT id, user_id, is_active FROM auto_follow_accounts ORDER BY id",
    )
    .fetch_all(&state.db)
    .await?;

    let auto_joins = sqlx::query_as::<_, AutoJoinRow>(
        "SELECT id, group_id, is_active FROM auto_join_groups ORDER BY id",
    )
    .fetch_all(&state.db)
    .await?;

    let auto_likes = sqlx::query_as::<_, AutoLikeRow>(
        "SELECT id, page_id, is_active FROM auto_like_pages ORDER BY id",
    )
    .fetch_all(&state.db)
    .await?;

    let auto_delete = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM site_config WHERE category = 'auto_delete' ORDER BY key",
    )
    .fetch_all(&state.db)
    .await?;

    let delete_config: serde_json::Map<String, Value> = auto_delete
        .into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect();

    Ok(Json(json!({
        "data": {
            "auto_friends": auto_friends,
            "auto_joins": auto_joins,
            "auto_likes": auto_likes,
            "auto_delete": delete_config
        }
    })))
}

#[derive(Debug, Deserialize)]
pub struct AutoFriendRequest {
    pub user_id: i64,
}

pub async fn add_auto_friend(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AutoFriendRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO auto_follow_accounts (user_id, is_active) VALUES ($1, true)
         ON CONFLICT DO NOTHING RETURNING id",
    )
    .bind(req.user_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(0);

    Ok(Json(json!({ "data": { "id": id } })))
}

pub async fn remove_auto_friend(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM auto_follow_accounts WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

#[derive(Debug, Deserialize)]
pub struct AutoJoinRequest {
    pub group_id: i64,
}

pub async fn add_auto_join(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AutoJoinRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO auto_join_groups (group_id, is_active) VALUES ($1, true)
         ON CONFLICT DO NOTHING RETURNING id",
    )
    .bind(req.group_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(0);

    Ok(Json(json!({ "data": { "id": id } })))
}

pub async fn remove_auto_join(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM auto_join_groups WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

#[derive(Debug, Deserialize)]
pub struct AutoLikeRequest {
    pub page_id: i64,
}

pub async fn add_auto_like(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AutoLikeRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO auto_like_pages (page_id, is_active) VALUES ($1, true)
         ON CONFLICT DO NOTHING RETURNING id",
    )
    .bind(req.page_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(0);

    Ok(Json(json!({ "data": { "id": id } })))
}

pub async fn remove_auto_like(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM auto_like_pages WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

#[derive(Debug, Deserialize)]
pub struct AutoDeleteSettingsRequest {
    pub enabled: bool,
    pub delete_posts_older_days: Option<i32>,
    pub delete_stories_enabled: Option<bool>,
    pub delete_inactive_users_days: Option<i32>,
}

pub async fn update_auto_delete_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AutoDeleteSettingsRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let mut tx = state.db.begin().await?;

    let pairs: Vec<(&str, String)> = vec![
        ("enabled", req.enabled.to_string()),
        ("delete_posts_older_days", req.delete_posts_older_days.unwrap_or(365).to_string()),
        ("delete_stories_enabled", req.delete_stories_enabled.unwrap_or(true).to_string()),
        ("delete_inactive_users_days", req.delete_inactive_users_days.unwrap_or(730).to_string()),
    ];

    for (key, value) in pairs {
        sqlx::query(
            "INSERT INTO site_config (category, key, value) VALUES ('auto_delete', $1, $2)
             ON CONFLICT (category, key) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(key)
        .bind(&value)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(Json(json!({ "data": { "updated": true } })))
}

// ── Custom Code ─────────────────────────────────────────────────────

pub async fn get_custom_code(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT position, content FROM custom_code ORDER BY position",
    )
    .fetch_all(&state.db)
    .await?;

    let code: serde_json::Map<String, Value> =
        rows.into_iter().map(|(pos, content)| (pos, Value::String(content))).collect();

    Ok(Json(json!({ "data": code })))
}

#[derive(Debug, Deserialize)]
pub struct CustomCodeRequest {
    pub header: Option<String>,
    pub footer: Option<String>,
}

pub async fn update_custom_code(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CustomCodeRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    if let Some(header) = &req.header {
        sqlx::query(
            "INSERT INTO custom_code (position, content) VALUES ('header', $1)
             ON CONFLICT (position) DO UPDATE SET content = EXCLUDED.content, updated_at = NOW()",
        )
        .bind(header)
        .execute(&state.db)
        .await?;
    }

    if let Some(footer) = &req.footer {
        sqlx::query(
            "INSERT INTO custom_code (position, content) VALUES ('footer', $1)
             ON CONFLICT (position) DO UPDATE SET content = EXCLUDED.content, updated_at = NOW()",
        )
        .bind(footer)
        .execute(&state.db)
        .await?;
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

// ── Site Ads (admin-managed system ads) ────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct SiteAdRow {
    pub id: i64,
    pub name: String,
    pub ad_type: String,
    pub position: String,
    pub is_active: bool,
    pub views: i32,
    pub clicks: i32,
    pub created_at: OffsetDateTime,
}

pub async fn list_site_ads(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let ads = sqlx::query_as::<_, SiteAdRow>(
        "SELECT id, name, ad_type, position, is_active, views, clicks, created_at
         FROM site_ads ORDER BY id DESC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": ads })))
}

#[derive(Debug, Deserialize)]
pub struct SiteAdRequest {
    pub name: String,
    pub ad_type: Option<String>,
    pub content: String,
    pub image: Option<String>,
    pub url: Option<String>,
    pub position: Option<String>,
    pub is_active: Option<bool>,
}

pub async fn create_site_ad(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SiteAdRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO site_ads (name, ad_type, content, image, url, position, is_active)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(req.name.trim())
    .bind(req.ad_type.as_deref().unwrap_or("banner"))
    .bind(&req.content)
    .bind(req.image.as_deref().unwrap_or(""))
    .bind(req.url.as_deref().unwrap_or(""))
    .bind(req.position.as_deref().unwrap_or("sidebar"))
    .bind(req.is_active.unwrap_or(true))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id } })))
}

pub async fn update_site_ad(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<SiteAdRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query(
        "UPDATE site_ads SET name=$1, ad_type=$2, content=$3, image=$4, url=$5, position=$6,
         is_active=$7, updated_at=NOW() WHERE id=$8",
    )
    .bind(req.name.trim())
    .bind(req.ad_type.as_deref().unwrap_or("banner"))
    .bind(&req.content)
    .bind(req.image.as_deref().unwrap_or(""))
    .bind(req.url.as_deref().unwrap_or(""))
    .bind(req.position.as_deref().unwrap_or("sidebar"))
    .bind(req.is_active.unwrap_or(true))
    .bind(id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "updated": true } })))
}

pub async fn delete_site_ad(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM site_ads WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── User Permissions ────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct UserPermissionRow {
    pub user_id: i64,
    pub can_moderate_posts: bool,
    pub can_moderate_users: bool,
    pub can_moderate_reports: bool,
    pub can_manage_content: bool,
    pub can_manage_payments: bool,
}

pub async fn get_user_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let perms = sqlx::query_as::<_, UserPermissionRow>(
        "SELECT user_id, can_moderate_posts, can_moderate_users, can_moderate_reports,
                can_manage_content, can_manage_payments
         FROM user_permissions WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(json!({ "data": perms.unwrap_or(UserPermissionRow {
        user_id,
        can_moderate_posts: false,
        can_moderate_users: false,
        can_moderate_reports: false,
        can_manage_content: false,
        can_manage_payments: false,
    }) })))
}

#[derive(Debug, Deserialize)]
pub struct UpdatePermissionsRequest {
    pub can_moderate_posts: Option<bool>,
    pub can_moderate_users: Option<bool>,
    pub can_moderate_reports: Option<bool>,
    pub can_manage_content: Option<bool>,
    pub can_manage_payments: Option<bool>,
}

pub async fn update_user_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
    Json(req): Json<UpdatePermissionsRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query(
        r#"INSERT INTO user_permissions
             (user_id, can_moderate_posts, can_moderate_users, can_moderate_reports,
              can_manage_content, can_manage_payments, granted_by)
           VALUES ($1,$2,$3,$4,$5,$6,$7)
           ON CONFLICT (user_id) DO UPDATE SET
             can_moderate_posts   = EXCLUDED.can_moderate_posts,
             can_moderate_users   = EXCLUDED.can_moderate_users,
             can_moderate_reports = EXCLUDED.can_moderate_reports,
             can_manage_content   = EXCLUDED.can_manage_content,
             can_manage_payments  = EXCLUDED.can_manage_payments,
             granted_by           = EXCLUDED.granted_by,
             updated_at           = NOW()"#,
    )
    .bind(user_id)
    .bind(req.can_moderate_posts.unwrap_or(false))
    .bind(req.can_moderate_users.unwrap_or(false))
    .bind(req.can_moderate_reports.unwrap_or(false))
    .bind(req.can_manage_content.unwrap_or(false))
    .bind(req.can_manage_payments.unwrap_or(false))
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "updated": true } })))
}

// ── Advanced User Management ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TopUpWalletRequest {
    pub amount: rust_decimal::Decimal,
    pub note: Option<String>,
}

pub async fn top_up_wallet(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
    Json(req): Json<TopUpWalletRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    if req.amount <= rust_decimal::Decimal::ZERO {
        return Err(ApiError::BadRequest("amount must be positive".into()));
    }

    let mut tx = state.db.begin().await?;

    sqlx::query("UPDATE users SET wallet = wallet + $1 WHERE id = $2")
        .bind(req.amount)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO payment_transactions (user_id, amount, currency, provider, type, status, metadata)
         VALUES ($1, $2, 'USD', 'admin', 'admin_top_up', 'completed', $3)",
    )
    .bind(user_id)
    .bind(req.amount)
    .bind(json!({ "note": req.note, "admin_id": auth.user_id }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(json!({ "data": { "topped_up": true, "amount": req.amount } })))
}

#[derive(Debug, Deserialize)]
pub struct SendEmailRequest {
    pub subject: String,
    pub body: String,
    pub user_id: Option<i64>,
}

pub async fn send_email_to_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SendEmailRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    if req.subject.trim().is_empty() || req.body.trim().is_empty() {
        return Err(ApiError::BadRequest("subject and body are required".into()));
    }

    // Record in sent_emails table
    let recipients = if let Some(uid) = req.user_id {
        let email: Option<String> = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
            .bind(uid)
            .fetch_optional(&state.db)
            .await?;
        email.map(|e| vec![e]).unwrap_or_default()
    } else {
        // Get all active user emails (for broadcast)
        sqlx::query_scalar("SELECT email FROM users WHERE deleted_at IS NULL AND is_active = TRUE LIMIT 1000")
            .fetch_all(&state.db)
            .await?
    };

    if recipients.is_empty() {
        return Err(ApiError::NotFound("No recipients found".into()));
    }

    sqlx::query(
        "INSERT INTO sent_emails (subject, body, recipient_count, sent_by) VALUES ($1, $2, $3, $4)",
    )
    .bind(req.subject.trim())
    .bind(req.body.trim())
    .bind(recipients.len() as i32)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "queued": true, "recipients": recipients.len() } })))
}

pub async fn delete_user_content(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let mut tx = state.db.begin().await?;

    // Soft delete posts
    let posts: i64 = sqlx::query_scalar(
        "WITH updated AS (UPDATE posts SET deleted_at = NOW() WHERE user_id = $1 AND deleted_at IS NULL RETURNING id)
         SELECT COUNT(*) FROM updated",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    // Delete stories
    let stories: i64 = sqlx::query_scalar(
        "WITH deleted AS (DELETE FROM stories WHERE user_id = $1 RETURNING id)
         SELECT COUNT(*) FROM deleted",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    // Delete notifications
    sqlx::query("DELETE FROM notifications WHERE recipient_id = $1 OR sender_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(json!({
        "data": {
            "posts_deleted": posts,
            "stories_deleted": stories
        }
    })))
}

// ── Content Monetization Admin ──────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct MonetizationSubRow {
    pub id: i64,
    pub creator_id: i64,
    pub subscriber_id: i64,
    pub amount: rust_decimal::Decimal,
    pub status: String,
    pub started_at: OffsetDateTime,
}

pub async fn list_monetization_subscriptions(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let rows = sqlx::query_as::<_, MonetizationSubRow>(
        "SELECT id, creator_id, subscriber_id, amount, status, started_at
         FROM creator_subscriptions WHERE id < $1 ORDER BY id DESC LIMIT $2",
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

// ── Get Category Config for Admin (settings pages) ─────────────────

pub async fn get_settings_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(category): Path<String>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let valid = ["auto_delete","push","ai","store","affiliates","pro","website_mode","ads"];
    if !valid.contains(&category.as_str()) {
        return Err(ApiError::BadRequest(format!("Unknown settings category: {}", category)));
    }

    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM site_config WHERE category = $1 ORDER BY key",
    )
    .bind(&category)
    .fetch_all(&state.db)
    .await?;

    let settings: serde_json::Map<String, Value> =
        rows.into_iter().map(|(k, v)| (k, Value::String(v))).collect();

    Ok(Json(json!({ "data": settings })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub settings: std::collections::HashMap<String, String>,
}

pub async fn update_settings_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(category): Path<String>,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let valid = ["auto_delete","push","ai","store","affiliates","pro","website_mode","ads"];
    if !valid.contains(&category.as_str()) {
        return Err(ApiError::BadRequest(format!("Unknown settings category: {}", category)));
    }

    let mut tx = state.db.begin().await?;

    for (key, value) in &req.settings {
        sqlx::query(
            "INSERT INTO site_config (category, key, value) VALUES ($1, $2, $3)
             ON CONFLICT (category, key) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(&category)
        .bind(key.as_str())
        .bind(value.as_str())
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(Json(json!({ "data": { "updated": req.settings.len() } })))
}
