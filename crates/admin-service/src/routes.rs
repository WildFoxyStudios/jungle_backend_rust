use axum::{
    routing::{delete, get, post, put},
    Router,
};
use shared::auth::AppState;
use crate::handlers;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // ── Dashboard ──
        .route("/v1/admin/dashboard", get(handlers::dashboard::stats))
        .route("/v1/admin/dashboard/charts", get(handlers::extras::charts))
        .route("/v1/admin/dashboard/top-countries", get(handlers::extras::top_countries))
        .route("/v1/admin/system-info", get(handlers::extras::system_info))
        // ── User Management ──
        .route("/v1/admin/users", get(handlers::users::list_users))
        .route("/v1/admin/users/{id}", get(handlers::users::get_user).put(handlers::users::update_user).delete(handlers::users::delete_user))
        .route("/v1/admin/users/{id}/ban", post(handlers::users::ban_user))
        .route("/v1/admin/users/{id}/unban", post(handlers::users::unban_user))
        .route("/v1/admin/users/{id}/verify", post(handlers::users::verify_user))
        // ── Reports ──
        .route("/v1/admin/reports", get(handlers::reports::list_reports))
        .route("/v1/admin/reports/{id}", get(handlers::reports::get_report))
        .route("/v1/admin/reports/{id}/resolve", post(handlers::reports::resolve_report))
        .route("/v1/admin/reports/{id}/dismiss", post(handlers::reports::dismiss_report))
        // ── Config ──
        .route("/v1/admin/config", get(handlers::config::list_config).put(handlers::config::update_config))
        .route("/v1/admin/config/catalog", get(handlers::config_catalog::get_catalog))
        .route(
            "/v1/admin/config/{category}",
            get(handlers::config::get_category).put(handlers::config::update_category),
        )
        // ── Categories ──
        .route("/v1/admin/categories", get(handlers::categories::list_categories).post(handlers::categories::create_category))
        .route("/v1/admin/categories/{id}", put(handlers::categories::update_category).delete(handlers::categories::delete_category))
        // ── Languages ──
        .route("/v1/admin/languages", get(handlers::languages::list_languages).post(handlers::languages::create_language))
        .route("/v1/admin/languages/{id}", put(handlers::languages::update_language).delete(handlers::languages::delete_language))
        // ── Announcements ──
        .route("/v1/admin/announcements", get(handlers::announcements::list_announcements).post(handlers::announcements::create_announcement))
        .route("/v1/admin/announcements/{id}", put(handlers::announcements::update_announcement).delete(handlers::announcements::delete_announcement))
        // ── Content Moderation ──
        .route("/v1/admin/moderation/posts", get(handlers::moderation::pending_posts))
        .route("/v1/admin/moderation/posts/{id}/approve", post(handlers::moderation::approve_post))
        .route("/v1/admin/moderation/posts/{id}/reject", post(handlers::moderation::reject_post))
        .route("/v1/admin/moderation/blogs", get(handlers::moderation::pending_blogs))
        .route("/v1/admin/moderation/blogs/{id}/approve", post(handlers::moderation::approve_blog))
        .route("/v1/admin/moderation/blogs/{id}/reject", post(handlers::moderation::reject_blog))
        // ── Verification Requests ──
        .route("/v1/admin/verifications", get(handlers::verifications::list_verification_requests))
        .route("/v1/admin/verifications/{id}/approve", post(handlers::verifications::approve_verification))
        .route("/v1/admin/verifications/{id}/reject", post(handlers::verifications::reject_verification))
        // ── Payments Admin ──
        .route("/v1/admin/payments/stats", get(handlers::payments_admin::payment_stats))
        .route("/v1/admin/payments/transactions", get(handlers::payments_admin::list_transactions))
        .route("/v1/admin/payments/withdrawals", get(handlers::payments_admin::list_pending_withdrawals))
        .route("/v1/admin/payments/withdrawals/{id}/approve", post(handlers::payments_admin::approve_withdrawal))
        .route("/v1/admin/payments/withdrawals/{id}/reject", post(handlers::payments_admin::reject_withdrawal))
        .route("/v1/admin/payments/pro-plans", get(handlers::payments_admin::list_pro_plans).post(handlers::payments_admin::upsert_pro_plan))
        // ── Payment Requests (dedicated endpoint with stats) ──
        .route("/v1/admin/payment-requests", get(handlers::payments_admin::list_payment_requests))
        // ── Banned IPs ──
        .route("/v1/admin/banned-ips", get(handlers::banned_ips::list_banned_ips).post(handlers::banned_ips::ban_ip))
        .route("/v1/admin/banned-ips/{id}", delete(handlers::banned_ips::unban_ip))
        // ── Custom Pages ──
        .route("/v1/admin/pages", get(handlers::custom_pages::list_custom_pages).post(handlers::custom_pages::create_custom_page))
        .route("/v1/admin/pages/{id}", put(handlers::custom_pages::update_custom_page).delete(handlers::custom_pages::delete_custom_page))
        .route("/v1/admin/pages/slug/{slug}", get(handlers::custom_pages::get_custom_page_by_slug))
        // ── Translations ──
        .route("/v1/admin/translations", get(handlers::translations::list_translations).post(handlers::translations::upsert_translation))
        .route("/v1/admin/translations/bulk", post(handlers::translations::bulk_upsert_translations))
        .route("/v1/admin/translations/{id}", delete(handlers::translations::delete_translation))
        // ── Newsletter ──
        .route("/v1/admin/newsletter/subscribers", get(handlers::newsletter::list_subscribers))
        .route("/v1/admin/newsletter/subscribers/{id}", delete(handlers::newsletter::remove_subscriber))
        .route("/v1/admin/newsletter/send", post(handlers::newsletter::send_newsletter))
        // ── Profile Fields ──
        .route("/v1/admin/profile-fields", get(handlers::profile_fields::list_fields).post(handlers::profile_fields::create_field))
        .route("/v1/admin/profile-fields/{id}", put(handlers::profile_fields::update_field).delete(handlers::profile_fields::delete_field))
        // ── User Roles ──
        .route("/v1/admin/users/{user_id}/make-admin", post(handlers::user_roles::make_admin))
        .route("/v1/admin/users/{user_id}/remove-admin", post(handlers::user_roles::remove_admin))
        .route("/v1/admin/users/{user_id}/make-pro", post(handlers::user_roles::make_pro))
        .route("/v1/admin/users/{user_id}/remove-pro", post(handlers::user_roles::remove_pro))
        // ── Email Templates ──
        .route("/v1/admin/email-templates", get(handlers::email_templates::list_templates).post(handlers::email_templates::create_template))
        .route("/v1/admin/email-templates/{id}", put(handlers::email_templates::update_template).delete(handlers::email_templates::delete_template))
        // ── Ads Management ──
        .route("/v1/admin/ads", get(handlers::extras::list_ads))
        .route("/v1/admin/ads/{id}", put(handlers::extras::update_ad))
        // ── Content Admin (Pages/Groups/Blogs/Products/Jobs/Funding/Events/Forums) ──
        .route("/v1/admin/site-pages", get(handlers::content_admin::list_pages))
        .route("/v1/admin/site-pages/{id}", delete(handlers::content_admin::delete_page))
        .route("/v1/admin/site-groups", get(handlers::content_admin::list_groups))
        .route("/v1/admin/site-groups/{id}", delete(handlers::content_admin::delete_group))
        .route("/v1/admin/site-blogs", get(handlers::content_admin::list_blogs))
        .route("/v1/admin/site-blogs/{id}/approve", post(handlers::content_admin::approve_blog))
        .route("/v1/admin/site-blogs/{id}", delete(handlers::content_admin::delete_blog))
        .route("/v1/admin/site-products", get(handlers::content_admin::list_products))
        .route("/v1/admin/site-products/{id}", delete(handlers::content_admin::delete_product))
        .route("/v1/admin/site-jobs", get(handlers::content_admin::list_jobs))
        .route("/v1/admin/site-jobs/{id}", delete(handlers::content_admin::delete_job))
        .route("/v1/admin/site-funding", get(handlers::content_admin::list_funding))
        .route("/v1/admin/site-funding/{id}", delete(handlers::content_admin::delete_funding))
        .route("/v1/admin/site-events", get(handlers::content_admin::list_events))
        .route("/v1/admin/site-events/{id}", delete(handlers::content_admin::delete_event))
        .route("/v1/admin/site-forums", get(handlers::content_admin::list_forums))
        .route("/v1/admin/site-forums/{id}", put(handlers::content_admin::update_forum).delete(handlers::content_admin::delete_forum))
        // ── Colored Post Templates ──
        .route("/v1/admin/colored-posts", get(handlers::templates_config::list_colored_post_templates).post(handlers::templates_config::create_colored_post_template))
        .route("/v1/admin/colored-posts/{id}", put(handlers::templates_config::update_colored_post_template).delete(handlers::templates_config::delete_colored_post_template))
        // ── Reaction Types ──
        .route("/v1/admin/reaction-types", get(handlers::templates_config::list_reaction_types).post(handlers::templates_config::create_reaction_type))
        .route("/v1/admin/reaction-types/{id}", put(handlers::templates_config::update_reaction_type).delete(handlers::templates_config::delete_reaction_type))
        // ── Gifts Admin ──
        .route("/v1/admin/gifts", get(handlers::gifts_stickers::list_gifts).post(handlers::gifts_stickers::create_gift))
        .route("/v1/admin/gifts/{id}", put(handlers::gifts_stickers::update_gift).delete(handlers::gifts_stickers::delete_gift))
        // ── Sticker Packs Admin ──
        .route("/v1/admin/sticker-packs", get(handlers::gifts_stickers::list_sticker_packs).post(handlers::gifts_stickers::create_sticker_pack))
        .route("/v1/admin/sticker-packs/{id}", put(handlers::gifts_stickers::update_sticker_pack).delete(handlers::gifts_stickers::delete_sticker_pack))
        .route("/v1/admin/sticker-packs/{pack_id}/stickers", get(handlers::gifts_stickers::list_stickers).post(handlers::gifts_stickers::add_sticker))
        .route("/v1/admin/stickers/{id}", delete(handlers::gifts_stickers::delete_sticker))
        // ── Activity Log ──
        .route("/v1/admin/activities", get(handlers::activity_log::list_activities))
        // ── Audit Log (every admin mutation) ──
        .route("/v1/admin/audit-log", get(handlers::audit_dlq::list_audit_log))
        // ── Dead Letter Queue (failed events) ──
        .route("/v1/admin/events/dlq", get(handlers::audit_dlq::list_dlq))
        .route("/v1/admin/events/dlq/{id}", delete(handlers::audit_dlq::discard_dlq_item))
        .route("/v1/admin/events/dlq/{id}/retry", post(handlers::audit_dlq::retry_dlq_item))
        // ── Storage providers (S3/R2/MinIO/...) ──
        .route("/v1/admin/storage/config", get(handlers::storage_and_perms::list_storage).post(handlers::storage_and_perms::create_storage))
        .route("/v1/admin/storage/config/{id}", axum::routing::patch(handlers::storage_and_perms::update_storage).delete(handlers::storage_and_perms::delete_storage))
        .route("/v1/admin/storage/config/{id}/test", post(handlers::storage_and_perms::test_storage))
        // ── Permissions catalog (static list of granular admin actions) ──
        .route("/v1/admin/permissions/catalog", get(handlers::storage_and_perms::permissions_catalog))
        // ── Invitations ──
        .route("/v1/admin/invitations", get(handlers::invitations::list_invitations).post(handlers::invitations::create_invitation))
        .route("/v1/admin/invitations/{id}", delete(handlers::invitations::delete_invitation))
        // ── OAuth Apps Admin ──
        .route("/v1/admin/oauth-apps", get(handlers::oauth_admin::list_oauth_apps))
        .route("/v1/admin/oauth-apps/{id}/toggle", post(handlers::oauth_admin::toggle_oauth_app))
        .route("/v1/admin/oauth-apps/{id}", delete(handlers::oauth_admin::delete_oauth_app))
        // ── Backups ──
        .route("/v1/admin/backups", get(handlers::backup::list_backups))
        .route("/v1/admin/backups/trigger", post(handlers::backup::trigger_backup))
        // ── Admin Delete Post (hard delete) ──
        .route("/v1/admin/posts/{id}", delete(handlers::moderation::admin_delete_post))
        // ── Genders ──
        .route("/v1/admin/genders", get(handlers::manage_content::list_genders).post(handlers::manage_content::create_gender))
        .route("/v1/admin/genders/{id}", put(handlers::manage_content::update_gender).delete(handlers::manage_content::delete_gender))
        // ── Sub-Categories ──
        .route("/v1/admin/sub-categories", get(handlers::manage_content::list_sub_categories).post(handlers::manage_content::create_sub_category))
        .route("/v1/admin/sub-categories/{id}", put(handlers::manage_content::update_sub_category).delete(handlers::manage_content::delete_sub_category))
        // ── Terms Pages ──
        .route("/v1/admin/terms-pages", get(handlers::manage_content::list_terms_pages))
        .route("/v1/admin/terms-pages/{id}", put(handlers::manage_content::update_terms_page))
        // ── Movies ──
        .route("/v1/admin/movies", get(handlers::manage_content::list_movies))
        .route("/v1/admin/movies/{id}", delete(handlers::manage_content::delete_movie))
        .route("/v1/admin/movies/{id}/approve", post(handlers::manage_content::approve_movie))
        .route("/v1/admin/movies/{id}/feature", post(handlers::manage_content::feature_movie))
        // ── Games ──
        .route("/v1/admin/games", get(handlers::manage_content::list_games).post(handlers::manage_content::create_game))
        .route("/v1/admin/games/{id}/toggle", post(handlers::manage_content::toggle_game))
        .route("/v1/admin/games/{id}", delete(handlers::manage_content::delete_game))
        // ── Bank Receipts ──
        .route("/v1/admin/bank-receipts", get(handlers::manage_content::list_bank_receipts))
        .route("/v1/admin/bank-receipts/{id}/approve", post(handlers::manage_content::approve_bank_receipt))
        .route("/v1/admin/bank-receipts/{id}/reject", post(handlers::manage_content::reject_bank_receipt))
        // ── Currencies ──
        .route("/v1/admin/currencies", get(handlers::manage_content::list_currencies).post(handlers::manage_content::create_currency))
        .route("/v1/admin/currencies/{id}", put(handlers::manage_content::update_currency).delete(handlers::manage_content::delete_currency))
        .route("/v1/admin/currencies/{id}/toggle", post(handlers::manage_content::toggle_currency))
        // ── Pro Members ──
        .route("/v1/admin/pro-members", get(handlers::manage_users::list_pro_members))
        // ── Online Users ──
        .route("/v1/admin/online-users", get(handlers::manage_users::list_online_users))
        // ── Referrals ──
        .route("/v1/admin/referrals", get(handlers::manage_users::list_referrals))
        // ── User Ads Management ──
        .route("/v1/admin/user-ads", get(handlers::manage_users::list_user_ads))
        .route("/v1/admin/user-ads/{id}/toggle", post(handlers::manage_users::toggle_user_ad))
        .route("/v1/admin/user-ads/{id}", delete(handlers::manage_users::delete_user_ad))
        // ── Stories Management ──
        .route("/v1/admin/stories", get(handlers::manage_users::list_stories))
        .route("/v1/admin/stories/{id}/hide", post(handlers::manage_users::hide_story))
        .route("/v1/admin/stories/{id}", delete(handlers::manage_users::delete_story))
        // ── Posts Management (full listing) ──
        .route("/v1/admin/manage-posts", get(handlers::manage_users::list_all_posts))
        // ── Offers Management ──
        .route("/v1/admin/offers", get(handlers::manage_users::list_all_offers))
        .route("/v1/admin/offers/{id}", delete(handlers::manage_users::delete_offer))
        // ── Orders Management ──
        .route("/v1/admin/orders", get(handlers::manage_users::list_all_orders))
        // ── Reviews Management ──
        .route("/v1/admin/reviews", get(handlers::manage_users::list_all_reviews))
        .route("/v1/admin/reviews/{id}", delete(handlers::manage_users::delete_review))
        // ── Pro Refunds ──
        .route("/v1/admin/refunds", get(handlers::manage_users::list_refund_requests))
        .route("/v1/admin/refunds/{id}/approve", post(handlers::manage_users::approve_refund))
        .route("/v1/admin/refunds/{id}/reject", post(handlers::manage_users::reject_refund))
        // ── Mass Notifications ──
        .route("/v1/admin/mass-notifications", get(handlers::manage_users::list_mass_notifications))
        .route("/v1/admin/mass-notifications/send", post(handlers::manage_users::send_mass_notification))
        // ── Sitemap ──
        .route("/v1/admin/sitemap/generate", post(handlers::manage_users::generate_sitemap))
        // ── Fake Users ──
        .route("/v1/admin/fake-users", get(handlers::manage_users::list_fake_users).post(handlers::manage_users::create_fake_user))
        // ── API Access Keys ──
        .route("/v1/admin/api-keys", get(handlers::manage_users::list_api_keys).post(handlers::manage_users::create_api_key))
        .route("/v1/admin/api-keys/{id}/toggle", post(handlers::manage_users::toggle_api_key))
        .route("/v1/admin/api-keys/{id}", delete(handlers::manage_users::delete_api_key))
        // ── Forum Sections Admin ──
        .route("/v1/admin/forum-sections", get(handlers::forum_admin::list_forum_sections).post(handlers::forum_admin::create_forum_section))
        .route("/v1/admin/forum-sections/{id}", put(handlers::forum_admin::update_forum_section).delete(handlers::forum_admin::delete_forum_section))
        // ── Forums Admin: Create ──
        .route("/v1/admin/forums", post(handlers::forum_admin::create_forum))
        // ── Forum Threads Admin ──
        .route("/v1/admin/forum-threads", get(handlers::forum_admin::list_forum_threads))
        .route("/v1/admin/forum-threads/{id}", delete(handlers::forum_admin::delete_forum_thread))
        // ── Forum Replies Admin ──
        .route("/v1/admin/forum-replies", get(handlers::forum_admin::list_forum_replies))
        .route("/v1/admin/forum-replies/{id}", delete(handlers::forum_admin::delete_forum_reply))
        // ── Movies: Create + Update ──
        .route("/v1/admin/manage-movies", post(handlers::forum_admin::create_movie))
        .route("/v1/admin/manage-movies/{id}", put(handlers::forum_admin::update_movie))
        // ── Auto Settings ──
        .route("/v1/admin/auto-settings", get(handlers::advanced_settings::get_auto_settings))
        .route("/v1/admin/auto-settings/auto-delete", put(handlers::advanced_settings::update_auto_delete_settings))
        .route("/v1/admin/auto-settings/friends", post(handlers::advanced_settings::add_auto_friend))
        .route("/v1/admin/auto-settings/friends/{id}", delete(handlers::advanced_settings::remove_auto_friend))
        .route("/v1/admin/auto-settings/joins", post(handlers::advanced_settings::add_auto_join))
        .route("/v1/admin/auto-settings/joins/{id}", delete(handlers::advanced_settings::remove_auto_join))
        .route("/v1/admin/auto-settings/likes", post(handlers::advanced_settings::add_auto_like))
        .route("/v1/admin/auto-settings/likes/{id}", delete(handlers::advanced_settings::remove_auto_like))
        // ── Custom Code ──
        .route("/v1/admin/custom-code", get(handlers::advanced_settings::get_custom_code).put(handlers::advanced_settings::update_custom_code))
        // ── Site Ads (admin-managed) ──
        .route("/v1/admin/site-ads", get(handlers::advanced_settings::list_site_ads).post(handlers::advanced_settings::create_site_ad))
        .route("/v1/admin/site-ads/{id}", put(handlers::advanced_settings::update_site_ad).delete(handlers::advanced_settings::delete_site_ad))
        // ── User Permissions ──
        .route("/v1/admin/users/{user_id}/permissions", get(handlers::advanced_settings::get_user_permissions).put(handlers::advanced_settings::update_user_permissions))
        // ── Advanced User Management ──
        .route("/v1/admin/users/{user_id}/top-up", post(handlers::advanced_settings::top_up_wallet))
        .route("/v1/admin/users/{user_id}/content", delete(handlers::advanced_settings::delete_user_content))
        .route("/v1/admin/send-email", post(handlers::advanced_settings::send_email_to_user))
        // ── Content Monetization ──
        .route("/v1/admin/monetization", get(handlers::advanced_settings::list_monetization_subscriptions))
        // ── Dynamic Settings by Category ──
        .route("/v1/admin/settings/{category}", get(handlers::advanced_settings::get_settings_category).put(handlers::advanced_settings::update_settings_category))
        // ── Live streams (moderation + analytics) ──
        .route("/v1/admin/live-streams", get(handlers::live_and_email::list_live_streams))
        .route("/v1/admin/live-streams/{id}", delete(handlers::live_and_email::force_end_live_stream))
        .route("/v1/admin/live/stats", get(handlers::live_and_email::live_stats))
        // ── Realtime platform stats (websocket + active sessions) ──
        .route("/v1/admin/realtime/stats", get(handlers::realtime::realtime_stats))
        // ── Bulk email campaigns ──
        .route("/v1/admin/email-campaigns", post(handlers::live_and_email::create_email_campaign))
        // ── Changelog (migrations history) ──
        .route("/v1/admin/changelog", get(handlers::live_and_email::list_changelog))
        // ── Cronjobs status + config ──
        .route("/v1/admin/cronjobs/status", get(handlers::live_and_email::cronjobs_status))
        .route("/v1/admin/cronjob-config", get(handlers::live_and_email::list_cronjob_config))
        .route("/v1/admin/cronjob-config/{name}", put(handlers::live_and_email::update_cronjob_config))
        // ── Health ──
        .route("/v1/admin/health", get(handlers::health::admin_health))
        .route("/health", get(handlers::health::health_check))
        .with_state(state)
}
