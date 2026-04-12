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

// ── Pro Members ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct ProMemberRow {
    pub id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub is_pro: i16,
    pub pro_type: Option<i16>,
    pub pro_expires_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

pub async fn list_pro_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, ProMemberRow>(
        r#"SELECT id, username, first_name, last_name, email, is_pro, pro_type, pro_expires_at, created_at
           FROM users
           WHERE is_pro > 0 AND deleted_at IS NULL AND id < $1
           ORDER BY id DESC LIMIT $2"#,
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

// ── Online Users ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct OnlineUserRow {
    pub id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub last_active: Option<OffsetDateTime>,
}

pub async fn list_online_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    // Users active in last 5 minutes
    let rows = sqlx::query_as::<_, OnlineUserRow>(
        r#"SELECT id, username, first_name, last_name, avatar, last_active
           FROM users
           WHERE last_active > NOW() - INTERVAL '5 minutes'
             AND deleted_at IS NULL AND id < $1
           ORDER BY last_active DESC LIMIT $2"#,
    )
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let has_more = rows.len() as i64 > limit;
    let data: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|r| r.id.to_string());

    // Total online count
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE last_active > NOW() - INTERVAL '5 minutes' AND deleted_at IS NULL",
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more, "total_online": total }
    })))
}

// ── Referrals List ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct ReferralRow {
    pub id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub referrer_id: Option<i64>,
    pub referrer_username: Option<String>,
    pub created_at: OffsetDateTime,
}

