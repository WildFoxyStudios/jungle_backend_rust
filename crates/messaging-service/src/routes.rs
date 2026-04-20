use axum::{
    routing::{delete, get, post, put},
    Router,
};
use shared::auth::AppState;

use crate::handlers;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Conversations
        .route("/v1/conversations", get(handlers::conversations::list_conversations).post(handlers::conversations::create_conversation))
        .route("/v1/conversations/group", post(handlers::conversations::create_group_conversation))
        .route("/v1/conversations/pinned", get(handlers::conversations::list_pinned))
        .route("/v1/conversations/archived", get(handlers::conversations::list_archived))
        .route("/v1/conversations/{id}", get(handlers::conversations::get_conversation).delete(handlers::conversations::delete_conversation))
        .route("/v1/conversations/{id}/color", put(handlers::conversations::update_color))
        .route("/v1/conversations/{id}/pin", post(handlers::conversations::pin_conversation).delete(handlers::conversations::unpin_conversation))
        .route("/v1/conversations/{id}/archive", post(handlers::conversations::archive_conversation).delete(handlers::conversations::unarchive_conversation))
        .route("/v1/conversations/{id}/read", post(handlers::conversations::mark_read))
        .route("/v1/conversations/mark-all-read", post(handlers::conversations::mark_all_read))
        .route("/v1/conversations/group/{id}", put(handlers::conversations::update_group))
        // Messages
        .route("/v1/conversations/{id}/messages", get(handlers::messages::list_messages).post(handlers::messages::send_message))
        .route("/v1/conversations/{id}/typing", post(handlers::messages::typing_indicator))
        .route("/v1/conversations/{id}/pinned-messages", get(handlers::messages::list_pinned_messages))
        .route("/v1/conversations/{id}/search", get(handlers::conversations::search_messages))
        .route("/v1/conversations/{id}/media", get(handlers::conversations::list_conversation_media))
        .route("/v1/messages/favorites", get(handlers::messages::list_favorite_messages))
        .route("/v1/messages/{id}", delete(handlers::messages::delete_message))
        .route("/v1/messages/{id}/favorite", post(handlers::messages::toggle_favorite))
        .route("/v1/messages/{id}/pin", post(handlers::messages::pin_message).delete(handlers::messages::unpin_message))
        .route("/v1/messages/{id}/forward", post(handlers::messages::forward_message))
        .route("/v1/messages/{id}/react", post(handlers::messages::react_to_message))
        .route("/v1/messages/{id}/listened", post(handlers::messages::mark_listened))
        // Broadcasts
        .route("/v1/broadcasts", get(handlers::broadcasts::list_broadcasts).post(handlers::broadcasts::create_broadcast))
        .route("/v1/broadcasts/{id}", put(handlers::broadcasts::update_broadcast).delete(handlers::broadcasts::delete_broadcast))
        .route("/v1/broadcasts/{id}/members", get(handlers::broadcasts::list_members).post(handlers::broadcasts::add_members))
        .route("/v1/broadcasts/{id}/members/{user_id}", delete(handlers::broadcasts::remove_member))
        .route("/v1/broadcasts/{id}/send", post(handlers::broadcasts::send_broadcast))
        // Calls
        .route("/v1/calls", get(handlers::calls::list_calls).post(handlers::calls::create_call))
        .route("/v1/calls/agora-token", post(handlers::calls::generate_agora_token))
        .route("/v1/calls/viewer-token", post(handlers::calls::generate_viewer_token))
        .route("/v1/calls/{id}", get(handlers::calls::get_call))
        .route("/v1/calls/{id}/status", put(handlers::calls::update_call_status))
        // Health
        .route("/health", get(handlers::health::health_check))
        .with_state(state)
}
