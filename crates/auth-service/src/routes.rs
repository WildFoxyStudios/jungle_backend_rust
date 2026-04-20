use axum::{
    routing::{delete, get, post, put},
    Router,
};
use shared::auth::AppState;

use crate::handlers;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // ── Core auth ──
        .route("/v1/auth/register", post(handlers::register::register))
        .route("/v1/auth/login", post(handlers::login::login))
        .route("/v1/auth/refresh", post(handlers::refresh::refresh_token))
        .route("/v1/auth/logout", post(handlers::logout::logout))
        .route("/v1/auth/me", get(handlers::me::me))
        // ── Password recovery ──
        .route("/v1/auth/forgot-password", post(handlers::password::forgot_password))
        .route("/v1/auth/reset-password", post(handlers::password::reset_password))
        .route("/v1/auth/password", put(handlers::password::change_password))
        // ── Email / Phone verification ──
        .route("/v1/auth/verify-email", post(handlers::verify::verify_email))
        .route("/v1/auth/verify-phone", post(handlers::verify::verify_phone))
        .route("/v1/auth/resend-code", post(handlers::verify::resend_verification))
        // ── 2FA ──
        .route("/v1/auth/2fa/setup", post(handlers::two_factor::setup_2fa))
        .route("/v1/auth/2fa/enable", post(handlers::two_factor::enable_2fa))
        .route("/v1/auth/2fa/verify", post(handlers::two_factor::verify_2fa))
        .route("/v1/auth/2fa/disable", post(handlers::two_factor::disable_2fa))
        .route("/v1/auth/2fa/backup-codes", get(handlers::two_factor::get_backup_codes))
        .route("/v1/auth/2fa/backup-codes/regenerate", post(handlers::two_factor::regenerate_backup_codes))
        // ── Social login (14 providers) ──
        .route("/v1/auth/social/login", post(handlers::social::social_login))
        .route("/v1/auth/social/set-password", post(handlers::password::set_social_password))
        // ── Switch Account ──
        .route("/v1/auth/switch-account", post(handlers::switch_account::switch_account))
        // ── Sessions ──
        .route("/v1/auth/sessions", get(handlers::sessions::list_sessions))
        .route("/v1/auth/sessions/{id}", delete(handlers::sessions::revoke_session))
        .route("/v1/auth/sessions/revoke-all", post(handlers::sessions::revoke_all_sessions))
        // ── OAuth Developer Portal ──
        .route("/v1/oauth/apps", get(handlers::oauth_apps::list_apps).post(handlers::oauth_apps::create_app))
        .route("/v1/oauth/apps/{id}", get(handlers::oauth_apps::get_app).put(handlers::oauth_apps::update_app).delete(handlers::oauth_apps::delete_app))
        .route("/v1/oauth/apps/{id}/permissions", get(handlers::oauth_apps::get_app_permissions))
        .route("/v1/oauth/authorize", post(handlers::oauth_apps::authorize))
        .route("/v1/oauth/token", post(handlers::oauth_apps::exchange_token))
        .route("/v1/oauth/revoke", post(handlers::oauth_apps::revoke_token))
        // ── Public (no auth required) ──
        .route("/v1/translations/{lang}", get(handlers::public::get_translations))
        .route("/v1/config/public", get(handlers::public::get_public_config))
        .route("/v1/site-settings", get(handlers::public::get_site_settings))
        .route("/v1/auth/check", get(handlers::public::check_availability))
        .route("/v1/auth/is-active", get(handlers::public::is_active))
        // ── Health ──
        .route("/health", get(handlers::health::health_check))
        .with_state(state)
}
