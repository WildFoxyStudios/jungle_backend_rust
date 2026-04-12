use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "WoWonder Backend API",
        version = "2.0.0",
        description = "WoWonder Social Network — Rust Backend API. 519 endpoints across 16 microservices.",
        contact(name = "WoWonder", url = "https://wowonder.com"),
        license(name = "Proprietary")
    ),
    servers(
        (url = "/", description = "API Gateway")
    ),
    tags(
        (name = "auth", description = "Authentication, sessions, 2FA, OAuth"),
        (name = "users", description = "User profiles, relationships, settings"),
        (name = "posts", description = "Posts, feed, reactions, comments, reels, hashtags, ads, live"),
        (name = "messaging", description = "Conversations, messages, broadcasts, calls"),
        (name = "media", description = "File uploads, stories, media processing"),
        (name = "notifications", description = "Notifications, push tokens, announcements"),
        (name = "groups", description = "Groups management"),
        (name = "pages", description = "Pages management"),
        (name = "events", description = "Events management"),
        (name = "content", description = "Blogs, forums, movies, games"),
        (name = "commerce", description = "Products, orders, jobs, funding, offers, gifts, stickers"),
        (name = "payments", description = "Payment gateways, wallet, subscriptions"),
        (name = "admin", description = "Admin panel — dashboard, users, config, moderation"),
        (name = "ai", description = "AI features — chat, suggestions, image description"),
        (name = "realtime", description = "WebSocket, presence, internal relay"),
    ),
    paths(),
    components(schemas())
)]
pub struct ApiDoc;

pub fn openapi_spec() -> utoipa::openapi::OpenApi {
    let mut doc = ApiDoc::openapi();

    add_auth_paths(&mut doc);
    add_user_paths(&mut doc);
    add_post_paths(&mut doc);
    add_messaging_paths(&mut doc);
    add_media_paths(&mut doc);
    add_notification_paths(&mut doc);
    add_group_paths(&mut doc);
    add_page_paths(&mut doc);
    add_event_paths(&mut doc);
    add_content_paths(&mut doc);
    add_commerce_paths(&mut doc);
    add_payment_paths(&mut doc);
    add_admin_paths(&mut doc);
    add_ai_paths(&mut doc);
    add_realtime_paths(&mut doc);

    doc
}

fn p(method: &str, path_str: &str, tag: &str, summary: &str, op_id: &str) -> (String, utoipa::openapi::PathItem) {
    use utoipa::openapi::path::*;
    use utoipa::openapi::*;

    let op = OperationBuilder::new()
        .tag(tag)
        .summary(Some(summary.to_string()))
        .operation_id(Some(op_id.to_string()))
        .response("200", ResponseBuilder::new().description("Success").build())
        .build();

    let item = match method {
        "POST" => PathItem::new(HttpMethod::Post, op),
        "PUT" => PathItem::new(HttpMethod::Put, op),
        "DELETE" => PathItem::new(HttpMethod::Delete, op),
        "PATCH" => PathItem::new(HttpMethod::Patch, op),
        _ => PathItem::new(HttpMethod::Get, op),
    };

    (path_str.to_string(), item)
}

fn insert_all(doc: &mut utoipa::openapi::OpenApi, paths: Vec<(String, utoipa::openapi::PathItem)>) {
    for (path, item) in paths {
        doc.paths.paths.insert(path, item);
    }
}

// ── Auth Service (33 endpoints) ──────────────────────────────────────────────

fn add_auth_paths(doc: &mut utoipa::openapi::OpenApi) {
    insert_all(doc, vec![
        p("POST", "/v1/auth/register", "auth", "Register new user", "auth_register"),
        p("POST", "/v1/auth/login", "auth", "Login", "auth_login"),
        p("POST", "/v1/auth/refresh", "auth", "Refresh JWT token", "auth_refresh"),
        p("POST", "/v1/auth/logout", "auth", "Logout", "auth_logout"),
        p("GET", "/v1/auth/me", "auth", "Get current authenticated user", "auth_me"),
        p("POST", "/v1/auth/forgot-password", "auth", "Request password reset", "auth_forgot_pw"),
        p("POST", "/v1/auth/reset-password", "auth", "Reset password with token", "auth_reset_pw"),
        p("POST", "/v1/auth/verify-email", "auth", "Verify email with code", "auth_verify_email"),
        p("POST", "/v1/auth/verify-phone", "auth", "Verify phone with code", "auth_verify_phone"),
        p("POST", "/v1/auth/resend-code", "auth", "Resend verification code", "auth_resend"),
        p("POST", "/v1/auth/2fa/setup", "auth", "Setup 2FA (get QR code)", "auth_2fa_setup"),
        p("POST", "/v1/auth/2fa/enable", "auth", "Enable 2FA after verification", "auth_2fa_enable"),
        p("POST", "/v1/auth/2fa/verify", "auth", "Verify 2FA token", "auth_2fa_verify"),
        p("POST", "/v1/auth/2fa/disable", "auth", "Disable 2FA", "auth_2fa_disable"),
        p("GET", "/v1/auth/2fa/backup-codes", "auth", "Get backup codes", "auth_2fa_backup"),
        p("POST", "/v1/auth/2fa/backup-codes/regenerate", "auth", "Regenerate backup codes", "auth_2fa_regen"),
        p("POST", "/v1/auth/social/login", "auth", "Social login (14 providers)", "auth_social"),
        p("POST", "/v1/auth/switch-account", "auth", "Switch between accounts", "auth_switch"),
        p("GET", "/v1/auth/sessions", "auth", "List active sessions", "auth_sessions"),
        p("DELETE", "/v1/auth/sessions/{id}", "auth", "Revoke session", "auth_revoke_session"),
        p("POST", "/v1/auth/sessions/revoke-all", "auth", "Revoke all sessions", "auth_revoke_all"),
        p("GET", "/v1/oauth/apps", "auth", "List OAuth apps", "oauth_list"),
        p("POST", "/v1/oauth/apps", "auth", "Create OAuth app", "oauth_create"),
        p("GET", "/v1/oauth/apps/{id}", "auth", "Get OAuth app", "oauth_get"),
        p("PUT", "/v1/oauth/apps/{id}", "auth", "Update OAuth app", "oauth_update"),
        p("DELETE", "/v1/oauth/apps/{id}", "auth", "Delete OAuth app", "oauth_delete"),
        p("GET", "/v1/oauth/apps/{id}/permissions", "auth", "Get app permissions", "oauth_perms"),
        p("POST", "/v1/oauth/authorize", "auth", "Authorize OAuth app", "oauth_authorize"),
        p("POST", "/v1/oauth/token", "auth", "Exchange OAuth token", "oauth_token"),
        p("POST", "/v1/oauth/revoke", "auth", "Revoke OAuth token", "oauth_revoke"),
        p("GET", "/v1/translations/{lang}", "auth", "Get translations", "public_translations"),
        p("GET", "/v1/config/public", "auth", "Get public site config", "public_config"),
    ]);
}

// ── User Service (57 endpoints) ──────────────────────────────────────────────

