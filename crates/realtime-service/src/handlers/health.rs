use axum::{Json, extract::State};
use serde_json::{Value, json};
use shared::auth::AppState;

use crate::hub::ConnectionHub;

type HealthState = (AppState, ConnectionHub);

pub async fn health_check(State((state, hub)): State<HealthState>) -> Json<Value> {
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
        "service": "realtime-service",
        "online_connections": hub.online_count(),
        "checks": { "database": db_ok, "redis": redis_ok }
    }))
}
