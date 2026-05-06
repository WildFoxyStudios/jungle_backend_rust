use crate::handlers;
use axum::{
    Router,
    routing::{delete, get, post, put},
};
use shared::auth::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Feed
        .route("/v1/feed", get(handlers::feed::get_feed))
        // Posts CRUD
        .route("/v1/posts", post(handlers::posts::create_post))
        .route(
            "/v1/posts/{id}",
            get(handlers::posts::get_post)
                .put(handlers::posts::update_post)
                .delete(handlers::posts::delete_post),
        )
        // Reactions
        .route(
            "/v1/posts/{id}/react",
            post(handlers::reactions::react_to_post).delete(handlers::reactions::unreact_to_post),
        )
        // Comments
        .route(
            "/v1/posts/{id}/comments",
            get(handlers::comments::get_comments).post(handlers::comments::create_comment),
        )
        .route(
            "/v1/comments/{id}",
            put(handlers::comments::update_comment).delete(handlers::comments::delete_comment),
        )
        .route(
            "/v1/comments/{id}/replies",
            get(handlers::comments::get_replies).post(handlers::extras::create_reply),
        )
        .route(
            "/v1/comments/{id}/react",
            post(handlers::reactions::react_to_comment)
                .delete(handlers::reactions::unreact_to_comment),
        )
        // Co-Authors
        .route(
            "/v1/posts/{id}/coauthors",
            get(handlers::posts::list_coauthors).post(handlers::posts::invite_coauthor),
        )
        .route(
            "/v1/posts/{id}/coauthors/accept",
            post(handlers::posts::accept_coauthor_invite),
        )
        .route(
            "/v1/posts/{pid}/coauthors/{uid}",
            delete(handlers::posts::remove_coauthor),
        )
        // Translation
        .route(
            "/v1/posts/{id}/translate",
            post(handlers::posts::translate_post),
        )
        // Save/Hide
        .route("/v1/posts/saved", get(handlers::posts::get_saved_posts))
        .route(
            "/v1/posts/{id}/save",
            post(handlers::posts::save_post).delete(handlers::posts::unsave_post),
        )
        .route("/v1/posts/{id}/hide", post(handlers::posts::hide_post))
        // Reels
        .route(
            "/v1/reels",
            get(handlers::reels::get_reels_feed).post(handlers::reels::create_reel),
        )
        .route(
            "/v1/reels/trending",
            get(handlers::reels::reels_trending),
        )
        .route(
            "/v1/reels/user/{username}",
            get(handlers::reels::reels_by_username),
        )
        .route(
            "/v1/reels/views",
            delete(handlers::reels::clear_reel_views),
        )
        .route("/v1/reels/explore", get(handlers::reels::reels_explore))
        .route(
            "/v1/reels/audio/trending",
            get(handlers::reels::reels_audio_trending),
        )
        .route(
            "/v1/reels/audio/search",
            get(handlers::reels::reels_audio_search),
        )
        .route(
            "/v1/reels/audio",
            post(handlers::reels::create_reel_audio_track),
        )
        .route(
            "/v1/reels/{id}",
            get(handlers::reels::get_reel).delete(handlers::reels::delete_reel),
        )
        .route("/v1/reels/{id}/view", post(handlers::reels::view_reel))
        .route(
            "/v1/reels/{id}/insights",
            post(handlers::reels::reel_insight_sample),
        )
        .route("/v1/reels/{id}/react", post(handlers::reels::react_to_reel))
        .route(
            "/v1/reels/{id}/share",
            post(handlers::reels::share_reel),
        )
        .route(
            "/v1/reels/{id}/comments",
            get(handlers::reels::reel_comments).post(handlers::reels::add_reel_comment),
        )
        .route(
            "/v1/reels/{reel_id}/comments/{comment_id}",
            delete(handlers::reels::delete_reel_comment),
        )
        // Reel Bonuses (Creator Monetization)
        .route("/v1/reels/bonuses", get(handlers::reels::get_reel_bonuses))
        .route(
            "/v1/reels/bonuses/history",
            get(handlers::reels::get_bonus_history),
        )
        // Search
        .route("/v1/search", get(handlers::search::global_search))
        .route(
            "/v1/search/recent",
            get(handlers::search::list_recent_searches)
                .post(handlers::search::save_recent_search)
                .delete(handlers::search::clear_recent_searches),
        )
        // Post Sharing
        .route("/v1/posts/{id}/share", post(handlers::sharing::share_post))
        // Hashtags
        .route(
            "/v1/hashtags/trending",
            get(handlers::hashtags::trending_hashtags),
        )
        .route(
            "/v1/hashtags/search",
            get(handlers::hashtags::search_hashtags),
        )
        .route(
            "/v1/hashtags/{tag}/posts",
            get(handlers::hashtags::posts_by_hashtag),
        )
        .route(
            "/v1/hashtags/{tag}/reels",
            get(handlers::hashtags::reels_by_hashtag),
        )
        // User Ads
        .route("/v1/ads", post(handlers::ads::create_ad))
        .route("/v1/ads/my", get(handlers::ads::my_ads))
        .route("/v1/ads/{id}/stats", get(handlers::ads::ad_stats))
        .route(
            "/v1/ads/{id}",
            delete(handlers::ads::cancel_ad).put(handlers::extras::update_ad),
        )
        .route("/v1/ads/{id}/click", post(handlers::sharing::ad_click))
        .route("/v1/ads/{id}/view", post(handlers::ads::record_ad_view))
        .route(
            "/v1/ads/estimated-audience",
            get(handlers::ads::get_estimated_audience),
        )
        // Polls
        .route(
            "/v1/posts/{id}/poll/vote",
            post(handlers::extras::vote_poll),
        )
        // Pin / Boost / Report
        .route(
            "/v1/posts/{id}/pin",
            post(handlers::extras::pin_post).delete(handlers::extras::unpin_post),
        )
        .route(
            "/v1/posts/{id}/boost",
            post(handlers::extras::boost_post).delete(handlers::extras::unboost_post),
        )
        .route("/v1/posts/{id}/report", post(handlers::extras::report_post))
        // Explore & Memories
        .route("/v1/feed/explore", get(handlers::extras::explore_feed))
        .route("/v1/memories", get(handlers::extras::get_memories))
        // Boosted content
        .route("/v1/boosted/posts", get(handlers::extras::my_boosted_posts))
        // Trending
        .route(
            "/v1/posts/most-liked",
            get(handlers::extras::most_liked_posts),
        )
        .route(
            "/v1/posts/most-watched",
            get(handlers::extras::most_watched_posts),
        )
        // Colored post templates & reaction types (public)
        .route(
            "/v1/posts/colored-templates",
            get(handlers::extras::list_colored_templates),
        )
        .route(
            "/v1/posts/reaction-types",
            get(handlers::extras::list_reaction_types),
        )
        // Albums
        .route("/v1/albums", post(handlers::albums::create_album))
        .route(
            "/v1/albums/{id}/images",
            get(handlers::albums::list_album_images).post(handlers::albums::add_album_images),
        )
        .route(
            "/v1/albums/{album_id}/images/{image_id}",
            delete(handlers::albums::delete_album_image),
        )
        .route(
            "/v1/users/{user_id}/albums",
            get(handlers::albums::list_user_albums),
        )
        // Live Streaming
        .route("/v1/live/start", post(handlers::live::start_live))
        .route("/v1/live/stop", post(handlers::live::stop_live))
        .route("/v1/live/active", get(handlers::live::active_lives))
        .route("/v1/live/friends", get(handlers::live::live_friends))
        .route("/v1/live/{id}", get(handlers::live::get_live))
        .route("/v1/live/{id}/comment", post(handlers::live::live_comment))
        .route("/v1/live/{id}/react", post(handlers::live::live_react))
        .route(
            "/v1/live/{id}/vod",
            get(handlers::live::live_vod).patch(handlers::live::live_vod_publish),
        )
        // Millicast (live broadcasting via Dolby Millicast SDK)
        .route(
            "/v1/live/millicast/publish-token",
            post(handlers::live::millicast_publish_token),
        )
        .route(
            "/v1/live/millicast/subscribe-token",
            post(handlers::live::millicast_subscribe_token),
        )
        // ── Phase 16: Co-hosts, Polls, VODs ──
        .route("/v1/live/{id}/cohost", post(handlers::live::invite_cohost))
        .route("/v1/live/{id}/cohost/accept", post(handlers::live::accept_cohost))
        .route("/v1/live/{lid}/cohost/{uid}", delete(handlers::live::remove_cohost))
        .route("/v1/live/{id}/polls", get(handlers::live::get_live_polls).post(handlers::live::create_live_poll))
        .route("/v1/live/{lid}/polls/{pid}/vote", post(handlers::live::vote_live_poll))
        .route("/v1/vods", get(handlers::live::list_vods))
        // ── XHR-equivalent endpoints (Batch 4) ──
        .route(
            "/v1/posts/preview-url",
            post(handlers::post_xhr::preview_url),
        )
        .route(
            "/v1/posts/audio",
            post(handlers::post_xhr::create_audio_post),
        )
        .route(
            "/v1/posts/open-to-work",
            get(handlers::extras::open_to_work_feed),
        )
        .route(
            "/v1/posts/{post_id}/media/{media_id}",
            delete(handlers::post_xhr::delete_post_media),
        )
        .route(
            "/v1/posts/{id}/comments-status",
            put(handlers::post_xhr::set_comments_status),
        )
        .route(
            "/v1/posts/{id}/mark-sold",
            post(handlers::post_xhr::mark_sold),
        )
        .route(
            "/v1/posts/{id}/notify-followers",
            post(handlers::post_xhr::notify_followers),
        )
        .route(
            "/v1/posts/{id}/video-view",
            post(handlers::post_xhr::video_view),
        )
        .route(
            "/v1/posts/{id}/wonder",
            post(handlers::post_xhr::toggle_wonder),
        )
        .route(
            "/v1/posts/{id}/reactors",
            get(handlers::post_xhr::list_reactors),
        )
        // Plan §3.18 SA7 — who viewed a given post. `POST /view` is idempotent
        // and only bumps `posts.post_views` the first time a user sees it.
        .route(
            "/v1/posts/{id}/view",
            post(handlers::post_viewers::record_view),
        )
        .route(
            "/v1/posts/{id}/viewers",
            get(handlers::post_viewers::list_viewers),
        )
        .route(
            "/v1/posts/{id}/impression",
            post(handlers::post_viewers::record_impression),
        )
        // Health
        .route("/health", get(handlers::health::health_check))
        .with_state(state)
}
