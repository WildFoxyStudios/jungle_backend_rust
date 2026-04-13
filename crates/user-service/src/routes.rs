use axum::{
    routing::{delete, get, post, put},
    Router,
};
use shared::auth::AppState;

use crate::handlers;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Profile
        .route("/v1/users/me", get(handlers::profile::get_me))
        .route("/v1/users/me", put(handlers::profile::update_me))
        .route("/v1/users/me", delete(handlers::extras::delete_account))
        .route("/v1/users/me/avatar", put(handlers::profile::update_avatar))
        .route("/v1/users/me/cover", put(handlers::profile::update_cover))
        .route("/v1/users/{username}", get(handlers::profile::get_user))
        // Search
        .route("/v1/users/search", get(handlers::search::search_users))
        .route("/v1/users/suggestions", get(handlers::search::suggestions))
        .route("/v1/users/pro-users", get(handlers::extras::pro_users))
        // Social
        .route("/v1/social/follow/{user_id}", post(handlers::social::follow_user))
        .route("/v1/social/follow/{user_id}", delete(handlers::social::unfollow_user))
        .route("/v1/users/{username}/followers", get(handlers::social::get_followers))
        .route("/v1/users/{username}/following", get(handlers::social::get_following))
        // Follow Requests
        .route("/v1/social/follow-requests", get(handlers::extras::list_follow_requests))
        .route("/v1/social/follow-requests/{id}/accept", post(handlers::extras::accept_follow_request))
        .route("/v1/social/follow-requests/{id}/reject", post(handlers::extras::reject_follow_request))
        // Block
        .route("/v1/social/blocked", get(handlers::social::list_blocked_users))
        .route("/v1/social/block/{user_id}", post(handlers::social::block_user))
        .route("/v1/social/block/{user_id}", delete(handlers::social::unblock_user))
        // Poke
        .route("/v1/social/pokes", get(handlers::social::list_pokes))
        .route("/v1/social/poke/{user_id}", post(handlers::social::poke_user))
        // Mute
        .route("/v1/social/mute/{user_id}", post(handlers::social::mute_user))
        .route("/v1/social/mute/{user_id}", delete(handlers::social::unmute_user))
        // Family
        .route("/v1/social/family/{user_id}", post(handlers::extras::send_family_request))
        .route("/v1/social/family/{id}", put(handlers::extras::respond_family_request))
        // Stop Notify
        .route("/v1/social/stop-notify/{user_id}", post(handlers::extras::stop_notify))
        // Professional / LinkedIn Mode
        .route("/v1/users/{user_id}/experience", get(handlers::professional::list_experience))
        .route("/v1/users/me/experience", post(handlers::professional::add_experience))
        .route("/v1/users/me/experience/{id}", delete(handlers::professional::delete_experience))
        .route("/v1/users/{user_id}/certifications", get(handlers::professional::list_certifications))
        .route("/v1/users/me/certifications", post(handlers::professional::add_certification))
        .route("/v1/users/me/certifications/{id}", delete(handlers::professional::delete_certification))
        .route("/v1/users/{user_id}/projects", get(handlers::professional::list_projects))
        .route("/v1/users/me/projects", post(handlers::professional::add_project))
        .route("/v1/users/me/projects/{id}", delete(handlers::professional::delete_project))
        .route("/v1/users/{user_id}/mutual-friends", get(handlers::professional::mutual_friends))
        .route("/v1/users/birthdays", get(handlers::professional::birthdays_today))
        // User Content
        .route("/v1/users/{username}/posts", get(handlers::extras::user_posts))
        .route("/v1/users/{username}/photos", get(handlers::extras::user_photos))
        .route("/v1/users/{username}/videos", get(handlers::extras::user_videos))
        .route("/v1/users/{username}/skills", get(handlers::extras::user_skills))
        // Skills Search
        .route("/v1/skills/search", get(handlers::extras::search_skills))
        // LinkedIn Mode
        .route("/v1/users/me/open-to-work", post(handlers::extras::set_open_to_work))
        .route("/v1/users/me/open-to-work", delete(handlers::extras::unset_open_to_work))
        .route("/v1/users/me/providing-service", post(handlers::extras::set_providing_service))
        .route("/v1/users/me/providing-service", delete(handlers::extras::unset_providing_service))
        // Nearby
        .route("/v1/users/nearby", get(handlers::extras::nearby_users))
        // Skills CRUD
        .route("/v1/users/me/skills", post(handlers::extras::add_skill))
        .route("/v1/users/me/skills/{id}", delete(handlers::extras::remove_skill))
        // Avatar Reset
        .route("/v1/users/me/avatar/reset", post(handlers::extras::reset_avatar))
        // Custom Profile Fields
        .route("/v1/users/me/fields", get(handlers::extras::get_my_field_values))
        .route("/v1/users/me/fields", put(handlers::extras::update_my_field_values))
        .route("/v1/users/{user_id}/fields", get(handlers::extras::get_user_field_values))
        // GDPR Data Export
        .route("/v1/users/me/download-info", post(handlers::extras::download_my_info))
        // Common Things (mutual friends, same city, etc.)
        .route("/v1/users/{user_id}/common", get(handlers::extras::common_things))
        // Unified Reports
        .route("/v1/reports", post(handlers::extras::create_report))
        // AdMob Points
        .route("/v1/points/admob", post(handlers::extras::record_admob_points))
        // Mentions autocomplete
        .route("/v1/mentions", get(handlers::search::mention_search))
        // User Activities
        .route("/v1/activities", get(handlers::search::list_my_activities))
        // User Location
        .route("/v1/users/me/location", put(handlers::search::update_location))
        // Batch user lookup
        .route("/v1/users/batch", post(handlers::extras::batch_users))
        .route("/v1/users/by-phone", get(handlers::extras::get_user_by_phone))
        // Presence / last seen
        .route("/v1/users/me/lastseen", put(handlers::extras::update_lastseen))
        // Referrals & Inviters
        .route("/v1/users/me/referrals", get(handlers::extras::my_referrals))
        .route("/v1/users/me/inviters", get(handlers::extras::my_inviters))
        // Onboarding
        .route("/v1/users/me/onboarding/skip", post(handlers::extras::skip_onboarding_step))
        // Recent search registration
        .route("/v1/search/register", post(handlers::extras::register_recent_search))
        // Contact form
        .route("/v1/contact", post(handlers::extras::contact_us))
        // General data batch fetch (mobile app startup)
        .route("/v1/general", post(handlers::extras::get_general_data))
        // User addresses
        .route("/v1/users/me/addresses", get(handlers::addresses::list_addresses))
        .route("/v1/users/me/addresses", post(handlers::addresses::create_address))
        .route("/v1/users/me/addresses/{id}", get(handlers::addresses::get_address))
        .route("/v1/users/me/addresses/{id}", put(handlers::addresses::update_address))
        .route("/v1/users/me/addresses/{id}", delete(handlers::addresses::delete_address))
        // Social links
        .route("/v1/users/me/social-links", put(handlers::profile::update_social_links))
        .route("/v1/users/{username}/social-links", get(handlers::profile::get_social_links))
        // Settings
        .route("/v1/users/me/privacy", get(handlers::settings::get_privacy_settings))
        .route("/v1/users/me/privacy", put(handlers::settings::update_privacy_settings))
        .route("/v1/users/me/notification-settings", get(handlers::settings::get_notification_settings))
        .route("/v1/users/me/notification-settings", put(handlers::settings::update_notification_settings))
        .route("/v1/users/me/invite-code", get(handlers::settings::get_my_invite_code))
        // Verification
        .route("/v1/users/me/verification-request", post(handlers::extras::request_verification))
        // Health
        .route("/health", get(handlers::health::health_check))
        .with_state(state)
}
