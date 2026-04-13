use axum::{
    routing::{get, post},
    Router,
};
use shared::auth::AppState;

use crate::handlers;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Media upload
        .route("/v1/media/upload", post(handlers::upload::upload_media))
        .route("/v1/media/upload/avatar", post(handlers::upload::upload_avatar))
        .route("/v1/media/upload/cover", post(handlers::upload::upload_cover))
        .route("/v1/media/{id}", get(handlers::upload::get_media).delete(handlers::upload::delete_media))
        .route("/v1/media/my", get(handlers::upload::my_media))
        // Stories
        .route("/v1/stories", get(handlers::stories::list_stories).post(handlers::stories::create_story))
        .route("/v1/stories/{id}", get(handlers::stories::get_story).delete(handlers::stories::delete_story))
        .route("/v1/stories/{id}/view", post(handlers::stories::view_story))
        .route("/v1/stories/{id}/viewers", get(handlers::stories::get_viewers))
        .route("/v1/stories/my", get(handlers::stories::my_stories))
        .route("/v1/stories/archive", get(handlers::stories::archived_stories))
        .route("/v1/stories/{id}/react", post(handlers::stories::react_to_story))
        .route("/v1/stories/{id}/reactions", get(handlers::stories::list_story_reactions))
        .route("/v1/stories/{id}/reply", post(handlers::stories::reply_to_story))
        // Health
        .route("/health", get(handlers::health::health_check))
        .with_state(state)
}
