use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::Value;
use shared::auth::AppState;

use crate::hub::{ConnectionHub, WsMessage};

type RealtimeState = (AppState, ConnectionHub);

/// POST /internal/send/{user_id}
/// Internal endpoint used by notification-service dispatcher to push messages
/// directly to a connected WebSocket client.
pub async fn send_to_user(
    State((_state, hub)): State<RealtimeState>,
    Path(user_id): Path<i64>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    let event = payload
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("notification")
        .to_string();

    let data = payload.get("data").cloned().unwrap_or(payload.clone());

    let msg = WsMessage { event, data };
    hub.send_to_user(user_id, msg);

    Json(serde_json::json!({ "ok": true }))
}

/// POST /internal/broadcast
/// Internal endpoint to broadcast a message to multiple users.
pub async fn broadcast_to_users(
    State((_state, hub)): State<RealtimeState>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    let event = payload
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("broadcast")
        .to_string();

    let data = payload.get("data").cloned().unwrap_or_default();
    let user_ids: Vec<i64> = payload
        .get("user_ids")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let msg = WsMessage { event, data };
    hub.send_to_users(&user_ids, msg);

    Json(serde_json::json!({ "ok": true, "sent_to": user_ids.len() }))
}
