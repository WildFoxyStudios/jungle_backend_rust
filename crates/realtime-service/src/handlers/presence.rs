use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::{Value, json};
use shared::auth::AppState;

use crate::hub::ConnectionHub;

type PresenceState = (AppState, ConnectionHub);

pub async fn online_users(State((_state, hub)): State<PresenceState>) -> Json<Value> {
    let users = hub.online_users();
    Json(json!({
        "data": {
            "online_count": hub.online_count(),
            "user_ids": users
        }
    }))
}

pub async fn is_online(
    State((_state, hub)): State<PresenceState>,
    Path(user_id): Path<i64>,
) -> Json<Value> {
    Json(json!({
        "data": {
            "user_id": user_id,
            "is_online": hub.is_online(user_id)
        }
    }))
}
