use axum::{extract::State, Json};
use serde_json::{json, Value};
use shared::auth::AppState;

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
        "service": "group-page-service",
        "checks": { "database": db_ok, "redis": redis_ok }
    }))
}
