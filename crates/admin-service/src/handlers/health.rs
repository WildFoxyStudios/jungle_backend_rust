use axum::{extract::State, Json};
use serde_json::{json, Value};
use shared::{auth::{AppState, AuthUser}, errors::ApiError};

pub async fn admin_health(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    let redis_ok = redis::cmd("PING")
        .query_async::<String>(&mut state.redis.clone())
        .await
        .is_ok();

    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE deleted_at IS NULL")
        .fetch_one(&state.db).await.unwrap_or(0);

    let total_posts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE deleted_at IS NULL")
        .fetch_one(&state.db).await.unwrap_or(0);

    let active_today: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE last_seen > NOW() - INTERVAL '24 hours' AND deleted_at IS NULL"
    ).fetch_one(&state.db).await.unwrap_or(0);

    let disk_usage = "N/A".to_string();

    Ok(Json(json!({
        "status": if db_ok && redis_ok { "healthy" } else { "degraded" },
        "checks": { "database": db_ok, "redis": redis_ok },
        "stats": {
            "total_users": total_users,
            "total_posts": total_posts,
            "active_today": active_today,
            "disk_usage": disk_usage
        },
        "uptime": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    })))
}

pub async fn health_check(State(state): State<AppState>) -> Json<Value> {
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    let redis_ok = redis::cmd("PING")
        .query_async::<String>(&mut state.redis.clone())
        .await
        .is_ok();

    Json(json!({
        "status": if db_ok && redis_ok { "healthy" } else { "degraded" },
        "service": "admin-service",
        "checks": { "database": db_ok, "redis": redis_ok }
    }))
}
