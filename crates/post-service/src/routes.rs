use axum::{
    routing::{delete, get, post, put},
    Router,
};
use shared::auth::AppState;
use crate::handlers;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Feed
        .route("/v1/feed", get(handlers::feed::get_feed))
        // Posts CRUD
        .route("/v1/posts", post(handlers::posts::create_post))
        .route("/v1/posts/{id}", get(handlers::posts::get_post))
        .route("/v1/posts/{id}", put(handlers::posts::update_post))
        .route("/v1/posts/{id}", delete(handlers::posts::delete_post))
        // Reactions
        .route("/v1/posts/{id}/react", post(handlers::reactions::react_to_post))
        .route("/v1/posts/{id}/react", delete(handlers::reactions::unreact_to_post))
        // Comments
        .route("/v1/posts/{id}/comments", get(handlers::comments::get_comments))
        .route("/v1/posts/{id}/comments", post(handlers::comments::create_comment))
        .route("/v1/comments/{id}", put(handlers::comments::update_comment))
        .route("/v1/comments/{id}", delete(handlers::comments::delete_comment))
        .route("/v1/comments/{id}/replies", get(handlers::comments::get_replies))
        .route("/v1/comments/{id}/react", post(handlers::reactions::react_to_comment))
        // Save/Hide
        .route("/v1/posts/{id}/save", post(handlers::posts::save_post))
        .route("/v1/posts/{id}/save", delete(handlers::posts::unsave_post))
        .route("/v1/posts/{id}/hide", post(handlers::posts::hide_post))
        // Reels
        .route("/v1/reels", get(handlers::reels::get_reels_feed))
        .route("/v1/reels", post(handlers::reels::create_reel))
        .route("/v1/reels/{id}", get(handlers::reels::get_reel))
        .route("/v1/reels/{id}", delete(handlers::reels::delete_reel))
        .route("/v1/reels/{id}/view", post(handlers::reels::view_reel))
        .route("/v1/reels/{id}/react", post(handlers::reels::react_to_reel))
        .route("/v1/reels/{id}/comments", get(handlers::reels::reel_comments))
        .route("/v1/reels/{id}/comments", post(handlers::reels::add_reel_comment))
        // Search
        .route("/v1/search", get(handlers::search::global_search))
        .route("/v1/search/recent", get(handlers::search::list_recent_searches))
        .route("/v1/search/recent", post(handlers::search::save_recent_search))
        .route("/v1/search/recent", delete(handlers::search::clear_recent_searches))
        // Post Sharing
        .route("/v1/posts/{id}/share", post(handlers::sharing::share_post))
        // Hashtags
        .route("/v1/hashtags/trending", get(handlers::hashtags::trending_hashtags))
        .route("/v1/hashtags/search", get(handlers::hashtags::search_hashtags))
        .route("/v1/hashtags/{tag}/posts", get(handlers::hashtags::posts_by_hashtag))
        // User Ads
        .route("/v1/ads", post(handlers::ads::create_ad))
        .route("/v1/ads/my", get(handlers::ads::my_ads))
        .route("/v1/ads/{id}/stats", get(handlers::ads::ad_stats))
        .route("/v1/ads/{id}", delete(handlers::ads::cancel_ad))
        .route("/v1/ads/{id}", put(handlers::extras::update_ad))
        .route("/v1/ads/{id}/click", post(handlers::sharing::ad_click))
        .route("/v1/ads/{id}/view", post(handlers::ads::record_ad_view))
        .route("/v1/ads/estimated-audience", get(handlers::ads::get_estimated_audience))
        // Polls
        .route("/v1/posts/{id}/poll/vote", post(handlers::extras::vote_poll))
        // Pin / Boost / Report
        .route("/v1/posts/{id}/pin", post(handlers::extras::pin_post))
        .route("/v1/posts/{id}/pin", delete(handlers::extras::unpin_post))
        .route("/v1/posts/{id}/boost", post(handlers::extras::boost_post))
        .route("/v1/posts/{id}/report", post(handlers::extras::report_post))
        // Explore & Memories
        .route("/v1/feed/explore", get(handlers::extras::explore_feed))
        .route("/v1/memories", get(handlers::extras::get_memories))
        // Reply convenience route
        .route("/v1/comments/{id}/replies", post(handlers::extras::create_reply))
        // Boosted content
        .route("/v1/boosted/posts", get(handlers::extras::my_boosted_posts))
        // Trending
        .route("/v1/posts/most-liked", get(handlers::extras::most_liked_posts))
        .route("/v1/posts/most-watched", get(handlers::extras::most_watched_posts))
        // Colored post templates & reaction types (public)
        .route("/v1/posts/colored-templates", get(handlers::extras::list_colored_templates))
        .route("/v1/posts/reaction-types", get(handlers::extras::list_reaction_types))
        // Albums
        .route("/v1/albums", post(handlers::albums::create_album))
        .route("/v1/albums/{id}/images", get(handlers::albums::list_album_images))
        .route("/v1/albums/{id}/images", post(handlers::albums::add_album_images))
        .route("/v1/albums/{album_id}/images/{image_id}", delete(handlers::albums::delete_album_image))
        .route("/v1/users/{user_id}/albums", get(handlers::albums::list_user_albums))
        // Live Streaming
        .route("/v1/live/start", post(handlers::live::start_live))
        .route("/v1/live/stop", post(handlers::live::stop_live))
        .route("/v1/live/active", get(handlers::live::active_lives))
        .route("/v1/live/friends", get(handlers::live::live_friends))
        .route("/v1/live/{id}/comment", post(handlers::live::live_comment))
        .route("/v1/live/{id}/react", post(handlers::live::live_react))
        // Health
        .route("/health", get(handlers::health::health_check))
        .with_state(state)
}
