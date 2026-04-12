use axum::{routing::{get, post}, Router};
use crate::handlers::{self, AiState};

pub fn create_router(state: AiState) -> Router {
    Router::new()
        .route("/v1/ai/chat", post(handlers::chat_completion))
        .route("/v1/ai/suggest-post", post(handlers::suggest_post))
        .route("/v1/ai/describe-image", post(handlers::describe_image))
        .route("/health", get(handlers::health_check))
        .with_state(state)
}
