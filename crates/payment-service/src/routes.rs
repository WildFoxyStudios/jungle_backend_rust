use axum::{
    routing::{get, post, put},
    Router,
};
use shared::auth::AppState;
use crate::handlers;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // ── Payments ──
        .route("/v1/payments/create", post(handlers::payments::create_payment))
        .route("/v1/payments/verify", post(handlers::payments::verify_payment))
        .route("/v1/payments/history", get(handlers::payments::payment_history))
        .route("/v1/payments/refund", post(handlers::payments::refund_payment))
        // ── Wallet ──
        .route("/v1/payments/wallet/balance", get(handlers::wallet::get_balance))
        .route("/v1/payments/wallet/add", post(handlers::wallet::add_funds))
        .route("/v1/payments/wallet/transfer", post(handlers::wallet::transfer))
        // ── Withdrawals ──
        .route("/v1/payments/withdraw", post(handlers::withdrawals::request_withdrawal))
        .route("/v1/payments/withdrawals", get(handlers::withdrawals::list_withdrawals))
        .route("/v1/payments/withdrawals/{id}/status", put(handlers::withdrawals::update_status))
        // ── Pro Subscriptions ──
        .route("/v1/payments/pro/plans", get(handlers::pro::list_plans))
        .route("/v1/payments/pro/subscribe", post(handlers::pro::subscribe))
        .route("/v1/payments/pro/cancel", post(handlers::pro::cancel))
        .route("/v1/payments/pro/refund-request", post(handlers::pro::request_refund))
        .route("/v1/payments/bank-receipt", post(handlers::pro::upload_bank_receipt))
        // ── Creator Subscriptions ──
        .route("/v1/payments/creator/tiers", post(handlers::creator::create_tier))
        .route("/v1/payments/creator/tiers/{id}", put(handlers::creator::update_tier).delete(handlers::creator::delete_tier))
        .route("/v1/payments/creator/{user_id}/tiers", get(handlers::creator::list_tiers))
        .route("/v1/payments/creator/subscribe/{user_id}", post(handlers::creator::subscribe).delete(handlers::creator::unsubscribe))
        .route("/v1/payments/creator/subscribers", get(handlers::creator::list_subscribers))
        .route("/v1/payments/creator/subscriptions", get(handlers::creator::my_subscriptions))
        // ── Webhooks ──
        .route("/v1/payments/webhooks/{provider}", post(handlers::webhooks::handle_webhook))
        // ── Health ──
        .route("/health", get(handlers::health::health_check))
        .with_state(state)
}
