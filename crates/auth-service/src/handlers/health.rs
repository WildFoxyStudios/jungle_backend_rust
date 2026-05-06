use axum::{Json, extract::State};
use shared::auth::AppState;

pub async fn health_check(State(state): State<AppState>) -> Json<serde_json::Value> {
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    let redis_ok = redis::cmd("PING")
        .query_async::<String>(&mut state.redis.clone())
        .await
        .is_ok();

    let status = if db_ok && redis_ok {
        "healthy"
    } else {
        "degraded"
    };

    Json(serde_json::json!({
        "status": status,
        "services": {
            "database": db_ok,
            "redis": redis_ok,
        },
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