fn add_user_paths(doc: &mut utoipa::openapi::OpenApi) {
    insert_all(doc, vec![
        p("GET", "/v1/users/me", "users", "Get my profile", "users_me"),
        p("PUT", "/v1/users/me", "users", "Update my profile", "users_update"),
        p("DELETE", "/v1/users/me", "users", "Delete my account", "users_delete"),
        p("PUT", "/v1/users/me/avatar", "users", "Update avatar", "users_avatar"),
        p("PUT", "/v1/users/me/cover", "users", "Update cover photo", "users_cover"),
        p("GET", "/v1/users/{username}", "users", "Get user by username", "users_get"),
        p("GET", "/v1/users/search", "users", "Search users", "users_search"),
        p("GET", "/v1/users/suggestions", "users", "User suggestions", "users_suggest"),
        p("GET", "/v1/users/pro-users", "users", "List pro users", "users_pro"),
        p("POST", "/v1/social/follow/{user_id}", "users", "Follow user", "social_follow"),
        p("DELETE", "/v1/social/follow/{user_id}", "users", "Unfollow user", "social_unfollow"),
        p("GET", "/v1/users/{username}/followers", "users", "Get followers", "users_followers"),
        p("GET", "/v1/users/{username}/following", "users", "Get following", "users_following"),
        p("GET", "/v1/social/follow-requests", "users", "List follow requests", "social_freq_list"),
        p("POST", "/v1/social/follow-requests/{id}/accept", "users", "Accept follow request", "social_freq_accept"),
        p("POST", "/v1/social/follow-requests/{id}/reject", "users", "Reject follow request", "social_freq_reject"),
        p("POST", "/v1/social/block/{user_id}", "users", "Block user", "social_block"),
        p("DELETE", "/v1/social/block/{user_id}", "users", "Unblock user", "social_unblock"),
        p("POST", "/v1/social/poke/{user_id}", "users", "Poke user", "social_poke"),
        p("POST", "/v1/social/mute/{user_id}", "users", "Mute user", "social_mute"),
        p("DELETE", "/v1/social/mute/{user_id}", "users", "Unmute user", "social_unmute"),
        p("POST", "/v1/social/family/{user_id}", "users", "Send family request", "social_family"),
        p("PUT", "/v1/social/family/{id}", "users", "Respond to family request", "social_family_resp"),
        p("POST", "/v1/social/stop-notify/{user_id}", "users", "Stop notifications from user", "social_stop_notify"),
        p("GET", "/v1/users/{user_id}/experience", "users", "List experience", "users_exp"),
        p("POST", "/v1/users/me/experience", "users", "Add experience", "users_exp_add"),
        p("DELETE", "/v1/users/me/experience/{id}", "users", "Delete experience", "users_exp_del"),
        p("GET", "/v1/users/{user_id}/certifications", "users", "List certifications", "users_cert"),
        p("POST", "/v1/users/me/certifications", "users", "Add certification", "users_cert_add"),
        p("DELETE", "/v1/users/me/certifications/{id}", "users", "Delete certification", "users_cert_del"),
        p("GET", "/v1/users/{user_id}/projects", "users", "List projects", "users_proj"),
        p("POST", "/v1/users/me/projects", "users", "Add project", "users_proj_add"),
        p("DELETE", "/v1/users/me/projects/{id}", "users", "Delete project", "users_proj_del"),
        p("GET", "/v1/users/{user_id}/mutual-friends", "users", "Mutual friends", "users_mutual"),
        p("GET", "/v1/users/birthdays", "users", "Birthdays today", "users_bdays"),
        p("GET", "/v1/users/{username}/posts", "users", "User posts", "users_posts"),
        p("GET", "/v1/users/{username}/photos", "users", "User photos", "users_photos"),
        p("GET", "/v1/users/{username}/videos", "users", "User videos", "users_videos"),
        p("GET", "/v1/users/{username}/skills", "users", "User skills", "users_skills"),
        p("GET", "/v1/skills/search", "users", "Search skills", "skills_search"),
        p("POST", "/v1/users/me/open-to-work", "users", "Set open to work", "users_otw"),
        p("DELETE", "/v1/users/me/open-to-work", "users", "Unset open to work", "users_otw_off"),
        p("POST", "/v1/users/me/providing-service", "users", "Set providing service", "users_ps"),
        p("DELETE", "/v1/users/me/providing-service", "users", "Unset providing service", "users_ps_off"),
        p("GET", "/v1/users/nearby", "users", "Nearby users", "users_nearby"),
        p("POST", "/v1/users/me/skills", "users", "Add skill", "users_skill_add"),
        p("DELETE", "/v1/users/me/skills/{id}", "users", "Remove skill", "users_skill_del"),
        p("POST", "/v1/users/me/avatar/reset", "users", "Reset avatar", "users_avatar_reset"),
        p("GET", "/v1/users/me/fields", "users", "Get my custom field values", "users_fields"),
        p("PUT", "/v1/users/me/fields", "users", "Update my custom field values", "users_fields_upd"),
        p("GET", "/v1/users/{user_id}/fields", "users", "Get user field values", "users_fields_get"),
        p("GET", "/v1/users/me/privacy", "users", "Get privacy settings", "users_privacy"),
        p("PUT", "/v1/users/me/privacy", "users", "Update privacy settings", "users_privacy_upd"),
        p("GET", "/v1/users/me/notification-settings", "users", "Get notification settings", "users_notif_settings"),
        p("PUT", "/v1/users/me/notification-settings", "users", "Update notification settings", "users_notif_settings_upd"),
        p("GET", "/v1/users/me/invite-code", "users", "Get my invite code", "users_invite"),
    ]);
}

// ── Post Service (56 endpoints) ──────────────────────────────────────────────

