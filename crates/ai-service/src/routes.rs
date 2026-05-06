use crate::handlers::{self, AiState};
use axum::{
    Router,
    routing::{get, patch, post},
};

pub fn create_router(state: AiState) -> Router {
    Router::new()
        // New v1 endpoints
        .route("/v1/ai/generate-post", post(handlers::generate_post))
        .route("/v1/ai/generate-blog", post(handlers::generate_blog))
        .route("/v1/ai/generate-images", post(handlers::generate_images))
        .route("/v1/ai/balance/words", get(handlers::get_balance_words))
        .route("/v1/ai/balance/images", get(handlers::get_balance_images))
        // Legacy endpoints (still supported)
        .route("/v1/ai/chat", post(handlers::chat_completion))
        .route("/v1/ai/chat-suggestions", post(handlers::chat_suggestions))
        .route("/v1/ai/suggest-post", post(handlers::suggest_post))
        .route("/v1/ai/describe-image", post(handlers::describe_image))
        // Admin CRUD for AI providers (under /v1/ai so the gateway routes to ai-service)
        .route(
            "/v1/ai/admin/providers",
            get(handlers::admin_list_providers).post(handlers::admin_create_provider),
        )
        .route(
            "/v1/ai/admin/providers/health",
            get(handlers::admin_providers_health),
        )
        .route(
            "/v1/ai/admin/providers/{id}",
            patch(handlers::admin_update_provider).delete(handlers::admin_delete_provider),
        )
        .route(
            "/v1/ai/admin/providers/{id}/test",
            post(handlers::admin_test_provider),
        )
        // Internal moderation
        .route(
            "/v1/internal/ai/moderate",
            post(handlers::moderate_content),
        )
        // Internal OCR
        .route("/v1/internal/ai/ocr", post(handlers::ocr_extract))
        // Internal transcription
        .route("/v1/internal/ai/transcribe", post(handlers::transcribe_audio))
        // Health
        .route("/health", get(handlers::health_check))
        .with_state(state)
}
