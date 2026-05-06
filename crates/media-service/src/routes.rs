use axum::{
    Router,
    routing::{get, post},
};
use shared::auth::AppState;

use crate::handlers;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Media upload
        .route("/v1/media/upload", post(handlers::upload::upload_media))
        .route(
            "/v1/media/upload/avatar",
            post(handlers::upload::upload_avatar),
        )
        .route(
            "/v1/media/upload/cover",
            post(handlers::upload::upload_cover),
        )
        // Transform (rotate/crop) — must come BEFORE {id} route
        .route(
            "/v1/media/{id}/rotate",
            post(handlers::transform::rotate_image),
        )
        .route("/v1/media/{id}/crop", post(handlers::transform::crop_image))
        .route(
            "/v1/media/{id}",
            get(handlers::upload::get_media).delete(handlers::upload::delete_media),
        )
        .route("/v1/media/my", get(handlers::upload::my_media))
        // Stories
        .route(
            "/v1/stories",
            get(handlers::stories::list_stories).post(handlers::stories::create_story),
        )
        .route(
            "/v1/stories/{id}",
            get(handlers::stories::get_story).delete(handlers::stories::delete_story),
        )
        .route("/v1/stories/{id}/view", post(handlers::stories::view_story))
        .route(
            "/v1/stories/{id}/viewers",
            get(handlers::stories::get_viewers),
        )
        .route("/v1/stories/my", get(handlers::stories::my_stories))
        .route(
            "/v1/stories/archive",
            get(handlers::stories::archived_stories),
        )
        .route(
            "/v1/stories/{id}/react",
            post(handlers::stories::react_to_story),
        )
        .route(
            "/v1/stories/{id}/reactions",
            get(handlers::stories::list_story_reactions),
        )
        .route(
            "/v1/stories/{id}/reply",
            post(handlers::stories::reply_to_story),
        )
        // Story Highlights (Instagram-style permanent story collections)
        .route(
            "/v1/story-highlights",
            post(handlers::highlights::create_highlight),
        )
        .route(
            "/v1/story-highlights/my",
            get(handlers::highlights::my_highlights),
        )
        .route(
            "/v1/users/{user_id}/story-highlights",
            get(handlers::highlights::user_highlights),
        )
        .route(
            "/v1/story-highlights/{id}",
            get(handlers::highlights::get_highlight)
                .put(handlers::highlights::update_highlight)
                .delete(handlers::highlights::delete_highlight),
        )
        .route(
            "/v1/story-highlights/{id}/stories",
            post(handlers::highlights::add_stories_to_highlight),
        )
        .route(
            "/v1/story-highlights/{id}/stories/{sid}",
            axum::routing::delete(handlers::highlights::remove_story_from_highlight),
        )
        // Health
        .route("/health", get(handlers::health::health_check))
        .with_state(state)
}
