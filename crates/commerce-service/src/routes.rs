use axum::{
    routing::{delete, get, post, put},
    Router,
};
use shared::auth::AppState;
use crate::handlers;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // ── Products ──
        .route("/v1/products", get(handlers::products::list_products))
        .route("/v1/products", post(handlers::products::create_product))
        .route("/v1/products/search", get(handlers::products::search_products))
        .route("/v1/products/my", get(handlers::products::my_products))
        .route("/v1/products/categories", get(handlers::products::list_categories))
        .route("/v1/products/{id}", get(handlers::products::get_product))
        .route("/v1/products/{id}", put(handlers::products::update_product))
        .route("/v1/products/{id}", delete(handlers::products::delete_product))
        .route("/v1/products/{id}/reviews", get(handlers::products::list_reviews))
        .route("/v1/products/{id}/reviews", post(handlers::products::add_review))
        .route("/v1/products/nearby", post(handlers::products::nearby_products))
        // ── Cart ──
        .route("/v1/cart", get(handlers::cart::list_cart))
        .route("/v1/cart", post(handlers::cart::add_to_cart))
        .route("/v1/cart", delete(handlers::cart::clear_cart))
        .route("/v1/cart/{id}", put(handlers::cart::update_cart_item))
        .route("/v1/cart/{id}", delete(handlers::cart::remove_from_cart))
        // ── Orders ──
        .route("/v1/orders", post(handlers::orders::create_order))
        .route("/v1/orders/my", get(handlers::orders::my_orders))
        .route("/v1/orders/sales", get(handlers::orders::my_sales))
        .route("/v1/orders/{id}", get(handlers::orders::get_order))
        .route("/v1/orders/{id}/status", put(handlers::orders::update_status))
        .route("/v1/orders/{id}/tracking", get(handlers::orders::get_order_tracking))
        .route("/v1/orders/{id}/refund", post(handlers::orders::request_order_refund))
        // ── Jobs ──
        .route("/v1/jobs", get(handlers::jobs::list_jobs))
        .route("/v1/jobs", post(handlers::jobs::create_job))
        .route("/v1/jobs/my", get(handlers::jobs::my_jobs))
        .route("/v1/jobs/applied", get(handlers::jobs::applied_jobs))
        .route("/v1/jobs/search", get(handlers::jobs::search_jobs))
        .route("/v1/jobs/categories", get(handlers::jobs::job_categories))
        .route("/v1/jobs/{id}", get(handlers::jobs::get_job))
        .route("/v1/jobs/{id}", put(handlers::jobs::update_job))
        .route("/v1/jobs/{id}", delete(handlers::jobs::delete_job))
        .route("/v1/jobs/{id}/apply", post(handlers::jobs::apply_job))
        .route("/v1/jobs/{id}/applications", get(handlers::jobs::list_applications))
        .route("/v1/jobs/applications/{id}/status", put(handlers::jobs::update_application_status))
        // ── Funding ──
        .route("/v1/fundings", get(handlers::funding::list_fundings))
        .route("/v1/fundings", post(handlers::funding::create_funding))
        .route("/v1/fundings/my", get(handlers::funding::my_fundings))
        .route("/v1/fundings/{id}", get(handlers::funding::get_funding))
        .route("/v1/fundings/{id}", put(handlers::funding::update_funding))
        .route("/v1/fundings/{id}", delete(handlers::funding::delete_funding))
        .route("/v1/fundings/{id}/donate", post(handlers::funding::donate))
        .route("/v1/fundings/{id}/donations", get(handlers::funding::list_donations))
        // ── Offers ──
        .route("/v1/offers", get(handlers::offers::list_offers))
        .route("/v1/offers", post(handlers::offers::create_offer))
        .route("/v1/offers/my", get(handlers::offers::my_offers))
        .route("/v1/offers/nearby", get(handlers::offers::nearby_offers))
        .route("/v1/offers/{id}", get(handlers::offers::get_offer))
        .route("/v1/offers/{id}", put(handlers::offers::update_offer))
        .route("/v1/offers/{id}", delete(handlers::offers::delete_offer))
        // ── Gifts ──
        .route("/v1/gifts", get(handlers::gifts::list_gifts))
        .route("/v1/gifts/categories", get(handlers::gifts::list_gift_categories))
        .route("/v1/gifts/send/{recipient_id}", post(handlers::gifts::send_gift))
        .route("/v1/gifts/received", get(handlers::gifts::my_received_gifts))
        // ── Stickers ──
        .route("/v1/stickers/packs", get(handlers::gifts::list_sticker_packs))
        .route("/v1/stickers/packs/{id}", get(handlers::gifts::get_sticker_pack))
        .route("/v1/stickers/packs/{id}/purchase", post(handlers::gifts::purchase_sticker_pack))
        .route("/v1/stickers/my", get(handlers::gifts::my_sticker_packs))
        // ── Health ──
        .route("/health", get(handlers::health::health_check))
        .with_state(state)
}
