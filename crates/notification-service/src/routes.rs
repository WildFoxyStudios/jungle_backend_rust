use axum::{
    Router,
    routing::{delete, get, post},
};
use shared::auth::AppState;

use crate::handlers;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Notifications CRUD
        .route(
            "/v1/notifications",
            get(handlers::notifications::list_notifications),
        )
        .route(
            "/v1/notifications/unread-count",
            get(handlers::notifications::unread_count),
        )
        .route(
            "/v1/notifications/read-all",
            post(handlers::notifications::mark_all_read),
        )
        .route(
            "/v1/notifications/{id}/read",
            post(handlers::notifications::mark_read),
        )
        .route(
            "/v1/notifications/{id}",
            delete(handlers::notifications::delete_notification),
        )
        .route(
            "/v1/notifications/clear",
            delete(handlers::notifications::clear_all),
        )
        // Preferences
        .route(
            "/v1/notifications/preferences",
            get(handlers::preferences::get_preferences)
                .put(handlers::preferences::update_preferences),
        )
        // Push tokens (FCM/APNS — native mobile)
        .route(
            "/v1/notifications/push-tokens",
            post(handlers::push_tokens::register_push_token)
                .get(handlers::push_tokens::list_my_tokens),
        )
        .route(
            "/v1/notifications/push-tokens/{token}",
            delete(handlers::push_tokens::unregister_push_token),
        )
        // VAPID Web Push subscriptions (W3C Push API — desktop browsers)
        .route(
            "/v1/notifications/web-push/subscribe",
            post(handlers::web_push_subscriptions::subscribe),
        )
        .route(
            "/v1/notifications/web-push/unsubscribe",
            post(handlers::web_push_subscriptions::unsubscribe),
        )
        .route(
            "/v1/notifications/web-push/subscriptions",
            get(handlers::web_push_subscriptions::list_my),
        )
        .route(
            "/v1/notifications/web-push/subscriptions/{id}",
            delete(handlers::web_push_subscriptions::delete_one),
        )
        .route(
            "/v1/notifications/web-push/public-key",
            get(handlers::web_push_subscriptions::vapid_public_key),
        )
        // Announcements (user-facing)
        .route(
            "/v1/announcements",
            get(handlers::announcements::list_active_announcements),
        )
        .route(
            "/v1/announcements/{id}/dismiss",
            post(handlers::announcements::dismiss_announcement),
        )
        // Newsletter (public)
        .route(
            "/v1/newsletter/subscribe",
            post(handlers::newsletter::subscribe),
        )
        .route(
            "/v1/newsletter/unsubscribe",
            post(handlers::newsletter::unsubscribe),
        )
        // Health
        .route("/health", get(handlers::health::health_check))
        .with_state(state)
}
