use axum::{
    routing::{delete, get, post, put},
    Router,
};
use shared::auth::AppState;
use crate::handlers;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // ── Blogs ──
        .route("/v1/blogs", get(handlers::blogs::list_blogs))
        .route("/v1/blogs", post(handlers::blogs::create_blog))
        .route("/v1/blogs/search", get(handlers::blogs::search_blogs))
        .route("/v1/blogs/my", get(handlers::blogs::my_blogs))
        .route("/v1/blogs/categories", get(handlers::blogs::list_categories))
        .route("/v1/blogs/{id}", get(handlers::blogs::get_blog))
        .route("/v1/blogs/{id}", put(handlers::blogs::update_blog))
        .route("/v1/blogs/{id}", delete(handlers::blogs::delete_blog))
        .route("/v1/blogs/{id}/comments", get(handlers::blogs::list_comments))
        .route("/v1/blogs/{id}/comments", post(handlers::blogs::add_comment))
        .route("/v1/blogs/comments/{id}", delete(handlers::blogs::delete_comment))
        .route("/v1/blogs/upload-image", post(handlers::blogs::upload_blog_image))
        .route("/v1/blogs/{id}/react", post(handlers::extras::react_to_blog))
        .route("/v1/blogs/category/{id}", get(handlers::extras::blogs_by_category))
        // ── Forums ──
        .route("/v1/forums/sections", get(handlers::forums::list_sections))
        .route("/v1/forums/{id}/threads", get(handlers::forums::list_threads))
        .route("/v1/forums/{id}/threads", post(handlers::forums::create_thread))
        .route("/v1/forums/threads/{id}", get(handlers::forums::get_thread))
        .route("/v1/forums/threads/{id}", put(handlers::forums::update_thread))
        .route("/v1/forums/threads/{id}", delete(handlers::forums::delete_thread))
        .route("/v1/forums/threads/{id}/replies", get(handlers::forums::list_replies))
        .route("/v1/forums/threads/{id}/replies", post(handlers::forums::create_reply))
        .route("/v1/forums/replies/{id}", put(handlers::forums::update_reply))
        .route("/v1/forums/replies/{id}", delete(handlers::forums::delete_reply))
        .route("/v1/forums/threads/{id}/vote", post(handlers::forums::vote_thread))
        .route("/v1/forums/threads/{id}/share", post(handlers::forums::share_thread))
        // ── Movies ──
        .route("/v1/movies", get(handlers::movies::list_movies))
        .route("/v1/movies", post(handlers::movies::create_movie))
        .route("/v1/movies/{id}", get(handlers::movies::get_movie))
        .route("/v1/movies/{id}", put(handlers::movies::update_movie))
        .route("/v1/movies/{id}", delete(handlers::movies::delete_movie))
        .route("/v1/movies/{id}/comments", get(handlers::movies::list_movie_comments))
        .route("/v1/movies/{id}/comments", post(handlers::movies::add_movie_comment))
        .route("/v1/movies/{id}/react", post(handlers::movies::react_to_movie))
        .route("/v1/movies/{id}/watch", post(handlers::movies::watch_movie))
        .route("/v1/blogs/comments/{id}/react", post(handlers::extras::react_to_blog_comment))
        // ── Games ──
        .route("/v1/games", get(handlers::games::list_games))
        .route("/v1/games/my", get(handlers::games::my_games))
        .route("/v1/games/{id}", get(handlers::games::get_game))
        .route("/v1/games/{id}/play", post(handlers::games::play_game))
        // ── Public Custom Pages ──
        .route("/v1/pages/custom", get(handlers::extras::list_custom_pages))
        .route("/v1/pages/custom/{slug}", get(handlers::extras::get_custom_page))
        // ── Health ──
        .route("/health", get(handlers::health::health_check))
        .with_state(state)
}