fn add_post_paths(doc: &mut utoipa::openapi::OpenApi) {
    insert_all(doc, vec![
        p("GET", "/v1/feed", "posts", "Get news feed", "feed"),
        p("POST", "/v1/posts", "posts", "Create post", "posts_create"),
        p("GET", "/v1/posts/{id}", "posts", "Get post", "posts_get"),
        p("PUT", "/v1/posts/{id}", "posts", "Update post", "posts_update"),
        p("DELETE", "/v1/posts/{id}", "posts", "Delete post", "posts_delete"),
        p("POST", "/v1/posts/{id}/react", "posts", "React to post", "posts_react"),
        p("DELETE", "/v1/posts/{id}/react", "posts", "Remove reaction", "posts_unreact"),
        p("GET", "/v1/posts/{id}/comments", "posts", "Get comments", "posts_comments"),
        p("POST", "/v1/posts/{id}/comments", "posts", "Create comment", "posts_comment"),
        p("PUT", "/v1/comments/{id}", "posts", "Update comment", "comments_update"),
        p("DELETE", "/v1/comments/{id}", "posts", "Delete comment", "comments_delete"),
        p("GET", "/v1/comments/{id}/replies", "posts", "Get replies", "comments_replies"),
        p("POST", "/v1/comments/{id}/replies", "posts", "Create reply", "comments_reply"),
        p("POST", "/v1/comments/{id}/react", "posts", "React to comment", "comments_react"),
        p("POST", "/v1/posts/{id}/save", "posts", "Save post", "posts_save"),
        p("DELETE", "/v1/posts/{id}/save", "posts", "Unsave post", "posts_unsave"),
        p("POST", "/v1/posts/{id}/hide", "posts", "Hide post", "posts_hide"),
        p("GET", "/v1/reels", "posts", "Get reels feed", "reels_feed"),
        p("POST", "/v1/reels", "posts", "Create reel", "reels_create"),
        p("GET", "/v1/reels/{id}", "posts", "Get reel", "reels_get"),
        p("DELETE", "/v1/reels/{id}", "posts", "Delete reel", "reels_delete"),
        p("POST", "/v1/reels/{id}/view", "posts", "View reel", "reels_view"),
        p("POST", "/v1/reels/{id}/react", "posts", "React to reel", "reels_react"),
        p("GET", "/v1/reels/{id}/comments", "posts", "Reel comments", "reels_comments"),
        p("POST", "/v1/reels/{id}/comments", "posts", "Add reel comment", "reels_comment"),
        p("GET", "/v1/search", "posts", "Global search", "search"),
        p("GET", "/v1/search/recent", "posts", "Recent searches", "search_recent"),
        p("POST", "/v1/search/recent", "posts", "Save recent search", "search_save"),
        p("DELETE", "/v1/search/recent", "posts", "Clear recent searches", "search_clear"),
        p("POST", "/v1/posts/{id}/share", "posts", "Share post", "posts_share"),
        p("GET", "/v1/hashtags/trending", "posts", "Trending hashtags", "hashtags_trend"),
        p("GET", "/v1/hashtags/search", "posts", "Search hashtags", "hashtags_search"),
        p("GET", "/v1/hashtags/{tag}/posts", "posts", "Posts by hashtag", "hashtags_posts"),
        p("POST", "/v1/ads", "posts", "Create ad", "ads_create"),
        p("GET", "/v1/ads/my", "posts", "My ads", "ads_my"),
        p("GET", "/v1/ads/{id}/stats", "posts", "Ad stats", "ads_stats"),
        p("DELETE", "/v1/ads/{id}", "posts", "Cancel ad", "ads_cancel"),
        p("PUT", "/v1/ads/{id}", "posts", "Update ad", "ads_update"),
        p("POST", "/v1/ads/{id}/click", "posts", "Ad click", "ads_click"),
        p("POST", "/v1/posts/{id}/poll/vote", "posts", "Vote on poll", "poll_vote"),
        p("POST", "/v1/posts/{id}/pin", "posts", "Pin post", "posts_pin"),
        p("DELETE", "/v1/posts/{id}/pin", "posts", "Unpin post", "posts_unpin"),
        p("POST", "/v1/posts/{id}/boost", "posts", "Boost post", "posts_boost"),
        p("POST", "/v1/posts/{id}/report", "posts", "Report post", "posts_report"),
        p("GET", "/v1/feed/explore", "posts", "Explore feed", "feed_explore"),
        p("GET", "/v1/memories", "posts", "On this day memories", "memories"),
        p("GET", "/v1/boosted/posts", "posts", "My boosted posts", "boosted_posts"),
        p("GET", "/v1/posts/colored-templates", "posts", "Colored post templates", "colored_tpl"),
        p("GET", "/v1/posts/reaction-types", "posts", "Available reaction types", "reaction_types"),
        p("POST", "/v1/live/start", "posts", "Start live stream", "live_start"),
        p("POST", "/v1/live/stop", "posts", "Stop live stream", "live_stop"),
        p("GET", "/v1/live/active", "posts", "Active live streams", "live_active"),
        p("GET", "/v1/live/friends", "posts", "Friends live", "live_friends"),
        p("POST", "/v1/live/{id}/comment", "posts", "Live comment", "live_comment"),
        p("POST", "/v1/live/{id}/react", "posts", "Live react", "live_react"),
    ]);
}

// ── Messaging Service (35 endpoints) ─────────────────────────────────────────

fn add_messaging_paths(doc: &mut utoipa::openapi::OpenApi) {
    insert_all(doc, vec![
        p("GET", "/v1/conversations", "messaging", "List conversations", "conv_list"),
        p("POST", "/v1/conversations", "messaging", "Create conversation", "conv_create"),
        p("POST", "/v1/conversations/group", "messaging", "Create group conversation", "conv_group"),
        p("GET", "/v1/conversations/pinned", "messaging", "List pinned conversations", "conv_pinned"),
        p("GET", "/v1/conversations/archived", "messaging", "List archived conversations", "conv_archived"),
        p("GET", "/v1/conversations/{id}", "messaging", "Get conversation", "conv_get"),
        p("DELETE", "/v1/conversations/{id}", "messaging", "Delete conversation", "conv_delete"),
        p("PUT", "/v1/conversations/{id}/color", "messaging", "Update conversation color", "conv_color"),
        p("POST", "/v1/conversations/{id}/pin", "messaging", "Pin conversation", "conv_pin"),
        p("DELETE", "/v1/conversations/{id}/pin", "messaging", "Unpin conversation", "conv_unpin"),
        p("POST", "/v1/conversations/{id}/archive", "messaging", "Archive conversation", "conv_archive"),
        p("DELETE", "/v1/conversations/{id}/archive", "messaging", "Unarchive conversation", "conv_unarchive"),
        p("POST", "/v1/conversations/{id}/read", "messaging", "Mark conversation read", "conv_read"),
        p("PUT", "/v1/conversations/group/{id}", "messaging", "Update group conversation", "conv_group_upd"),
        p("GET", "/v1/conversations/{id}/messages", "messaging", "List messages", "msg_list"),
        p("POST", "/v1/conversations/{id}/messages", "messaging", "Send message", "msg_send"),
        p("POST", "/v1/conversations/{id}/typing", "messaging", "Typing indicator", "msg_typing"),
        p("DELETE", "/v1/messages/{id}", "messaging", "Delete message", "msg_delete"),
        p("POST", "/v1/messages/{id}/favorite", "messaging", "Toggle message favorite", "msg_fav"),
        p("POST", "/v1/messages/{id}/pin", "messaging", "Pin message", "msg_pin"),
        p("DELETE", "/v1/messages/{id}/pin", "messaging", "Unpin message", "msg_unpin"),
        p("POST", "/v1/messages/{id}/forward", "messaging", "Forward message", "msg_forward"),
        p("GET", "/v1/broadcasts", "messaging", "List broadcasts", "bc_list"),
        p("POST", "/v1/broadcasts", "messaging", "Create broadcast", "bc_create"),
        p("PUT", "/v1/broadcasts/{id}", "messaging", "Update broadcast", "bc_update"),
        p("DELETE", "/v1/broadcasts/{id}", "messaging", "Delete broadcast", "bc_delete"),
        p("GET", "/v1/broadcasts/{id}/members", "messaging", "List broadcast members", "bc_members"),
        p("POST", "/v1/broadcasts/{id}/members", "messaging", "Add broadcast members", "bc_add_members"),
        p("DELETE", "/v1/broadcasts/{id}/members/{user_id}", "messaging", "Remove broadcast member", "bc_rm_member"),
        p("POST", "/v1/broadcasts/{id}/send", "messaging", "Send broadcast", "bc_send"),
        p("GET", "/v1/calls", "messaging", "List calls", "calls_list"),
        p("POST", "/v1/calls", "messaging", "Create call", "calls_create"),
        p("GET", "/v1/calls/{id}", "messaging", "Get call", "calls_get"),
        p("PUT", "/v1/calls/{id}/status", "messaging", "Update call status", "calls_status"),
    ]);
}

// ── Media Service (18 endpoints) ─────────────────────────────────────────────

