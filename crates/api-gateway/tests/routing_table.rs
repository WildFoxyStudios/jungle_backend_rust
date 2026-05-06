//! Integration test for the api-gateway prefix routing table.
//!
//! Validates the longest-prefix routing logic across the full surface area:
//! every service prefix declared in `routing::ServiceMap::from_env`. The
//! port assertions intentionally rely on the deterministic localhost defaults
//! (3001..=3013) because the test wipes the matching env vars before
//! constructing the map.

use api_gateway::routing::ServiceMap;

const ENV_KEYS: &[&str] = &[
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
];

fn fresh_map() -> ServiceMap {
    for key in ENV_KEYS {
        // SAFETY: the test crate is single-threaded.
        unsafe {
            std::env::remove_var(key);
        }
    }
    ServiceMap::from_env()
}

fn assert_route(map: &ServiceMap, path: &str, expected_port: &str) {
    let url = map
        .resolve(path)
        .unwrap_or_else(|| panic!("no route resolved for {path}"));
    assert!(
        url.ends_with(expected_port),
        "path {path} → {url} did not match expected port {expected_port}"
    );
}

#[test]
fn full_routing_table_resolves_all_known_prefixes() {
    let map = fresh_map();

    let cases: &[(&str, &str)] = &[
        // Auth (3001)
        ("/v1/auth/login", "3001"),
        ("/v1/oauth/google/callback", "3001"),
        ("/v1/translations", "3001"),
        ("/v1/config/public", "3001"),
        // Users (3002)
        ("/v1/users/123", "3002"),
        ("/v1/social/follow", "3002"),
        ("/v1/skills", "3002"),
        ("/v1/activities/recent", "3002"),
        ("/v1/reports/123", "3002"),
        ("/v1/mentions?q=jane", "3002"),
        ("/v1/points/history", "3002"),
        ("/v1/contact", "3002"),
        ("/v1/general/ping", "3002"),
        ("/v1/search/register", "3002"),
        // Posts / feed / etc. (3003)
        ("/v1/posts/feed", "3003"),
        ("/v1/comments/42", "3003"),
        ("/v1/feed", "3003"),
        ("/v1/reels/trending", "3003"),
        ("/v1/reels/explore", "3003"),
        ("/v1/reels/audio/trending", "3003"),
        ("/v1/reels/audio/search", "3003"),
        ("/v1/reels/user/jane", "3003"),
        ("/v1/reels/views", "3003"),
        ("/v1/hashtags/summer/reels", "3003"),
        ("/v1/search/everything", "3003"),
        ("/v1/ads/active", "3003"),
        ("/v1/hashtags/trending", "3003"),
        ("/v1/memories", "3003"),
        ("/v1/boosted/posts", "3003"),
        ("/v1/live/start", "3003"),
        // Media (3005)
        ("/v1/stories/today", "3005"),
        ("/v1/story-highlights/123", "3005"),
        ("/v1/albums/1", "3005"),
        ("/v1/media/upload", "3005"),
        ("/uploads/x/y", "3005"),
        // Messaging (3004)
        ("/v1/conversations", "3004"),
        ("/v1/messages/123", "3004"),
        ("/v1/broadcasts", "3004"),
        ("/v1/calls/start", "3004"),
        // Notifications (3006)
        ("/v1/notifications", "3006"),
        ("/v1/announcements", "3006"),
        ("/v1/newsletter/subscribe", "3006"),
        // Groups / Pages / Events (3007), with custom pages override (3008)
        ("/v1/pages/123", "3007"),
        ("/v1/pages/custom/about", "3008"),
        ("/v1/groups/abc", "3007"),
        ("/v1/events/upcoming", "3007"),
        ("/v1/boosted/pages", "3007"),
        // Content (3008)
        ("/v1/blogs/post", "3008"),
        ("/v1/forums/threads/1", "3008"),
        ("/v1/gifs/search", "3008"),
        ("/v1/movies/list", "3008"),
        ("/v1/games/list", "3008"),
        ("/v1/emojis", "3008"),
        // Commerce (3009)
        ("/v1/products/123", "3009"),
        ("/v1/orders/456", "3009"),
        ("/v1/cart", "3009"),
        ("/v1/jobs/listing", "3009"),
        ("/v1/fundings/active", "3009"),
        ("/v1/offers", "3009"),
        ("/v1/gifts", "3009"),
        ("/v1/stickers", "3009"),
        // Payments / Admin / AI / Realtime
        ("/v1/payments/checkout", "3011"),
        ("/v1/admin/users", "3010"),
        ("/v1/ai/chat", "3013"),
        ("/v1/presence", "3012"),
        ("/ws", "3012"),
    ];

    assert!(
        cases.len() >= 20,
        "routing table coverage should exceed 20 routes"
    );

    for (path, expected) in cases {
        assert_route(&map, path, expected);
    }
}

#[test]
fn longest_prefix_wins_search_register_over_search() {
    let map = fresh_map();
    assert_route(&map, "/v1/search/register", "3002");
    assert_route(&map, "/v1/search/foo", "3003");
}

#[test]
fn longest_prefix_wins_pages_custom_over_pages() {
    let map = fresh_map();
    assert_route(&map, "/v1/pages/custom/page", "3008");
    assert_route(&map, "/v1/pages/123", "3007");
}

#[test]
fn longest_prefix_wins_boosted_pages_over_boosted() {
    let map = fresh_map();
    assert_route(&map, "/v1/boosted/pages", "3007");
    assert_route(&map, "/v1/boosted/posts", "3003");
}

#[test]
fn unknown_prefix_returns_none() {
    let map = fresh_map();
    assert!(map.resolve("/v1/totally-not-a-route").is_none());
    assert!(map.resolve("/random").is_none());
}
