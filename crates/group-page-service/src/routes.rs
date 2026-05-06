use crate::handlers;
use axum::{
    Router,
    routing::{delete, get, post, put},
};
use shared::auth::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // ── Pages ──
        .route(
            "/v1/pages",
            get(handlers::pages::list_pages).post(handlers::pages::create_page),
        )
        .route(
            "/v1/pages/categories",
            get(handlers::pages::list_categories),
        )
        .route("/v1/pages/search", get(handlers::pages::search_pages))
        .route("/v1/pages/suggested", get(handlers::pages::suggested_pages))
        .route("/v1/pages/my", get(handlers::pages::my_pages))
        .route("/v1/pages/liked", get(handlers::pages::liked_pages))
        .route("/v1/pages/nearby", get(handlers::pages::nearby_pages))
        .route(
            "/v1/pages/{id}",
            get(handlers::pages::get_page)
                .put(handlers::pages::update_page)
                .delete(handlers::pages::delete_page),
        )
        .route(
            "/v1/pages/{id}/like",
            post(handlers::pages::like_page).delete(handlers::pages::unlike_page),
        )
        .route("/v1/pages/{id}/rate", post(handlers::pages::rate_page))
        .route("/v1/pages/{id}/likes", get(handlers::pages::page_likers))
        .route(
            "/v1/pages/{id}/admins",
            get(handlers::pages::list_admins).post(handlers::pages::add_admin),
        )
        .route(
            "/v1/pages/{id}/admins/{user_id}",
            delete(handlers::pages::remove_admin),
        )
        .route("/v1/pages/{id}/posts", get(handlers::extras::page_posts))
        .route(
            "/v1/pages/{id}/invite",
            post(handlers::extras::invite_page_like),
        )
        .route(
            "/v1/pages/{id}/avatar",
            put(handlers::extras::update_page_avatar),
        )
        .route(
            "/v1/pages/{id}/cover",
            put(handlers::extras::update_page_cover),
        )
        .route("/v1/pages/{id}/boost", post(handlers::extras::boost_page))
        .route(
            "/v1/pages/{id}/verify",
            post(handlers::extras::request_page_verification),
        )
        .route(
            "/v1/pages/{id}/ratings",
            get(handlers::extras::list_page_ratings),
        )
        .route(
            "/v1/pages/{id}/non-likes",
            get(handlers::extras::page_non_likers),
        )
        .route(
            "/v1/pages/check-name",
            get(handlers::extras::check_page_name),
        )
        // ── Groups ──
        .route("/v1/groups", post(handlers::groups::create_group))
        .route(
            "/v1/groups/categories",
            get(handlers::groups::list_categories),
        )
        .route("/v1/groups/search", get(handlers::groups::search_groups))
        .route(
            "/v1/groups/suggested",
            get(handlers::groups::suggested_groups),
        )
        .route("/v1/groups/my", get(handlers::groups::my_groups))
        .route("/v1/groups/joined", get(handlers::groups::joined_groups))
        .route(
            "/v1/groups/{id}",
            get(handlers::groups::get_group)
                .put(handlers::groups::update_group)
                .delete(handlers::groups::delete_group),
        )
        .route(
            "/v1/groups/{id}/join",
            post(handlers::groups::join_group).delete(handlers::groups::leave_group),
        )
        .route(
            "/v1/groups/{id}/members",
            get(handlers::groups::list_members),
        )
        .route(
            "/v1/groups/{id}/members/{uid}",
            delete(handlers::groups::kick_member),
        )
        .route(
            "/v1/groups/{id}/members/{uid}/role",
            post(handlers::groups::change_role),
        )
        .route(
            "/v1/groups/{id}/join-requests",
            get(handlers::groups::join_requests),
        )
        .route(
            "/v1/groups/{id}/join-requests/{rid}/accept",
            post(handlers::groups::accept_join),
        )
        .route(
            "/v1/groups/{id}/join-requests/{rid}/reject",
            post(handlers::groups::reject_join),
        )
        .route("/v1/groups/{id}/posts", get(handlers::extras::group_posts))
        .route(
            "/v1/groups/{id}/invite",
            post(handlers::extras::invite_group_join),
        )
        .route(
            "/v1/groups/{id}/avatar",
            put(handlers::extras::update_group_avatar),
        )
        .route(
            "/v1/groups/{id}/cover",
            put(handlers::extras::update_group_cover),
        )
        .route(
            "/v1/groups/{id}/non-members",
            get(handlers::extras::group_non_members),
        )
        .route(
            "/v1/groups/check-name",
            get(handlers::extras::check_group_name),
        )
        .route(
            "/v1/groups/{id}/rules",
            get(handlers::groups::list_group_rules)
                .post(handlers::groups::create_group_rule),
        )
        .route(
            "/v1/groups/{gid}/rules/{rid}",
            delete(handlers::groups::delete_group_rule),
        )
        // ── Events ──
        .route("/v1/events", post(handlers::events::create_event))
        .route(
            "/v1/events/upcoming",
            get(handlers::events::upcoming_events),
        )
        .route("/v1/events/my", get(handlers::events::my_events))
        .route(
            "/v1/events/attending",
            get(handlers::events::attending_events),
        )
        .route("/v1/events/going", get(handlers::events::going_events))
        .route(
            "/v1/events/interested",
            get(handlers::events::interested_events),
        )
        .route("/v1/events/invited", get(handlers::events::invited_events))
        .route("/v1/events/past", get(handlers::events::past_events))
        .route(
            "/v1/events/{id}",
            get(handlers::events::get_event)
                .put(handlers::events::update_event)
                .delete(handlers::events::delete_event),
        )
        .route(
            "/v1/events/{id}/respond",
            post(handlers::events::respond_event),
        )
        .route("/v1/events/{id}/going", get(handlers::events::list_going))
        .route(
            "/v1/events/{id}/interested",
            get(handlers::events::list_interested),
        )
        .route(
            "/v1/events/{id}/invite",
            post(handlers::events::invite_users),
        )
        .route("/v1/events/{id}/posts", get(handlers::extras::event_posts))
        .route(
            "/v1/events/{id}/cover",
            put(handlers::extras::update_event_cover),
        )
        .route(
            "/v1/events/{id}/cohosts",
            post(handlers::events::add_event_cohost),
        )
        .route(
            "/v1/events/{eid}/cohosts/{uid}",
            delete(handlers::events::remove_event_cohost),
        )
        .route(
            "/v1/events/{id}/discussion",
            get(handlers::events::list_event_discussion)
                .post(handlers::events::add_event_discussion),
        )
        // ── Event Tickets ──
        .route(
            "/v1/events/{id}/tickets",
            post(handlers::events::purchase_ticket).get(handlers::events::list_my_tickets),
        )
        // Plan §3.5 E1 — calendar export.
        .route(
            "/v1/events/{id}/ics",
            get(handlers::analytics_extras::export_event_ics),
        )
        // Plan §3.5 G1 — per-group analytics dashboard.
        .route(
            "/v1/groups/{id}/analytics",
            get(handlers::analytics_extras::group_analytics),
        )
        // Plan §3.5 PG1 — page autoresponder (messages).
        .route(
            "/v1/pages/{id}/autoresponder",
            get(handlers::analytics_extras::get_autoresponder)
                .put(handlers::analytics_extras::put_autoresponder),
        )
        // ── Boosted ──
        .route("/v1/boosted/pages", get(handlers::extras::my_boosted_pages))
        // ── Health ──
        .route("/health", get(handlers::health::health_check))
        .with_state(state)
}