fn add_media_paths(doc: &mut utoipa::openapi::OpenApi) {
    insert_all(doc, vec![
        p("POST", "/v1/media/upload", "media", "Upload media file", "media_upload"),
        p("POST", "/v1/media/upload/avatar", "media", "Upload avatar", "media_avatar"),
        p("POST", "/v1/media/upload/cover", "media", "Upload cover photo", "media_cover"),
        p("GET", "/v1/media/{id}", "media", "Get media info", "media_get"),
        p("DELETE", "/v1/media/{id}", "media", "Delete media", "media_delete"),
        p("GET", "/v1/media/my", "media", "My uploaded media", "media_my"),
        p("GET", "/v1/stories", "media", "List stories feed", "stories_feed"),
        p("POST", "/v1/stories", "media", "Create story", "stories_create"),
        p("GET", "/v1/stories/{id}", "media", "Get story", "stories_get"),
        p("DELETE", "/v1/stories/{id}", "media", "Delete story", "stories_delete"),
        p("POST", "/v1/stories/{id}/view", "media", "View story", "stories_view"),
        p("GET", "/v1/stories/{id}/viewers", "media", "Story viewers", "stories_viewers"),
        p("GET", "/v1/stories/my", "media", "My stories", "stories_my"),
        p("GET", "/v1/stories/archive", "media", "Archived stories", "stories_archive"),
        p("POST", "/v1/stories/{id}/react", "media", "React to story", "stories_react"),
        p("GET", "/v1/stories/{id}/reactions", "media", "Story reactions", "stories_reactions"),
        p("POST", "/v1/stories/{id}/reply", "media", "Reply to story", "stories_reply"),
    ]);
}

// ── Notification Service (16 endpoints) ──────────────────────────────────────

fn add_notification_paths(doc: &mut utoipa::openapi::OpenApi) {
    insert_all(doc, vec![
        p("GET", "/v1/notifications", "notifications", "List notifications", "notif_list"),
        p("GET", "/v1/notifications/unread-count", "notifications", "Unread count", "notif_count"),
        p("POST", "/v1/notifications/read-all", "notifications", "Mark all read", "notif_read_all"),
        p("POST", "/v1/notifications/{id}/read", "notifications", "Mark notification read", "notif_read"),
        p("DELETE", "/v1/notifications/{id}", "notifications", "Delete notification", "notif_delete"),
        p("DELETE", "/v1/notifications/clear", "notifications", "Clear all notifications", "notif_clear"),
        p("GET", "/v1/notifications/preferences", "notifications", "Get notification preferences", "notif_prefs"),
        p("PUT", "/v1/notifications/preferences", "notifications", "Update notification preferences", "notif_prefs_upd"),
        p("POST", "/v1/notifications/push-tokens", "notifications", "Register push token", "push_register"),
        p("GET", "/v1/notifications/push-tokens", "notifications", "List push tokens", "push_list"),
        p("DELETE", "/v1/notifications/push-tokens/{token}", "notifications", "Unregister push token", "push_unreg"),
        p("GET", "/v1/announcements", "notifications", "List active announcements", "announce_list"),
        p("POST", "/v1/announcements/{id}/dismiss", "notifications", "Dismiss announcement", "announce_dismiss"),
        p("POST", "/v1/newsletter/subscribe", "notifications", "Subscribe to newsletter", "nl_sub"),
        p("POST", "/v1/newsletter/unsubscribe", "notifications", "Unsubscribe from newsletter", "nl_unsub"),
    ]);
}

// ── Group-Page-Service: Groups (22 endpoints) ────────────────────────────────

fn add_group_paths(doc: &mut utoipa::openapi::OpenApi) {
    insert_all(doc, vec![
        p("POST", "/v1/groups", "groups", "Create group", "groups_create"),
        p("GET", "/v1/groups/categories", "groups", "Group categories", "groups_cats"),
        p("GET", "/v1/groups/search", "groups", "Search groups", "groups_search"),
        p("GET", "/v1/groups/suggested", "groups", "Suggested groups", "groups_suggested"),
        p("GET", "/v1/groups/my", "groups", "My groups", "groups_my"),
        p("GET", "/v1/groups/joined", "groups", "Joined groups", "groups_joined"),
        p("GET", "/v1/groups/{slug}", "groups", "Get group", "groups_get"),
        p("PUT", "/v1/groups/{id}", "groups", "Update group", "groups_update"),
        p("DELETE", "/v1/groups/{id}", "groups", "Delete group", "groups_delete"),
        p("POST", "/v1/groups/{id}/join", "groups", "Join group", "groups_join"),
        p("DELETE", "/v1/groups/{id}/join", "groups", "Leave group", "groups_leave"),
        p("GET", "/v1/groups/{id}/members", "groups", "List group members", "groups_members"),
        p("DELETE", "/v1/groups/{id}/members/{uid}", "groups", "Kick member", "groups_kick"),
        p("POST", "/v1/groups/{id}/members/{uid}/role", "groups", "Change member role", "groups_role"),
        p("GET", "/v1/groups/{id}/join-requests", "groups", "Join requests", "groups_reqs"),
        p("POST", "/v1/groups/{id}/join-requests/{rid}/accept", "groups", "Accept join request", "groups_accept"),
        p("POST", "/v1/groups/{id}/join-requests/{rid}/reject", "groups", "Reject join request", "groups_reject"),
        p("GET", "/v1/groups/{id}/posts", "groups", "Group posts", "groups_posts"),
        p("POST", "/v1/groups/{id}/invite", "groups", "Invite to group", "groups_invite"),
        p("PUT", "/v1/groups/{id}/avatar", "groups", "Update group avatar", "groups_avatar"),
        p("PUT", "/v1/groups/{id}/cover", "groups", "Update group cover", "groups_cover"),
    ]);
}

// ── Group-Page-Service: Pages (22 endpoints) ─────────────────────────────────

fn add_page_paths(doc: &mut utoipa::openapi::OpenApi) {
    insert_all(doc, vec![
        p("POST", "/v1/pages", "pages", "Create page", "pages_create"),
        p("GET", "/v1/pages/categories", "pages", "Page categories", "pages_cats"),
        p("GET", "/v1/pages/search", "pages", "Search pages", "pages_search"),
        p("GET", "/v1/pages/suggested", "pages", "Suggested pages", "pages_suggested"),
        p("GET", "/v1/pages/my", "pages", "My pages", "pages_my"),
        p("GET", "/v1/pages/liked", "pages", "Liked pages", "pages_liked"),
        p("GET", "/v1/pages/{slug}", "pages", "Get page", "pages_get"),
        p("PUT", "/v1/pages/{id}", "pages", "Update page", "pages_update"),
        p("DELETE", "/v1/pages/{id}", "pages", "Delete page", "pages_delete"),
        p("POST", "/v1/pages/{id}/like", "pages", "Like page", "pages_like"),
        p("DELETE", "/v1/pages/{id}/like", "pages", "Unlike page", "pages_unlike"),
        p("POST", "/v1/pages/{id}/rate", "pages", "Rate page", "pages_rate"),
        p("GET", "/v1/pages/{id}/likes", "pages", "Page likers", "pages_likers"),
        p("GET", "/v1/pages/{id}/admins", "pages", "Page admins", "pages_admins"),
        p("POST", "/v1/pages/{id}/admins", "pages", "Add page admin", "pages_admin_add"),
        p("DELETE", "/v1/pages/{id}/admins/{user_id}", "pages", "Remove page admin", "pages_admin_rm"),
        p("GET", "/v1/pages/{id}/posts", "pages", "Page posts", "pages_posts"),
        p("POST", "/v1/pages/{id}/invite", "pages", "Invite to like page", "pages_invite"),
        p("PUT", "/v1/pages/{id}/avatar", "pages", "Update page avatar", "pages_avatar"),
        p("PUT", "/v1/pages/{id}/cover", "pages", "Update page cover", "pages_cover"),
        p("POST", "/v1/pages/{id}/boost", "pages", "Boost page", "pages_boost"),
        p("POST", "/v1/pages/{id}/verify", "pages", "Request page verification", "pages_verify"),
        p("GET", "/v1/boosted/pages", "pages", "My boosted pages", "boosted_pages"),
    ]);
}

