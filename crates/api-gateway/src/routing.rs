use axum::{routing::any, Router};
use std::collections::HashMap;

use crate::proxy::{self, GatewayState};

/// Maps path prefixes to upstream service base URLs.
pub struct ServiceMap {
    routes: Vec<(String, String)>,
}

impl ServiceMap {
    pub fn from_env() -> Self {
        let mut routes = Vec::new();

        let defaults: Vec<(&str, &str)> = vec![
            // Auth + OAuth + Public
            ("/v1/auth", "AUTH_SERVICE_URL"),
            ("/v1/oauth", "AUTH_SERVICE_URL"),
            ("/v1/translations", "AUTH_SERVICE_URL"),
            ("/v1/config/public", "AUTH_SERVICE_URL"),
            // Users
            ("/v1/users", "USER_SERVICE_URL"),
            ("/v1/social", "USER_SERVICE_URL"),
            // Users extras
            ("/v1/skills", "USER_SERVICE_URL"),
            // Posts / Feed / Reels / Search / Ads / Comments / Live
            ("/v1/posts", "POST_SERVICE_URL"),
            ("/v1/comments", "POST_SERVICE_URL"),
            ("/v1/feed", "POST_SERVICE_URL"),
            ("/v1/reels", "POST_SERVICE_URL"),
            ("/v1/search", "POST_SERVICE_URL"),
            ("/v1/ads", "POST_SERVICE_URL"),
            ("/v1/hashtags", "POST_SERVICE_URL"),
            ("/v1/memories", "POST_SERVICE_URL"),
            ("/v1/boosted/pages", "GROUP_PAGE_SERVICE_URL"),
            ("/v1/boosted", "POST_SERVICE_URL"),
            ("/v1/live", "POST_SERVICE_URL"),
            // Media & Stories & Uploads
            ("/v1/stories", "MEDIA_SERVICE_URL"),
            ("/v1/media", "MEDIA_SERVICE_URL"),
            ("/uploads", "MEDIA_SERVICE_URL"),
            // Messaging
            ("/v1/conversations", "MESSAGING_SERVICE_URL"),
            ("/v1/messages", "MESSAGING_SERVICE_URL"),
            ("/v1/broadcasts", "MESSAGING_SERVICE_URL"),
            ("/v1/calls", "MESSAGING_SERVICE_URL"),
            // Notifications + Announcements
            ("/v1/notifications", "NOTIFICATION_SERVICE_URL"),
            ("/v1/announcements", "NOTIFICATION_SERVICE_URL"),
            // Groups / Pages / Events
            ("/v1/pages/custom", "CONTENT_SERVICE_URL"),
            ("/v1/pages", "GROUP_PAGE_SERVICE_URL"),
            ("/v1/groups", "GROUP_PAGE_SERVICE_URL"),
            ("/v1/events", "GROUP_PAGE_SERVICE_URL"),
            // Content
            ("/v1/blogs", "CONTENT_SERVICE_URL"),
            ("/v1/forums", "CONTENT_SERVICE_URL"),
            ("/v1/movies", "CONTENT_SERVICE_URL"),
            ("/v1/games", "CONTENT_SERVICE_URL"),
            ("/v1/emojis", "CONTENT_SERVICE_URL"),
            // Commerce
            ("/v1/products", "COMMERCE_SERVICE_URL"),
            ("/v1/orders", "COMMERCE_SERVICE_URL"),
            ("/v1/jobs", "COMMERCE_SERVICE_URL"),
            ("/v1/fundings", "COMMERCE_SERVICE_URL"),
            ("/v1/offers", "COMMERCE_SERVICE_URL"),
            ("/v1/gifts", "COMMERCE_SERVICE_URL"),
            ("/v1/stickers", "COMMERCE_SERVICE_URL"),
            // Newsletter
            ("/v1/newsletter", "NOTIFICATION_SERVICE_URL"),
            // Payments
            ("/v1/payments", "PAYMENT_SERVICE_URL"),
            // Admin
            ("/v1/admin", "ADMIN_SERVICE_URL"),
            // AI
            ("/v1/ai", "AI_SERVICE_URL"),
            // Realtime
            ("/v1/presence", "REALTIME_SERVICE_URL"),
            ("/ws", "REALTIME_SERVICE_URL"),
        ];

        let port_defaults: HashMap<&str, &str> = [
            ("AUTH_SERVICE_URL", "http://127.0.0.1:3001"),
            ("USER_SERVICE_URL", "http://127.0.0.1:3002"),
            ("POST_SERVICE_URL", "http://127.0.0.1:3003"),
            ("MESSAGING_SERVICE_URL", "http://127.0.0.1:3004"),
            ("MEDIA_SERVICE_URL", "http://127.0.0.1:3005"),
            ("NOTIFICATION_SERVICE_URL", "http://127.0.0.1:3006"),
            ("GROUP_PAGE_SERVICE_URL", "http://127.0.0.1:3007"),
            ("CONTENT_SERVICE_URL", "http://127.0.0.1:3008"),
            ("COMMERCE_SERVICE_URL", "http://127.0.0.1:3009"),
            ("ADMIN_SERVICE_URL", "http://127.0.0.1:3010"),
            ("PAYMENT_SERVICE_URL", "http://127.0.0.1:3011"),
            ("REALTIME_SERVICE_URL", "http://127.0.0.1:3012"),
            ("AI_SERVICE_URL", "http://127.0.0.1:3013"),
        ]
        .into();

        for (prefix, env_key) in defaults {
            let base_url = std::env::var(env_key)
                .unwrap_or_else(|_| port_defaults.get(env_key).unwrap_or(&"").to_string());
            if !base_url.is_empty() {
                routes.push((prefix.to_string(), base_url));
            }
        }

        // Sort longest prefix first for greedy matching
        routes.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        Self { routes }
    }

    pub fn resolve(&self, path: &str) -> Option<&str> {
        for (prefix, url) in &self.routes {
            if path.starts_with(prefix) {
                return Some(url);
            }
        }
        None
    }
}

pub fn create_router(state: GatewayState) -> Router {
    Router::new()
        .route("/ws", axum::routing::get(crate::ws_proxy::ws_proxy_handler))
        .route("/health", axum::routing::get(gateway_health))
        .fallback(any(proxy::proxy_request))
        .with_state(state)
}

async fn gateway_health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "api-gateway"
    }))
}
