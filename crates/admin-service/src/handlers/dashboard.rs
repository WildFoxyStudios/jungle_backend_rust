use axum::{extract::State, Json};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    permissions::Permission,
};

pub async fn stats(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ViewDashboard, &state).await?;

    // ── Core counts ──
    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db).await?;

    let new_users_today: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE created_at >= CURRENT_DATE",
    ).fetch_one(&state.db).await?;

    let online_users: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE last_seen >= NOW() - INTERVAL '5 minutes'",
    ).fetch_one(&state.db).await.unwrap_or(0);

    let pro_subscribers: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE is_pro = TRUE AND deleted_at IS NULL",
    ).fetch_one(&state.db).await?;

    let total_posts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL")
        .fetch_one(&state.db).await?;

    let total_groups: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM groups WHERE active = TRUE")
        .fetch_one(&state.db).await?;

    let total_pages: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pages WHERE active = TRUE")
        .fetch_one(&state.db).await?;

    // ── Content counts ──
    let total_blogs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blogs")
        .fetch_one(&state.db).await?;

    let total_products: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM products WHERE status = 'active'")
        .fetch_one(&state.db).await?;

    let total_stories: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM stories WHERE expires_at > NOW()",
    ).fetch_one(&state.db).await.unwrap_or(0);

    let total_messages: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
        .fetch_one(&state.db).await.unwrap_or(0);

    // ── Pending actions ──
    let pending_reports: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reports WHERE status = 'pending'",
    ).fetch_one(&state.db).await?;

    let pending_verifications: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM verification_requests WHERE status = 'pending'",
    ).fetch_one(&state.db).await.unwrap_or(0);

    let pending_withdrawals: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM withdrawal_requests WHERE status = 'pending'",
    ).fetch_one(&state.db).await.unwrap_or(0);

    let pending_posts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM posts WHERE is_approved = FALSE AND deleted_at IS NULL",
    ).fetch_one(&state.db).await.unwrap_or(0);

    // ── Revenue ──
    let revenue_today: rust_decimal::Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM payment_transactions WHERE status = 'completed' AND created_at >= CURRENT_DATE",
    ).fetch_one(&state.db).await.unwrap_or_default();

    let revenue_month: rust_decimal::Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM payment_transactions WHERE status = 'completed' AND created_at >= DATE_TRUNC('month', CURRENT_DATE)",
    ).fetch_one(&state.db).await.unwrap_or_default();

    // ── Ads ──
    let active_ads: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_ads WHERE status = 'active' AND budget > 0",
    ).fetch_one(&state.db).await.unwrap_or(0);

    Ok(Json(json!({
        "data": {
            "users": {
                "total": total_users,
                "new_today": new_users_today,
                "online": online_users,
                "pro_subscribers": pro_subscribers,
            },
            "content": {
                "total_posts": total_posts,
                "total_groups": total_groups,
                "total_pages": total_pages,
                "total_blogs": total_blogs,
                "total_products": total_products,
                "total_stories": total_stories,
                "total_messages": total_messages,
            },
            "pending": {
                "reports": pending_reports,
                "verifications": pending_verifications,
                "withdrawals": pending_withdrawals,
                "posts_moderation": pending_posts,
            },
            "revenue": {
                "today": revenue_today.to_string(),
                "this_month": revenue_month.to_string(),
            },
            "ads": {
                "active": active_ads,
            }
        }
    })))
}