// ── Group-Page-Service: Events (14 endpoints) ────────────────────────────────

fn add_event_paths(doc: &mut utoipa::openapi::OpenApi) {
    insert_all(doc, vec![
        p("POST", "/v1/events", "events", "Create event", "events_create"),
        p("GET", "/v1/events/upcoming", "events", "Upcoming events", "events_upcoming"),
        p("GET", "/v1/events/my", "events", "My events", "events_my"),
        p("GET", "/v1/events/attending", "events", "Attending events", "events_attending"),
        p("GET", "/v1/events/{id}", "events", "Get event", "events_get"),
        p("PUT", "/v1/events/{id}", "events", "Update event", "events_update"),
        p("DELETE", "/v1/events/{id}", "events", "Delete event", "events_delete"),
        p("POST", "/v1/events/{id}/respond", "events", "Respond to event", "events_respond"),
        p("GET", "/v1/events/{id}/going", "events", "List going", "events_going"),
        p("GET", "/v1/events/{id}/interested", "events", "List interested", "events_interested"),
        p("POST", "/v1/events/{id}/invite", "events", "Invite to event", "events_invite"),
        p("GET", "/v1/events/{id}/posts", "events", "Event posts", "events_posts"),
        p("PUT", "/v1/events/{id}/cover", "events", "Update event cover", "events_cover"),
    ]);
}

// ── Content Service (38 endpoints) ───────────────────────────────────────────

fn add_content_paths(doc: &mut utoipa::openapi::OpenApi) {
    insert_all(doc, vec![
        p("GET", "/v1/blogs", "content", "List blogs", "blogs_list"),
        p("POST", "/v1/blogs", "content", "Create blog", "blogs_create"),
        p("GET", "/v1/blogs/search", "content", "Search blogs", "blogs_search"),
        p("GET", "/v1/blogs/my", "content", "My blogs", "blogs_my"),
        p("GET", "/v1/blogs/categories", "content", "Blog categories", "blogs_cats"),
        p("GET", "/v1/blogs/{id}", "content", "Get blog", "blogs_get"),
        p("PUT", "/v1/blogs/{id}", "content", "Update blog", "blogs_update"),
        p("DELETE", "/v1/blogs/{id}", "content", "Delete blog", "blogs_delete"),
        p("GET", "/v1/blogs/{id}/comments", "content", "Blog comments", "blogs_comments"),
        p("POST", "/v1/blogs/{id}/comments", "content", "Add blog comment", "blogs_comment"),
        p("DELETE", "/v1/blogs/comments/{id}", "content", "Delete blog comment", "blogs_comment_del"),
        p("POST", "/v1/blogs/upload-image", "content", "Upload blog image", "blogs_img"),
        p("POST", "/v1/blogs/{id}/react", "content", "React to blog", "blogs_react"),
        p("GET", "/v1/blogs/category/{id}", "content", "Blogs by category", "blogs_by_cat"),
        p("GET", "/v1/forums/sections", "content", "Forum sections", "forums_sections"),
        p("GET", "/v1/forums/{id}/threads", "content", "Forum threads", "forums_threads"),
        p("POST", "/v1/forums/{id}/threads", "content", "Create forum thread", "forums_thread_create"),
        p("GET", "/v1/forums/threads/{id}", "content", "Get thread", "forums_thread_get"),
        p("PUT", "/v1/forums/threads/{id}", "content", "Update thread", "forums_thread_upd"),
        p("DELETE", "/v1/forums/threads/{id}", "content", "Delete thread", "forums_thread_del"),
        p("GET", "/v1/forums/threads/{id}/replies", "content", "Thread replies", "forums_replies"),
        p("POST", "/v1/forums/threads/{id}/replies", "content", "Create thread reply", "forums_reply"),
        p("DELETE", "/v1/forums/replies/{id}", "content", "Delete forum reply", "forums_reply_del"),
        p("GET", "/v1/movies", "content", "List movies", "movies_list"),
        p("POST", "/v1/movies", "content", "Create movie", "movies_create"),
        p("GET", "/v1/movies/{id}", "content", "Get movie", "movies_get"),
        p("PUT", "/v1/movies/{id}", "content", "Update movie", "movies_update"),
        p("DELETE", "/v1/movies/{id}", "content", "Delete movie", "movies_delete"),
        p("GET", "/v1/movies/{id}/comments", "content", "Movie comments", "movies_comments"),
        p("POST", "/v1/movies/{id}/comments", "content", "Add movie comment", "movies_comment"),
        p("POST", "/v1/movies/{id}/react", "content", "React to movie", "movies_react"),
        p("GET", "/v1/games", "content", "List games", "games_list"),
        p("GET", "/v1/games/my", "content", "My games", "games_my"),
        p("GET", "/v1/games/{id}", "content", "Get game", "games_get"),
        p("POST", "/v1/games/{id}/play", "content", "Play game", "games_play"),
        p("GET", "/v1/pages/custom", "content", "List custom pages", "custom_pages"),
        p("GET", "/v1/pages/custom/{slug}", "content", "Get custom page", "custom_page"),
    ]);
}

// ── Commerce Service (52 endpoints) ──────────────────────────────────────────

