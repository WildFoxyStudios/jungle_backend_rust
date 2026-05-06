use axum::{Router, routing::any};
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
            ("/v1/activities", "USER_SERVICE_URL"),
            ("/v1/reports", "USER_SERVICE_URL"),
            ("/v1/mentions", "USER_SERVICE_URL"),
            ("/v1/points", "USER_SERVICE_URL"),
            ("/v1/contact", "USER_SERVICE_URL"),
            ("/v1/general", "USER_SERVICE_URL"),
            // Note: "/v1/search/register" is deliberately LONGER than "/v1/search"
            // so the longest-prefix sort below (line ~109) wins and routes it to
            // USER_SERVICE_URL instead of POST_SERVICE_URL.
            ("/v1/search/register", "USER_SERVICE_URL"),
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
            ("/v1/live-native", "LIVE_SERVICE_URL"),
            // Media & Stories & Uploads
            ("/v1/stories", "MEDIA_SERVICE_URL"),
            ("/v1/story-highlights", "MEDIA_SERVICE_URL"),
            ("/v1/albums", "MEDIA_SERVICE_URL"),
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
            ("/v1/gifs", "CONTENT_SERVICE_URL"),
            ("/v1/movies", "CONTENT_SERVICE_URL"),
            ("/v1/games", "CONTENT_SERVICE_URL"),
            ("/v1/emojis", "CONTENT_SERVICE_URL"),
            ("/v1/lookups", "CONTENT_SERVICE_URL"),
            ("/v1/countries", "CONTENT_SERVICE_URL"),
            // Commerce
            ("/v1/products", "COMMERCE_SERVICE_URL"),
            ("/v1/orders", "COMMERCE_SERVICE_URL"),
            ("/v1/cart", "COMMERCE_SERVICE_URL"),
            ("/v1/jobs", "COMMERCE_SERVICE_URL"),
            ("/v1/fundings", "COMMERCE_SERVICE_URL"),
            ("/v1/offers", "COMMERCE_SERVICE_URL"),
            ("/v1/gifts", "COMMERCE_SERVICE_URL"),
            ("/v1/stickers", "COMMERCE_SERVICE_URL"),
            // Commerce sub-routes under /v1/users/me (must be longer than "/v1/users")
            ("/v1/users/me/saved-products", "COMMERCE_SERVICE_URL"),
            ("/v1/users/me/saved-jobs", "COMMERCE_SERVICE_URL"),
            ("/v1/users/me/job-alerts", "COMMERCE_SERVICE_URL"),
            ("/v1/users/me/resume", "COMMERCE_SERVICE_URL"),
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
            ("/ws/live-native", "LIVE_SERVICE_URL"),
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
            ("LIVE_SERVICE_URL", "http://127.0.0.1:3014"),
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
        .route(
            "/ws/live-native",
            axum::routing::get(crate::ws_proxy::ws_live_native_proxy_handler),
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    fn map_with_env() -> ServiceMap {
        // Ensure deterministic defaults even in CI where env vars may be set.
        for key in [
            "AUTH_SERVICE_URL",
            "USER_SERVICE_URL",
            "POST_SERVICE_URL",
            "MESSAGING_SERVICE_URL",
            "MEDIA_SERVICE_URL",
            "NOTIFICATION_SERVICE_URL",
            "GROUP_PAGE_SERVICE_URL",
            "CONTENT_SERVICE_URL",
            "COMMERCE_SERVICE_URL",
            "ADMIN_SERVICE_URL",
            "PAYMENT_SERVICE_URL",
            "REALTIME_SERVICE_URL",
            "AI_SERVICE_URL",
            "LIVE_SERVICE_URL",
        ] {
            // SAFETY: tests are single-threaded within this test module.
            unsafe {
                std::env::remove_var(key);
            }
        }
        ServiceMap::from_env()
    }

    fn assert_route(sm: &ServiceMap, path: &str, expected_port: &str) {
        let url = sm
            .resolve(path)
            .unwrap_or_else(|| panic!("no route for {path}"));
        assert!(
            url.ends_with(expected_port),
            "path {path} → {url} did not match expected port {expected_port}"
        );
    }

    #[test]
    fn search_register_overrides_search_prefix() {
        let sm = map_with_env();
        // /v1/search goes to post-service (3003) but /v1/search/register
        // must beat it because it is longer and is registered as USER_SERVICE.
        assert_route(&sm, "/v1/search/foo", "3003");
        assert_route(&sm, "/v1/search/register", "3002");
    }

    #[test]
    fn user_extras_route_to_user_service() {
        let sm = map_with_env();
        for path in [
            "/v1/activities/recent",
            "/v1/reports/123",
            "/v1/mentions?q=jane",
            "/v1/points/history",
            "/v1/contact",
            "/v1/general/ping",
        ] {
            assert_route(&sm, path, "3002");
        }
    }

    #[test]
    fn albums_and_cart() {
        let sm = map_with_env();
        assert_route(&sm, "/v1/albums/1", "3005");
        assert_route(&sm, "/v1/cart", "3009");
    }

    #[test]
    fn baseline_prefixes_still_resolve() {
        let sm = map_with_env();
        assert_route(&sm, "/v1/auth/login", "3001");
        assert_route(&sm, "/v1/users/123", "3002");
        assert_route(&sm, "/v1/posts/feed", "3003");
        assert_route(&sm, "/v1/conversations", "3004");
        assert_route(&sm, "/v1/media/upload", "3005");
        assert_route(&sm, "/v1/notifications", "3006");
        assert_route(&sm, "/v1/admin/users", "3010");
        assert_route(&sm, "/v1/payments/checkout", "3011");
        assert_route(&sm, "/ws", "3012");
        assert_route(&sm, "/v1/ai/chat", "3013");
        assert_route(&sm, "/v1/live-native/rooms", "3014");
        assert_route(&sm, "/ws/live-native", "3014");
    }
}
