use axum::{
    routing::{delete, get, post, put},
    Router,
};
use shared::auth::AppState;

use crate::handlers;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Notifications CRUD
        .route("/v1/notifications", get(handlers::notifications::list_notifications))
        .route("/v1/notifications/unread-count", get(handlers::notifications::unread_count))
        .route("/v1/notifications/read-all", post(handlers::notifications::mark_all_read))
        .route("/v1/notifications/{id}/read", post(handlers::notifications::mark_read))
        .route("/v1/notifications/{id}", delete(handlers::notifications::delete_notification))
        .route("/v1/notifications/clear", delete(handlers::notifications::clear_all))
        // Preferences
        .route("/v1/notifications/preferences", get(handlers::preferences::get_preferences))
        .route("/v1/notifications/preferences", put(handlers::preferences::update_preferences))
        // Push tokens
        .route("/v1/notifications/push-tokens", post(handlers::push_tokens::register_push_token))
        .route("/v1/notifications/push-tokens", get(handlers::push_tokens::list_my_tokens))
        .route("/v1/notifications/push-tokens/{token}", delete(handlers::push_tokens::unregister_push_token))
        // Announcements (user-facing)
        .route("/v1/announcements", get(handlers::announcements::list_active_announcements))
        .route("/v1/announcements/{id}/dismiss", post(handlers::announcements::dismiss_announcement))
        // Newsletter (public)
        .route("/v1/newsletter/subscribe", post(handlers::newsletter::subscribe))
        .route("/v1/newsletter/unsubscribe", post(handlers::newsletter::unsubscribe))
        // Health
        .route("/health", get(handlers::health::health_check))
        .with_state(state)
}
