use crate::handlers;
use axum::{
    Router,
    routing::{get, post, put},
};
use shared::auth::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // ── Products ──
        .route(
            "/v1/products",
            get(handlers::products::list_products).post(handlers::products::create_product),
        )
        .route(
            "/v1/products/search",
            get(handlers::products::search_products),
        )
        .route("/v1/products/my", get(handlers::products::my_products))
        // Plan §3.6 MK3 — seller dashboard totals + 30-day sparkline.
        .route(
            "/v1/products/me/stats",
            get(handlers::products::seller_stats),
        )
        .route(
            "/v1/products/categories",
            get(handlers::products::list_categories),
        )
        .route(
            "/v1/products/{id}",
            get(handlers::products::get_product)
                .put(handlers::products::update_product)
                .delete(handlers::products::delete_product),
        )
        .route(
            "/v1/products/{id}/reviews",
            get(handlers::products::list_reviews).post(handlers::products::add_review),
        )
        .route(
            "/v1/products/nearby",
            post(handlers::products::nearby_products),
        )
        // ── Saved Products & Price Alerts (Phases 13-15) ──
        .route(
            "/v1/products/{id}/save",
            post(handlers::products::save_product).delete(handlers::products::unsave_product),
        )
        .route(
            "/v1/users/me/saved-products",
            get(handlers::products::list_saved_products),
        )
        .route(
            "/v1/products/{id}/price-alert",
            post(handlers::products::create_price_alert),
        )
        // ── Cart ──
        .route(
            "/v1/cart",
            get(handlers::cart::list_cart)
                .post(handlers::cart::add_to_cart)
                .delete(handlers::cart::clear_cart),
        )
        .route(
            "/v1/cart/{id}",
            put(handlers::cart::update_cart_item).delete(handlers::cart::remove_from_cart),
        )
        // ── Orders ──
        .route("/v1/orders", post(handlers::orders::create_order))
        .route(
            "/v1/orders/checkout-wallet",
            post(handlers::orders::checkout_wallet),
        )
        .route("/v1/orders/my", get(handlers::orders::my_orders))
        .route("/v1/orders/sales", get(handlers::orders::my_sales))
        .route("/v1/orders/{id}", get(handlers::orders::get_order))
        .route(
            "/v1/orders/{id}/status",
            put(handlers::orders::update_status),
        )
        .route(
            "/v1/orders/{id}/tracking",
            get(handlers::orders::get_order_tracking),
        )
        .route(
            "/v1/orders/{id}/refund",
            post(handlers::orders::request_order_refund),
        )
        .route(
            "/v1/orders/{id}/invoice",
            get(handlers::invoices::download_invoice),
        )
        .route(
            "/v1/orders/{id}/dispute",
            post(handlers::orders::create_order_dispute),
        )
        .route(
            "/v1/users/me/disputes",
            get(handlers::orders::list_my_disputes),
        )
        // ── Jobs ──
        .route(
            "/v1/jobs",
            get(handlers::jobs::list_jobs).post(handlers::jobs::create_job),
        )
        .route("/v1/jobs/my", get(handlers::jobs::my_jobs))
        .route("/v1/jobs/applied", get(handlers::jobs::applied_jobs))
        .route("/v1/jobs/search", get(handlers::jobs::search_jobs))
        .route("/v1/jobs/categories", get(handlers::jobs::job_categories))
        .route("/v1/jobs/nearby", get(handlers::jobs::nearby_jobs))
        .route(
            "/v1/jobs/{id}",
            get(handlers::jobs::get_job)
                .put(handlers::jobs::update_job)
                .delete(handlers::jobs::delete_job),
        )
        .route("/v1/jobs/{id}/apply", post(handlers::jobs::apply_job))
        .route(
            "/v1/jobs/{id}/applications",
            get(handlers::jobs::list_applications),
        )
        .route(
            "/v1/jobs/applications/{id}/status",
            put(handlers::jobs::update_application_status),
        )
        // ── Funding ──
        .route(
            "/v1/fundings",
            get(handlers::funding::list_fundings).post(handlers::funding::create_funding),
        )
        .route("/v1/fundings/my", get(handlers::funding::my_fundings))
        .route(
            "/v1/fundings/{id}",
            get(handlers::funding::get_funding)
                .put(handlers::funding::update_funding)
                .delete(handlers::funding::delete_funding),
        )
        .route("/v1/fundings/{id}/donate", post(handlers::funding::donate))
        .route(
            "/v1/fundings/{id}/donations",
            get(handlers::funding::list_donations),
        )
        .route(
            "/v1/funding/personal",
            post(handlers::funding::create_personal_cause),
        )
        .route(
            "/v1/funding/{id}/withdraw",
            post(handlers::funding::withdraw_funding),
        )
        // ── Offers ──
        .route(
            "/v1/offers",
            get(handlers::offers::list_offers).post(handlers::offers::create_offer),
        )
        .route("/v1/offers/my", get(handlers::offers::my_offers))
        .route("/v1/offers/nearby", get(handlers::offers::nearby_offers))
        .route(
            "/v1/offers/{id}",
            get(handlers::offers::get_offer)
                .put(handlers::offers::update_offer)
                .delete(handlers::offers::delete_offer),
        )
        // ── Gifts ──
        .route("/v1/gifts", get(handlers::gifts::list_gifts))
        .route(
            "/v1/gifts/categories",
            get(handlers::gifts::list_gift_categories),
        )
        .route(
            "/v1/gifts/send/{recipient_id}",
            post(handlers::gifts::send_gift),
        )
        .route(
            "/v1/gifts/received",
            get(handlers::gifts::my_received_gifts),
        )
        // ── Stickers ──
        .route(
            "/v1/stickers/packs",
            get(handlers::gifts::list_sticker_packs),
        )
        .route(
            "/v1/stickers/packs/{id}",
            get(handlers::gifts::get_sticker_pack),
        )
        .route(
            "/v1/stickers/packs/{id}/purchase",
            post(handlers::gifts::purchase_sticker_pack),
        )
        .route("/v1/stickers/my", get(handlers::gifts::my_sticker_packs))
        // ── Saved Jobs + Alerts + Resumes (Phase 14) ──
        .route("/v1/jobs/{id}/save", post(handlers::jobs::save_job))
        .route("/v1/users/me/saved-jobs", get(handlers::jobs::list_saved_jobs))
        .route("/v1/users/me/job-alerts", post(handlers::jobs::create_job_alert).get(handlers::jobs::list_job_alerts))
        .route("/v1/users/me/resume", post(handlers::jobs::upload_resume).get(handlers::jobs::get_my_resume))
        // ── Health ──
        .route("/health", get(handlers::health::health_check))
        .with_state(state)
}