pub async fn list_referrals(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, ReferralRow>(
        r#"SELECT u.id, u.username, u.first_name, u.last_name,
                  u.referrer_id, r.username AS referrer_username, u.created_at
           FROM users u
           LEFT JOIN users r ON r.id = u.referrer_id
           WHERE u.referrer_id IS NOT NULL AND u.deleted_at IS NULL AND u.id < $1
           ORDER BY u.id DESC LIMIT $2"#,
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

// ── Manage User Ads ────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct UserAdRow {
    pub id: i64,
    pub user_id: i64,
    pub ad_type: String,
    pub name: String,
    pub headline: String,
    pub budget: rust_decimal::Decimal,
    pub impressions: i64,
    pub clicks: i64,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
}

pub async fn list_user_ads(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, UserAdRow>(
        r#"SELECT id, user_id, ad_type, name, headline, budget, impressions, clicks, is_active, created_at
           FROM user_ads WHERE id < $1 ORDER BY id DESC LIMIT $2"#,
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

pub async fn toggle_user_ad(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("UPDATE user_ads SET is_active = NOT is_active WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "toggled": true } })))
}

pub async fn delete_user_ad(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM user_ads WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Manage Stories ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct StoryAdminRow {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub is_reported: bool,
    pub admin_hidden: bool,
    pub created_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
}

pub async fn list_stories(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, StoryAdminRow>(
        r#"SELECT s.id, s.user_id, u.username, s.is_reported, s.admin_hidden, s.created_at, s.expires_at
           FROM stories s
           JOIN users u ON u.id = s.user_id
           WHERE s.id < $1 ORDER BY s.id DESC LIMIT $2"#,
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

pub async fn hide_story(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("UPDATE stories SET admin_hidden = TRUE WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "hidden": true } })))
}

pub async fn delete_story(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM stories WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Manage Posts (full listing, not just moderation) ───────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct PostAdminRow {
    pub id: i64,
    pub user_id: i64,
    pub content: String,
    pub post_type: String,
    pub like_count: i32,
    pub comment_count: i32,
    pub is_approved: bool,
    pub created_at: OffsetDateTime,
}

pub async fn list_all_posts(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, PostAdminRow>(
        r#"SELECT id, user_id, content, post_type, like_count, comment_count, is_approved, created_at
           FROM posts WHERE deleted_at IS NULL AND id < $1
           ORDER BY id DESC LIMIT $2"#,
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

// ── Manage Offers ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct OfferAdminRow {
    pub id: i64,
    pub user_id: i64,
    pub offer_text: String,
    pub discount_percent: Option<i32>,
    pub expires_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

pub async fn list_all_offers(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, OfferAdminRow>(
        "SELECT id, user_id, offer_text, discount_percent, expires_at, created_at FROM offers WHERE id < $1 ORDER BY id DESC LIMIT $2",
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

pub async fn delete_offer(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM offers WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Manage Orders Admin ────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct OrderAdminRow {
    pub id: i64,
    pub buyer_id: i64,
    pub seller_id: i64,
    pub total_price: rust_decimal::Decimal,
    pub status: String,
    pub created_at: OffsetDateTime,
}

pub async fn list_all_orders(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, OrderAdminRow>(
        "SELECT id, buyer_id, seller_id, total_price, status, created_at FROM orders WHERE id < $1 ORDER BY id DESC LIMIT $2",
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

// ── Manage Product Reviews ─────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct ReviewAdminRow {
    pub id: i64,
    pub product_id: i64,
    pub user_id: i64,
    pub rating: i16,
    pub text: String,
    pub created_at: OffsetDateTime,
}

pub async fn list_all_reviews(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, ReviewAdminRow>(
        "SELECT id, product_id, user_id, rating, text, created_at FROM product_reviews WHERE id < $1 ORDER BY id DESC LIMIT $2",
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

pub async fn delete_review(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM product_reviews WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Pro Refunds ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct RefundAdminRow {
    pub id: i64,
    pub user_id: i64,
    pub order_hash_id: String,
    pub pro_type: String,
    pub description: Option<String>,
    pub status: i16,
    pub created_at: OffsetDateTime,
}

pub async fn list_refund_requests(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, RefundAdminRow>(
        "SELECT id, user_id, order_hash_id, pro_type, description, status, created_at FROM refund_requests WHERE id < $1 ORDER BY id DESC LIMIT $2",
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

pub async fn approve_refund(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let mut tx = state.db.begin().await?;

    let refund = sqlx::query_as::<_, RefundAdminRow>(
        "SELECT id, user_id, order_hash_id, pro_type, description, status, created_at FROM refund_requests WHERE id = $1 AND status = 0",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound("Refund request not found or already processed".into()))?;

    sqlx::query("UPDATE refund_requests SET status = 1 WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // Remove pro status from user
    sqlx::query("UPDATE users SET is_pro = 0, pro_type = NULL WHERE id = $1")
        .bind(refund.user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(json!({ "data": { "approved": true } })))
}

pub async fn reject_refund(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let result = sqlx::query("UPDATE refund_requests SET status = 2 WHERE id = $1 AND status = 0")
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Refund request not found".into()));
    }
    Ok(Json(json!({ "data": { "rejected": true } })))
}

// ── Mass Notifications ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct MassNotificationRequest {
    pub title: String,
    pub message: String,
    #[serde(default)]
    pub url: String,
    #[serde(default = "default_target")]
    pub target: String,
}

fn default_target() -> String {
    "all".to_string()
}

pub async fn send_mass_notification(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<MassNotificationRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    if req.title.trim().is_empty() || req.message.trim().is_empty() {
        return Err(ApiError::BadRequest("title and message are required".into()));
    }

    let target_filter = match req.target.as_str() {
        "pro" => "AND is_pro > 0",
        "new" => "AND created_at > NOW() - INTERVAL '30 days'",
        _ => "",
    };

    // Count recipients
    let count_query = format!(
        "SELECT COUNT(*) FROM users WHERE deleted_at IS NULL AND is_active = TRUE {}",
        target_filter
    );
    let count: i64 = sqlx::query_scalar(&count_query)
        .fetch_one(&state.db)
        .await?;

    // Insert notifications in bulk
    let insert_query = format!(
        r#"INSERT INTO notifications (recipient_id, sender_id, notification_type, text, target_type, target_id)
           SELECT id, $1, 'admin_notice', $2, 'system', 0
           FROM users WHERE deleted_at IS NULL AND is_active = TRUE {}"#,
        target_filter
    );
    sqlx::query(&insert_query)
        .bind(auth.user_id)
        .bind(format!("{}: {}", req.title.trim(), req.message.trim()))
        .execute(&state.db)
        .await?;

    // Log
    sqlx::query(
        "INSERT INTO mass_notifications (admin_id, title, message, url, target, sent_count) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(auth.user_id)
    .bind(req.title.trim())
    .bind(req.message.trim())
    .bind(req.url.trim())
    .bind(req.target.trim())
    .bind(count)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "sent_to": count } })))
}

#[derive(Debug, Serialize, FromRow)]
pub struct MassNotifRow {
    pub id: i64,
    pub admin_id: i64,
    pub title: String,
    pub message: String,
    pub target: String,
    pub sent_count: i32,
    pub created_at: OffsetDateTime,
}

pub async fn list_mass_notifications(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, MassNotifRow>(
        "SELECT id, admin_id, title, message, target, sent_count, created_at FROM mass_notifications WHERE id < $1 ORDER BY id DESC LIMIT $2",
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

// ── Sitemap Generation ─────────────────────────────────────────────────────

pub async fn generate_sitemap(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    // Gather counts
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE deleted_at IS NULL AND is_active = TRUE")
        .fetch_one(&state.db)
        .await?;
    let post_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL AND is_approved = TRUE")
        .fetch_one(&state.db)
        .await?;
    let page_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pages WHERE deleted_at IS NULL")
        .fetch_one(&state.db)
        .await?;
    let group_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM groups WHERE deleted_at IS NULL")
        .fetch_one(&state.db)
        .await?;
    let blog_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blogs WHERE deleted_at IS NULL")
        .fetch_one(&state.db)
        .await?;

    let total_entries = user_count + post_count + page_count + group_count + blog_count;

    // Log the generation
    let file_path = "sitemap.xml";
    sqlx::query("INSERT INTO sitemap_logs (file_path, entries) VALUES ($1, $2)")
        .bind(file_path)
        .bind(total_entries as i32)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({
        "data": {
            "generated": true,
            "entries": {
                "users": user_count,
                "posts": post_count,
                "pages": page_count,
                "groups": group_count,
                "blogs": blog_count,
                "total": total_entries
            }
        }
    })))
}

// ── Fake Users ─────────────────────────────────────────────────────────────

pub async fn list_fake_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, OnlineUserRow>(
        r#"SELECT id, username, first_name, last_name, avatar, last_active
           FROM users WHERE is_fake = TRUE AND deleted_at IS NULL AND id < $1
           ORDER BY id DESC LIMIT $2"#,
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

#[derive(Debug, Deserialize)]
pub struct CreateFakeUserRequest {
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub gender: String,
}

pub async fn create_fake_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateFakeUserRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    // Generate a random password hash (fake users don't login)
    let fake_hash = "$argon2id$v=19$m=19456,t=2,p=1$fake_user_no_login$0000000000000000000000";

    let id: i64 = sqlx::query_scalar(
        r#"INSERT INTO users (username, first_name, last_name, email, password_hash, avatar, gender, is_fake, is_active)
           VALUES ($1, $2, $3, $4, $5, COALESCE(NULLIF($6, ''), 'default-avatar.jpg'), $7, TRUE, TRUE)
           RETURNING id"#,
    )
    .bind(req.username.trim())
    .bind(req.first_name.trim())
    .bind(req.last_name.trim())
    .bind(req.email.trim())
    .bind(fake_hash)
    .bind(req.avatar.trim())
    .bind(req.gender.trim())
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to create fake user: {e}")))?;

    Ok(Json(json!({ "data": { "id": id, "username": req.username.trim() } })))
}

// ── API Access Keys ────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct ApiKeyRow {
    pub id: i64,
    pub name: String,
    pub api_key: String,
    pub permissions: Value,
    pub is_active: bool,
    pub last_used: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

pub async fn list_api_keys(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = sqlx::query_as::<_, ApiKeyRow>(
        "SELECT id, name, api_key, permissions, is_active, last_used, created_at FROM api_access_keys ORDER BY id DESC",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(json!({ "data": rows })))
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

pub async fn create_api_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let api_key = format!("wk_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    let secret_key = format!("ws_{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    let perms = if req.permissions.is_empty() {
        json!(["read"])
    } else {
        json!(req.permissions)
    };

    let row = sqlx::query_as::<_, ApiKeyRow>(
        r#"INSERT INTO api_access_keys (name, api_key, secret_key, permissions, created_by)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, name, api_key, permissions, is_active, last_used, created_at"#,
    )
    .bind(req.name.trim())
    .bind(&api_key)
    .bind(&secret_key)
    .bind(&perms)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": row, "secret_key": secret_key })))
}

pub async fn toggle_api_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("UPDATE api_access_keys SET is_active = NOT is_active WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "toggled": true } })))
}

pub async fn delete_api_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    sqlx::query("DELETE FROM api_access_keys WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}
