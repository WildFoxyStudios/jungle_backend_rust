use crate::handlers;
use crate::hub::ConnectionHub;
use axum::{
    Router,
    routing::{get, post},
};
use shared::auth::AppState;

pub fn create_router_with_hub(state: AppState, hub: ConnectionHub) -> Router {
    Router::new()
        .route("/ws", get(handlers::ws::ws_handler))
        .route("/v1/presence/online", get(handlers::presence::online_users))
        .route("/v1/presence/{user_id}", get(handlers::presence::is_online))
        .route(
            "/internal/send/{user_id}",
            post(handlers::internal::send_to_user),
        )
        .route(
            "/internal/broadcast",
            post(handlers::internal::broadcast_to_users),
        )
        .route("/health", get(handlers::health::health_check))
        .with_state((state, hub))
}
