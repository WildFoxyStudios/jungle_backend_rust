//! Admin realtime platform statistics endpoint.
//! Provides high-level realtime metrics: active sessions, online users,
//! pending notifications, websocket connections, and recent activity.

use axum::{extract::State, Json};
use serde_json::{json, Value};
use shared::{auth::{AppState, AuthUser}, errors::ApiError, permissions::Permission};

/// GET /v1/admin/realtime/stats
///
/// Returns realtime platform metrics:
/// - online_users: distinct users with active sessions (last 5 min)
/// - active_sessions: total active session rows
/// - active_conversations: conversations with recent messages (last hour)
/// - pending_notifications: notifications not yet processed/sent
/// - recent_signups: users registered last hour
/// - recent_posts: posts created last hour
/// - pending_jobs: background jobs in 'pending' status
pub async fn realtime_stats(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ViewDashboard, &state).await?;
    // Online users (distinct users with sessions updated in last 5 minutes)
    let online_users: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT user_id)::bigint
          FROM sessions
         WHERE updated_at > NOW() - INTERVAL '5 minutes'
           AND user_id IS NOT NULL
        "#,
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    // Total active sessions
    let active_sessions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM sessions WHERE expires_at > NOW()",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    // Active conversations (with messages in last hour)
    let active_conversations: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT conversation_id)::bigint
          FROM messages
         WHERE created_at > NOW() - INTERVAL '1 hour'
        "#,
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    // Pending notifications (not read, created recently)
    let pending_notifications: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
          FROM notifications
         WHERE is_read = FALSE
           AND created_at > NOW() - INTERVAL '24 hours'
        "#,
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    // Recent signups (last hour)
    let recent_signups: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM users WHERE created_at > NOW() - INTERVAL '1 hour'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    // Recent posts (last hour, excluding deleted)
    let recent_posts: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
          FROM posts
         WHERE created_at > NOW() - INTERVAL '1 hour'
           AND deleted_at IS NULL
        "#,
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    // Pending background jobs
    let pending_jobs: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::bigint
          FROM background_jobs
         WHERE status = 'pending'
           AND run_at <= NOW()
        "#,
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    // Live streamers currently broadcasting
    let live_streamers: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT user_id)::bigint
          FROM live_streams
         WHERE ended_at IS NULL
           AND status = 'live'
        "#,
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    Ok(Json(json!({
        "data": {
            "online_users": online_users,
            "active_sessions": active_sessions,
            "active_conversations": active_conversations,
            "pending_notifications": pending_notifications,
            "recent_signups": recent_signups,
            "recent_posts": recent_posts,
            "pending_jobs": pending_jobs,
            "live_streamers": live_streamers,
        }
    })))
}