fn add_commerce_paths(doc: &mut utoipa::openapi::OpenApi) {
    insert_all(doc, vec![
        p("GET", "/v1/products", "commerce", "List products", "products_list"),
        p("POST", "/v1/products", "commerce", "Create product", "products_create"),
        p("GET", "/v1/products/search", "commerce", "Search products", "products_search"),
        p("GET", "/v1/products/my", "commerce", "My products", "products_my"),
        p("GET", "/v1/products/categories", "commerce", "Product categories", "products_cats"),
        p("GET", "/v1/products/{id}", "commerce", "Get product", "products_get"),
        p("PUT", "/v1/products/{id}", "commerce", "Update product", "products_update"),
        p("DELETE", "/v1/products/{id}", "commerce", "Delete product", "products_delete"),
        p("GET", "/v1/products/{id}/reviews", "commerce", "Product reviews", "products_reviews"),
        p("POST", "/v1/products/{id}/reviews", "commerce", "Add product review", "products_review"),
        p("POST", "/v1/products/nearby", "commerce", "Nearby products", "products_nearby"),
        p("POST", "/v1/orders", "commerce", "Create order", "orders_create"),
        p("GET", "/v1/orders/my", "commerce", "My orders", "orders_my"),
        p("GET", "/v1/orders/sales", "commerce", "My sales", "orders_sales"),
        p("GET", "/v1/orders/{id}", "commerce", "Get order", "orders_get"),
        p("PUT", "/v1/orders/{id}/status", "commerce", "Update order status", "orders_status"),
        p("GET", "/v1/jobs", "commerce", "List jobs", "jobs_list"),
        p("POST", "/v1/jobs", "commerce", "Create job", "jobs_create"),
        p("GET", "/v1/jobs/my", "commerce", "My jobs", "jobs_my"),
        p("GET", "/v1/jobs/applied", "commerce", "Applied jobs", "jobs_applied"),
        p("GET", "/v1/jobs/search", "commerce", "Search jobs", "jobs_search"),
        p("GET", "/v1/jobs/categories", "commerce", "Job categories", "jobs_cats"),
        p("GET", "/v1/jobs/{id}", "commerce", "Get job", "jobs_get"),
        p("PUT", "/v1/jobs/{id}", "commerce", "Update job", "jobs_update"),
        p("DELETE", "/v1/jobs/{id}", "commerce", "Delete job", "jobs_delete"),
        p("POST", "/v1/jobs/{id}/apply", "commerce", "Apply to job", "jobs_apply"),
        p("GET", "/v1/jobs/{id}/applications", "commerce", "Job applications", "jobs_apps"),
        p("PUT", "/v1/jobs/applications/{id}/status", "commerce", "Update application status", "jobs_app_status"),
        p("GET", "/v1/fundings", "commerce", "List fundings", "funding_list"),
        p("POST", "/v1/fundings", "commerce", "Create funding", "funding_create"),
        p("GET", "/v1/fundings/my", "commerce", "My fundings", "funding_my"),
        p("GET", "/v1/fundings/{id}", "commerce", "Get funding", "funding_get"),
        p("PUT", "/v1/fundings/{id}", "commerce", "Update funding", "funding_update"),
        p("DELETE", "/v1/fundings/{id}", "commerce", "Delete funding", "funding_delete"),
        p("POST", "/v1/fundings/{id}/donate", "commerce", "Donate to funding", "funding_donate"),
        p("GET", "/v1/fundings/{id}/donations", "commerce", "Funding donations", "funding_donations"),
        p("GET", "/v1/offers", "commerce", "List offers", "offers_list"),
        p("POST", "/v1/offers", "commerce", "Create offer", "offers_create"),
        p("GET", "/v1/offers/my", "commerce", "My offers", "offers_my"),
        p("GET", "/v1/offers/nearby", "commerce", "Nearby offers", "offers_nearby"),
        p("GET", "/v1/offers/{id}", "commerce", "Get offer", "offers_get"),
        p("PUT", "/v1/offers/{id}", "commerce", "Update offer", "offers_update"),
        p("DELETE", "/v1/offers/{id}", "commerce", "Delete offer", "offers_delete"),
        p("GET", "/v1/gifts", "commerce", "List gifts", "gifts_list"),
        p("GET", "/v1/gifts/categories", "commerce", "Gift categories", "gifts_cats"),
        p("POST", "/v1/gifts/send/{recipient_id}", "commerce", "Send gift", "gifts_send"),
        p("GET", "/v1/gifts/received", "commerce", "My received gifts", "gifts_received"),
        p("GET", "/v1/stickers/packs", "commerce", "List sticker packs", "stickers_packs"),
        p("GET", "/v1/stickers/packs/{id}", "commerce", "Get sticker pack", "stickers_pack"),
        p("POST", "/v1/stickers/packs/{id}/purchase", "commerce", "Purchase sticker pack", "stickers_buy"),
        p("GET", "/v1/stickers/my", "commerce", "My sticker packs", "stickers_my"),
    ]);
}

// ── Payment Service (23 endpoints) ───────────────────────────────────────────

fn add_payment_paths(doc: &mut utoipa::openapi::OpenApi) {
    insert_all(doc, vec![
        p("POST", "/v1/payments/create", "payments", "Create payment", "pay_create"),
        p("POST", "/v1/payments/verify", "payments", "Verify payment", "pay_verify"),
        p("GET", "/v1/payments/history", "payments", "Payment history", "pay_history"),
        p("POST", "/v1/payments/refund", "payments", "Refund payment", "pay_refund"),
        p("GET", "/v1/payments/wallet/balance", "payments", "Get wallet balance", "wallet_balance"),
        p("POST", "/v1/payments/wallet/add", "payments", "Add funds to wallet", "wallet_add"),
        p("POST", "/v1/payments/wallet/transfer", "payments", "Transfer from wallet", "wallet_transfer"),
        p("POST", "/v1/payments/withdraw", "payments", "Request withdrawal", "pay_withdraw"),
        p("GET", "/v1/payments/withdrawals", "payments", "List withdrawals", "pay_withdrawals"),
        p("PUT", "/v1/payments/withdrawals/{id}/status", "payments", "Update withdrawal status", "pay_wd_status"),
        p("GET", "/v1/payments/pro/plans", "payments", "List pro plans", "pro_plans"),
        p("POST", "/v1/payments/pro/subscribe", "payments", "Subscribe to pro", "pro_subscribe"),
        p("POST", "/v1/payments/pro/cancel", "payments", "Cancel pro subscription", "pro_cancel"),
        p("POST", "/v1/payments/creator/tiers", "payments", "Create creator tier", "creator_tier"),
        p("PUT", "/v1/payments/creator/tiers/{id}", "payments", "Update creator tier", "creator_tier_upd"),
        p("DELETE", "/v1/payments/creator/tiers/{id}", "payments", "Delete creator tier", "creator_tier_del"),
        p("GET", "/v1/payments/creator/{user_id}/tiers", "payments", "List creator tiers", "creator_tiers"),
        p("POST", "/v1/payments/creator/subscribe/{user_id}", "payments", "Subscribe to creator", "creator_sub"),
        p("DELETE", "/v1/payments/creator/subscribe/{user_id}", "payments", "Unsubscribe from creator", "creator_unsub"),
        p("GET", "/v1/payments/creator/subscribers", "payments", "List creator subscribers", "creator_subs"),
        p("GET", "/v1/payments/creator/subscriptions", "payments", "My creator subscriptions", "creator_my_subs"),
        p("POST", "/v1/payments/webhooks/{provider}", "payments", "Payment provider webhook", "pay_webhook"),
    ]);
}

// ── Admin Service (123 endpoints) ────────────────────────────────────────────

fn add_admin_paths(doc: &mut utoipa::openapi::OpenApi) {
    insert_all(doc, vec![
        // Dashboard
        p("GET", "/v1/admin/dashboard", "admin", "Dashboard stats", "admin_stats"),
        p("GET", "/v1/admin/dashboard/charts", "admin", "Dashboard charts", "admin_charts"),
        p("GET", "/v1/admin/dashboard/top-countries", "admin", "Top countries", "admin_countries"),
        p("GET", "/v1/admin/system-info", "admin", "System info", "admin_sysinfo"),
        // Users
        p("GET", "/v1/admin/users", "admin", "List users", "admin_users"),
        p("GET", "/v1/admin/users/{id}", "admin", "Get user", "admin_user"),
        p("PUT", "/v1/admin/users/{id}", "admin", "Update user", "admin_user_upd"),
        p("POST", "/v1/admin/users/{id}/ban", "admin", "Ban user", "admin_ban"),
        p("POST", "/v1/admin/users/{id}/unban", "admin", "Unban user", "admin_unban"),
        p("POST", "/v1/admin/users/{id}/verify", "admin", "Verify user", "admin_verify_user"),
        p("DELETE", "/v1/admin/users/{id}", "admin", "Delete user", "admin_del_user"),
        // Reports
        p("GET", "/v1/admin/reports", "admin", "List reports", "admin_reports"),
        p("GET", "/v1/admin/reports/{id}", "admin", "Get report", "admin_report"),
        p("POST", "/v1/admin/reports/{id}/resolve", "admin", "Resolve report", "admin_resolve"),
        p("POST", "/v1/admin/reports/{id}/dismiss", "admin", "Dismiss report", "admin_dismiss"),
        // Config
        p("GET", "/v1/admin/config", "admin", "List config", "admin_config"),
        p("GET", "/v1/admin/config/{category}", "admin", "Get config category", "admin_config_cat"),
        p("PUT", "/v1/admin/config", "admin", "Update config", "admin_config_upd"),
        // Categories
        p("GET", "/v1/admin/categories", "admin", "List categories", "admin_cats"),
        p("POST", "/v1/admin/categories", "admin", "Create category", "admin_cat_create"),
        p("PUT", "/v1/admin/categories/{id}", "admin", "Update category", "admin_cat_upd"),
        p("DELETE", "/v1/admin/categories/{id}", "admin", "Delete category", "admin_cat_del"),
        // Languages
        p("GET", "/v1/admin/languages", "admin", "List languages", "admin_langs"),
        p("POST", "/v1/admin/languages", "admin", "Create language", "admin_lang_create"),
        p("PUT", "/v1/admin/languages/{id}", "admin", "Update language", "admin_lang_upd"),
        p("DELETE", "/v1/admin/languages/{id}", "admin", "Delete language", "admin_lang_del"),
        // Announcements
        p("GET", "/v1/admin/announcements", "admin", "List announcements", "admin_ann"),
        p("POST", "/v1/admin/announcements", "admin", "Create announcement", "admin_ann_create"),
        p("PUT", "/v1/admin/announcements/{id}", "admin", "Update announcement", "admin_ann_upd"),
        p("DELETE", "/v1/admin/announcements/{id}", "admin", "Delete announcement", "admin_ann_del"),
        // Moderation
        p("GET", "/v1/admin/moderation/posts", "admin", "Pending posts", "admin_mod_posts"),
        p("POST", "/v1/admin/moderation/posts/{id}/approve", "admin", "Approve post", "admin_mod_approve"),
        p("POST", "/v1/admin/moderation/posts/{id}/reject", "admin", "Reject post", "admin_mod_reject"),
        p("GET", "/v1/admin/moderation/blogs", "admin", "Pending blogs", "admin_mod_blogs"),
        p("POST", "/v1/admin/moderation/blogs/{id}/approve", "admin", "Approve blog", "admin_mod_blog_approve"),
        p("POST", "/v1/admin/moderation/blogs/{id}/reject", "admin", "Reject blog", "admin_mod_blog_reject"),
        p("DELETE", "/v1/admin/posts/{id}", "admin", "Hard delete post", "admin_del_post"),
        // Verifications
        p("GET", "/v1/admin/verifications", "admin", "List verification requests", "admin_verif"),
        p("POST", "/v1/admin/verifications/{id}/approve", "admin", "Approve verification", "admin_verif_approve"),
        p("POST", "/v1/admin/verifications/{id}/reject", "admin", "Reject verification", "admin_verif_reject"),
        // Payments Admin
        p("GET", "/v1/admin/payments/stats", "admin", "Payment stats", "admin_pay_stats"),
        p("GET", "/v1/admin/payments/transactions", "admin", "List transactions", "admin_pay_tx"),
        p("GET", "/v1/admin/payments/withdrawals", "admin", "Pending withdrawals", "admin_pay_wd"),
        p("POST", "/v1/admin/payments/withdrawals/{id}/approve", "admin", "Approve withdrawal", "admin_pay_wd_approve"),
        p("POST", "/v1/admin/payments/withdrawals/{id}/reject", "admin", "Reject withdrawal", "admin_pay_wd_reject"),
        p("GET", "/v1/admin/payments/pro-plans", "admin", "List pro plans", "admin_pay_plans"),
        p("POST", "/v1/admin/payments/pro-plans", "admin", "Upsert pro plan", "admin_pay_plan_upsert"),
        // Banned IPs
        p("GET", "/v1/admin/banned-ips", "admin", "List banned IPs", "admin_bans"),
        p("POST", "/v1/admin/banned-ips", "admin", "Ban IP", "admin_ban_ip"),
        p("DELETE", "/v1/admin/banned-ips/{id}", "admin", "Unban IP", "admin_unban_ip"),
        // Custom Pages
        p("GET", "/v1/admin/pages", "admin", "List custom pages", "admin_pages"),
        p("POST", "/v1/admin/pages", "admin", "Create custom page", "admin_page_create"),
        p("PUT", "/v1/admin/pages/{id}", "admin", "Update custom page", "admin_page_upd"),
        p("DELETE", "/v1/admin/pages/{id}", "admin", "Delete custom page", "admin_page_del"),
        p("GET", "/v1/admin/pages/slug/{slug}", "admin", "Get page by slug", "admin_page_slug"),
        // Translations
        p("GET", "/v1/admin/translations", "admin", "List translations", "admin_trans"),
        p("POST", "/v1/admin/translations", "admin", "Upsert translation", "admin_trans_upsert"),
        p("POST", "/v1/admin/translations/bulk", "admin", "Bulk upsert translations", "admin_trans_bulk"),
        p("DELETE", "/v1/admin/translations/{id}", "admin", "Delete translation", "admin_trans_del"),
        // Newsletter
        p("GET", "/v1/admin/newsletter/subscribers", "admin", "Newsletter subscribers", "admin_nl_subs"),
        p("DELETE", "/v1/admin/newsletter/subscribers/{id}", "admin", "Remove subscriber", "admin_nl_rm"),
        p("POST", "/v1/admin/newsletter/send", "admin", "Send newsletter", "admin_nl_send"),
        // Profile Fields
        p("GET", "/v1/admin/profile-fields", "admin", "List profile fields", "admin_fields"),
        p("POST", "/v1/admin/profile-fields", "admin", "Create profile field", "admin_field_create"),
        p("PUT", "/v1/admin/profile-fields/{id}", "admin", "Update profile field", "admin_field_upd"),
        p("DELETE", "/v1/admin/profile-fields/{id}", "admin", "Delete profile field", "admin_field_del"),
        // User Roles
        p("POST", "/v1/admin/users/{user_id}/make-admin", "admin", "Make admin", "admin_make_admin"),
        p("POST", "/v1/admin/users/{user_id}/remove-admin", "admin", "Remove admin", "admin_rm_admin"),
        p("POST", "/v1/admin/users/{user_id}/make-pro", "admin", "Make pro", "admin_make_pro"),
        p("POST", "/v1/admin/users/{user_id}/remove-pro", "admin", "Remove pro", "admin_rm_pro"),
        // Email Templates
        p("GET", "/v1/admin/email-templates", "admin", "List email templates", "admin_email_tpl"),
        p("POST", "/v1/admin/email-templates", "admin", "Create email template", "admin_email_tpl_create"),
        p("PUT", "/v1/admin/email-templates/{id}", "admin", "Update email template", "admin_email_tpl_upd"),
        p("DELETE", "/v1/admin/email-templates/{id}", "admin", "Delete email template", "admin_email_tpl_del"),
        // Ads
        p("GET", "/v1/admin/ads", "admin", "List ads", "admin_ads"),
        p("PUT", "/v1/admin/ads/{id}", "admin", "Update ad", "admin_ad_upd"),
        // Content Admin
        p("GET", "/v1/admin/site-pages", "admin", "List site pages", "admin_site_pages"),
        p("DELETE", "/v1/admin/site-pages/{id}", "admin", "Delete site page", "admin_site_page_del"),
        p("GET", "/v1/admin/site-groups", "admin", "List site groups", "admin_site_groups"),
        p("DELETE", "/v1/admin/site-groups/{id}", "admin", "Delete site group", "admin_site_group_del"),
        p("GET", "/v1/admin/site-blogs", "admin", "List site blogs", "admin_site_blogs"),
        p("POST", "/v1/admin/site-blogs/{id}/approve", "admin", "Approve blog", "admin_site_blog_approve"),
        p("DELETE", "/v1/admin/site-blogs/{id}", "admin", "Delete site blog", "admin_site_blog_del"),
        p("GET", "/v1/admin/site-products", "admin", "List site products", "admin_site_products"),
        p("DELETE", "/v1/admin/site-products/{id}", "admin", "Delete site product", "admin_site_product_del"),
        p("GET", "/v1/admin/site-jobs", "admin", "List site jobs", "admin_site_jobs"),
        p("DELETE", "/v1/admin/site-jobs/{id}", "admin", "Delete site job", "admin_site_job_del"),
        p("GET", "/v1/admin/site-funding", "admin", "List site funding", "admin_site_funding"),
        p("DELETE", "/v1/admin/site-funding/{id}", "admin", "Delete site funding", "admin_site_funding_del"),
        p("GET", "/v1/admin/site-events", "admin", "List site events", "admin_site_events"),
        p("DELETE", "/v1/admin/site-events/{id}", "admin", "Delete site event", "admin_site_event_del"),
        p("GET", "/v1/admin/site-forums", "admin", "List site forums", "admin_site_forums"),
        p("PUT", "/v1/admin/site-forums/{id}", "admin", "Update site forum", "admin_site_forum_upd"),
        p("DELETE", "/v1/admin/site-forums/{id}", "admin", "Delete site forum", "admin_site_forum_del"),
        // Colored Posts
        p("GET", "/v1/admin/colored-posts", "admin", "List colored post templates", "admin_colored"),
        p("POST", "/v1/admin/colored-posts", "admin", "Create colored template", "admin_colored_create"),
        p("PUT", "/v1/admin/colored-posts/{id}", "admin", "Update colored template", "admin_colored_upd"),
        p("DELETE", "/v1/admin/colored-posts/{id}", "admin", "Delete colored template", "admin_colored_del"),
        // Reaction Types
        p("GET", "/v1/admin/reaction-types", "admin", "List reaction types", "admin_reactions"),
        p("POST", "/v1/admin/reaction-types", "admin", "Create reaction type", "admin_reaction_create"),
        p("PUT", "/v1/admin/reaction-types/{id}", "admin", "Update reaction type", "admin_reaction_upd"),
        p("DELETE", "/v1/admin/reaction-types/{id}", "admin", "Delete reaction type", "admin_reaction_del"),
        // Gifts
        p("GET", "/v1/admin/gifts", "admin", "List gifts", "admin_gifts"),
        p("POST", "/v1/admin/gifts", "admin", "Create gift", "admin_gift_create"),
        p("PUT", "/v1/admin/gifts/{id}", "admin", "Update gift", "admin_gift_upd"),
        p("DELETE", "/v1/admin/gifts/{id}", "admin", "Delete gift", "admin_gift_del"),
        // Stickers
        p("GET", "/v1/admin/sticker-packs", "admin", "List sticker packs", "admin_stickers"),
        p("POST", "/v1/admin/sticker-packs", "admin", "Create sticker pack", "admin_sticker_create"),
        p("PUT", "/v1/admin/sticker-packs/{id}", "admin", "Update sticker pack", "admin_sticker_upd"),
        p("DELETE", "/v1/admin/sticker-packs/{id}", "admin", "Delete sticker pack", "admin_sticker_del"),
        p("GET", "/v1/admin/sticker-packs/{pack_id}/stickers", "admin", "List stickers in pack", "admin_sticker_items"),
        p("POST", "/v1/admin/sticker-packs/{pack_id}/stickers", "admin", "Add sticker", "admin_sticker_add"),
        p("DELETE", "/v1/admin/stickers/{id}", "admin", "Delete sticker", "admin_sticker_item_del"),
        // Activity Log
        p("GET", "/v1/admin/activities", "admin", "Activity log", "admin_activities"),
        // Invitations
        p("GET", "/v1/admin/invitations", "admin", "List invitations", "admin_invitations"),
        p("POST", "/v1/admin/invitations", "admin", "Create invitation", "admin_invite"),
        p("DELETE", "/v1/admin/invitations/{id}", "admin", "Delete invitation", "admin_invite_del"),
        // OAuth Admin
        p("GET", "/v1/admin/oauth-apps", "admin", "List OAuth apps", "admin_oauth"),
        p("POST", "/v1/admin/oauth-apps/{id}/toggle", "admin", "Toggle OAuth app", "admin_oauth_toggle"),
        p("DELETE", "/v1/admin/oauth-apps/{id}", "admin", "Delete OAuth app", "admin_oauth_del"),
        // Backups
        p("GET", "/v1/admin/backups", "admin", "List backups", "admin_backups"),
        p("POST", "/v1/admin/backups/trigger", "admin", "Trigger backup", "admin_backup_trigger"),
    ]);
}

// ── AI Service (3 endpoints) ─────────────────────────────────────────────────

fn add_ai_paths(doc: &mut utoipa::openapi::OpenApi) {
    insert_all(doc, vec![
        p("POST", "/v1/ai/chat", "ai", "AI chat completion", "ai_chat"),
        p("POST", "/v1/ai/suggest-post", "ai", "AI post suggestion", "ai_suggest"),
        p("POST", "/v1/ai/describe-image", "ai", "AI image description", "ai_describe"),
    ]);
}

// ── Realtime Service (5 endpoints) ───────────────────────────────────────────

fn add_realtime_paths(doc: &mut utoipa::openapi::OpenApi) {
    insert_all(doc, vec![
        p("GET", "/ws", "realtime", "WebSocket connection", "ws"),
        p("GET", "/v1/presence/online", "realtime", "Online users", "presence_online"),
        p("GET", "/v1/presence/{user_id}", "realtime", "Check if user is online", "presence_check"),
        p("POST", "/internal/send/{user_id}", "realtime", "Internal: send to user via WS", "internal_send"),
        p("POST", "/internal/broadcast", "realtime", "Internal: broadcast to users via WS", "internal_broadcast"),
    ]);
}
