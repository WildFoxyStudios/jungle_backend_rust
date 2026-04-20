//! Static catalog of every configurable site_config key the admin UI
//! surfaces. Returned by `GET /v1/admin/config/catalog`.
//!
//! This is the single source of truth for the settings-form blitz
//! (`Admin A1` in the delivery plan): the frontend auto-generates every
//! tab in `/settings/*` from this list instead of hard-coding the fields
//! in each page. New config keys added to the product only need to be
//! appended here.
//!
//! The admin UI pairs the `category` field with
//! `GET /v1/admin/config/{category}` and `PUT /v1/admin/config` to read
//! and write values.

use axum::{extract::State, Json};
use serde::Serialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

/// Input widget the admin UI should render for a given key.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    /// Short free-form text.
    Text,
    /// Like `Text` but input type=password so the value is masked in the UI.
    Password,
    /// Like `Text` but input type=email.
    Email,
    /// Like `Text` but input type=url.
    Url,
    /// Integer or decimal. The admin form coerces to string for storage.
    Number,
    /// Boolean toggle.
    Boolean,
    /// Multi-line textarea.
    Textarea,
    /// Dropdown; `options` must be set.
    Select,
    /// Free-form JSON — rendered as a textarea with validation hint.
    Json,
    /// A media URL whose value comes from the upload-storage flow.
    MediaUrl,
}

/// Specification for a single configurable key.
#[derive(Debug, Clone, Serialize)]
pub struct FieldSpec {
    pub category: &'static str,
    pub key: &'static str,
    pub label: &'static str,
    pub r#type: FieldType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'static str>,
    /// Sub-group within a category (shown as a section header in the UI).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<&'static str>,
    /// Default value (stringified for storage parity with site_config.value).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<&'static str>,
    /// `true` means the value contains a secret. The GET endpoints will
    /// still return the real value for admins, but the UI should mask it.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub secret: bool,
    /// For `Select` — the allowed values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<&'static [&'static str]>,
    /// Optional placeholder shown in the input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<&'static str>,
}

impl FieldSpec {
    const fn new(
        category: &'static str,
        key: &'static str,
        label: &'static str,
        r#type: FieldType,
    ) -> Self {
        Self {
            category,
            key,
            label,
            r#type,
            description: None,
            group: None,
            default: None,
            secret: false,
            options: None,
            placeholder: None,
        }
    }

    const fn group(mut self, g: &'static str) -> Self { self.group = Some(g); self }
    const fn default(mut self, d: &'static str) -> Self { self.default = Some(d); self }
    const fn describe(mut self, d: &'static str) -> Self { self.description = Some(d); self }
    const fn secret(mut self) -> Self { self.secret = true; self }
    const fn options(mut self, o: &'static [&'static str]) -> Self { self.options = Some(o); self }
    const fn placeholder(mut self, p: &'static str) -> Self { self.placeholder = Some(p); self }
}

/// Shortcut constructor used by the catalog list below.
const fn f(
    category: &'static str,
    key: &'static str,
    label: &'static str,
    r#type: FieldType,
) -> FieldSpec {
    FieldSpec::new(category, key, label, r#type)
}

// ─── Catalog ────────────────────────────────────────────────────────────────

#[rustfmt::skip]
fn catalog() -> Vec<FieldSpec> {
    vec![
        // ── general ────────────────────────────────────────────────────────
        f("general", "site_name", "Site name", FieldType::Text).group("Branding").placeholder("WoWonder"),
        f("general", "site_tagline", "Tagline", FieldType::Text).group("Branding").placeholder("Your social network"),
        f("general", "site_email", "Admin / contact email", FieldType::Email).group("Branding"),
        f("general", "site_keywords", "Meta keywords", FieldType::Text).group("Branding").describe("Comma-separated SEO keywords."),
        f("general", "site_logo", "Site logo URL", FieldType::MediaUrl).group("Branding"),
        f("general", "site_favicon", "Favicon URL", FieldType::MediaUrl).group("Branding"),
        f("general", "timezone", "Default timezone", FieldType::Text).group("Locale").default("UTC"),
        f("general", "default_language", "Default language (ISO 639-1)", FieldType::Text).group("Locale").default("en"),
        f("general", "default_currency", "Default currency (ISO 4217)", FieldType::Text).group("Locale").default("USD"),
        f("general", "registration_mode", "Registration mode", FieldType::Select).group("Accounts").options(&["open", "invite_only", "approval_required", "closed"]).default("open"),
        f("general", "require_email_verification", "Require email verification", FieldType::Boolean).group("Accounts").default("true"),
        f("general", "require_phone_verification", "Require phone verification", FieldType::Boolean).group("Accounts").default("false"),
        f("general", "min_username_length", "Minimum username length", FieldType::Number).group("Accounts").default("3"),
        f("general", "max_username_length", "Maximum username length", FieldType::Number).group("Accounts").default("32"),
        f("general", "min_password_length", "Minimum password length", FieldType::Number).group("Accounts").default("8"),
        f("general", "maintenance_mode", "Maintenance mode", FieldType::Boolean).group("Availability").describe("Blocks non-admins from the site."),
        f("general", "maintenance_message", "Maintenance message", FieldType::Textarea).group("Availability"),
        f("general", "cors_origins", "CORS origins (comma-separated)", FieldType::Text).group("Security"),
        f("general", "allowed_file_types", "Allowed upload MIME types", FieldType::Textarea).group("Security").default("image/jpeg,image/png,image/gif,image/webp,video/mp4,audio/mpeg"),

        // ── features (platform-wide toggles) ───────────────────────────────
        f("features", "groups", "Groups", FieldType::Boolean).default("true"),
        f("features", "pages", "Pages", FieldType::Boolean).default("true"),
        f("features", "events", "Events", FieldType::Boolean).default("true"),
        f("features", "blogs", "Blogs", FieldType::Boolean).default("true"),
        f("features", "forums", "Forums", FieldType::Boolean).default("true"),
        f("features", "marketplace", "Marketplace", FieldType::Boolean).default("true"),
        f("features", "jobs", "Jobs", FieldType::Boolean).default("true"),
        f("features", "funding", "Funding", FieldType::Boolean).default("true"),
        f("features", "movies", "Movies", FieldType::Boolean).default("true"),
        f("features", "games", "Games", FieldType::Boolean).default("true"),
        f("features", "stories", "Stories", FieldType::Boolean).default("true"),
        f("features", "reels", "Reels", FieldType::Boolean).default("true"),
        f("features", "live", "Live streams", FieldType::Boolean).default("true"),
        f("features", "pokes", "Pokes", FieldType::Boolean).default("true"),
        f("features", "gifts", "Gifts", FieldType::Boolean).default("true"),
        f("features", "stickers", "Stickers", FieldType::Boolean).default("true"),
        f("features", "polls", "Polls", FieldType::Boolean).default("true"),
        f("features", "colored_posts", "Colored posts", FieldType::Boolean).default("true"),
        f("features", "offers", "Offers", FieldType::Boolean).default("true"),
        f("features", "points", "Points / gamification", FieldType::Boolean).default("true"),
        f("features", "affiliates", "Affiliates", FieldType::Boolean).default("true"),
        f("features", "ads", "User ads", FieldType::Boolean).default("true"),
        f("features", "monetization", "Creator monetization", FieldType::Boolean).default("true"),
        f("features", "ai", "AI features (writer, images)", FieldType::Boolean).default("true"),
        f("features", "calls_audio", "Audio calls", FieldType::Boolean).default("true"),
        f("features", "calls_video", "Video calls", FieldType::Boolean).default("true"),
        f("features", "broadcast_messages", "Broadcast messages (Pro)", FieldType::Boolean).default("true"),
        f("features", "disappearing_messages", "Disappearing messages", FieldType::Boolean).default("true"),
        f("features", "voice_messages", "Voice messages", FieldType::Boolean).default("true"),
        f("features", "two_factor_auth", "Two-factor authentication", FieldType::Boolean).default("true"),
        f("features", "privacy_controls", "Granular privacy controls", FieldType::Boolean).default("true"),

        // ── posts ──────────────────────────────────────────────────────────
        f("posts", "max_post_length", "Max post length (chars)", FieldType::Number).default("5000"),
        f("posts", "max_hashtags_per_post", "Max hashtags per post", FieldType::Number).default("30"),
        f("posts", "max_mentions_per_post", "Max mentions per post", FieldType::Number).default("20"),
        f("posts", "max_media_per_post", "Max media items per post", FieldType::Number).default("10"),
        f("posts", "auto_approve_posts", "Auto-approve posts", FieldType::Boolean).default("true").describe("If off, posts require admin approval before showing in feeds."),
        f("posts", "word_blacklist", "Word blacklist (one per line)", FieldType::Textarea).describe("Posts containing any of these words are auto-rejected."),
        f("posts", "allow_scheduled", "Allow scheduled posts", FieldType::Boolean).default("true"),
        f("posts", "allow_anonymous_page_posts", "Anonymous page posts", FieldType::Boolean).default("false"),
        f("posts", "default_privacy", "Default post privacy", FieldType::Select).options(&["public", "friends", "only_me"]).default("public"),
        f("posts", "enable_reactions", "Enable reactions", FieldType::Boolean).default("true"),
        f("posts", "enable_wonder_button", "Enable 'Wonder' button", FieldType::Boolean).default("true"),
        f("posts", "enable_comments", "Enable comments", FieldType::Boolean).default("true"),
        f("posts", "enable_comment_reactions", "Reactions on comments", FieldType::Boolean).default("true"),
        f("posts", "enable_shares", "Enable shares", FieldType::Boolean).default("true"),

        // ── video (client-side per architecture) ───────────────────────────
        f("video", "max_size_mb", "Max video size (MB)", FieldType::Number).default("100"),
        f("video", "max_duration_seconds", "Max duration (seconds)", FieldType::Number).default("300"),
        f("video", "allowed_formats", "Allowed formats", FieldType::Text).default("mp4,webm,mov"),
        f("video", "require_watermark", "Add watermark to uploads", FieldType::Boolean).default("false"),
        f("video", "watermark_url", "Watermark image URL", FieldType::MediaUrl),
        f("video", "autoplay_in_feed", "Autoplay in feed", FieldType::Boolean).default("true"),
        f("video", "autoplay_mute", "Autoplay muted", FieldType::Boolean).default("true"),

        // ── website_mode ───────────────────────────────────────────────────
        f("website_mode", "mode", "Primary site mode", FieldType::Select).options(&["social", "linkedin", "marketplace", "forum", "dating", "instagram"]).default("social").describe("Affects default landing page, navigation, and onboarding."),
        f("website_mode", "default_route_social", "Social mode landing route", FieldType::Text).default("/feed"),
        f("website_mode", "default_route_linkedin", "LinkedIn mode landing route", FieldType::Text).default("/jobs"),
        f("website_mode", "default_route_marketplace", "Marketplace mode landing route", FieldType::Text).default("/marketplace"),
        f("website_mode", "default_route_forum", "Forum mode landing route", FieldType::Text).default("/forums"),
        f("website_mode", "default_route_dating", "Dating mode landing route", FieldType::Text).default("/dating"),
        f("website_mode", "default_route_instagram", "Instagram mode landing route", FieldType::Text).default("/explore"),

        // ── email ──────────────────────────────────────────────────────────
        f("email", "transport", "Transport", FieldType::Select).group("Provider").options(&["smtp", "sendgrid", "mailgun", "ses", "postmark"]).default("smtp"),
        f("email", "from_address", "From address", FieldType::Email).group("Provider"),
        f("email", "from_name", "From name", FieldType::Text).group("Provider"),
        f("email", "smtp_host", "SMTP host", FieldType::Text).group("SMTP"),
        f("email", "smtp_port", "SMTP port", FieldType::Number).group("SMTP").default("587"),
        f("email", "smtp_username", "SMTP username", FieldType::Text).group("SMTP"),
        f("email", "smtp_password", "SMTP password", FieldType::Password).group("SMTP").secret(),
        f("email", "smtp_encryption", "SMTP encryption", FieldType::Select).group("SMTP").options(&["none", "tls", "ssl"]).default("tls"),
        f("email", "sendgrid_api_key", "SendGrid API key", FieldType::Password).group("SendGrid").secret(),
        f("email", "mailgun_api_key", "Mailgun API key", FieldType::Password).group("Mailgun").secret(),
        f("email", "mailgun_domain", "Mailgun domain", FieldType::Text).group("Mailgun"),
        f("email", "ses_region", "SES region", FieldType::Text).group("AWS SES").default("us-east-1"),
        f("email", "ses_access_key", "SES access key", FieldType::Text).group("AWS SES"),
        f("email", "ses_secret_key", "SES secret key", FieldType::Password).group("AWS SES").secret(),
        f("email", "postmark_server_token", "Postmark server token", FieldType::Password).group("Postmark").secret(),
        f("email", "retry_attempts", "Retry attempts on failure", FieldType::Number).group("Reliability").default("3"),

        // ── email security ─────────────────────────────────────────────────
        f("email_security", "blocked_domains", "Blocked email domains (one per line)", FieldType::Textarea),
        f("email_security", "block_disposable_domains", "Block disposable email providers", FieldType::Boolean).default("true"),
        f("email_security", "require_spf_check", "Require SPF record on senders", FieldType::Boolean).default("false"),

        // ── sms ────────────────────────────────────────────────────────────
        f("sms", "provider", "SMS provider", FieldType::Select).options(&["twilio", "infobip", "msg91", "disabled"]).default("disabled"),
        f("sms", "twilio_account_sid", "Twilio account SID", FieldType::Text).group("Twilio"),
        f("sms", "twilio_auth_token", "Twilio auth token", FieldType::Password).group("Twilio").secret(),
        f("sms", "twilio_from_number", "Twilio 'from' number", FieldType::Text).group("Twilio").placeholder("+1…"),
        f("sms", "infobip_api_key", "Infobip API key", FieldType::Password).group("Infobip").secret(),
        f("sms", "infobip_base_url", "Infobip base URL", FieldType::Url).group("Infobip"),
        f("sms", "msg91_auth_key", "MSG91 auth key", FieldType::Password).group("MSG91").secret(),
        f("sms", "msg91_sender_id", "MSG91 sender ID", FieldType::Text).group("MSG91"),

        // ── push (FCM + APNs + Web Push) ───────────────────────────────────
        f("push", "fcm_server_key", "FCM server key (legacy)", FieldType::Password).group("Firebase").secret(),
        f("push", "fcm_project_id", "FCM project ID", FieldType::Text).group("Firebase"),
        f("push", "fcm_service_account_json", "FCM service-account JSON", FieldType::Json).group("Firebase").secret(),
        f("push", "apns_key_id", "APNs key ID", FieldType::Text).group("Apple Push"),
        f("push", "apns_team_id", "APNs team ID", FieldType::Text).group("Apple Push"),
        f("push", "apns_bundle_id", "iOS bundle ID", FieldType::Text).group("Apple Push"),
        f("push", "apns_p8_key", "APNs .p8 key (PEM)", FieldType::Textarea).group("Apple Push").secret(),
        f("push", "apns_production", "APNs production mode", FieldType::Boolean).group("Apple Push").default("false"),
        f("push", "vapid_public_key", "VAPID public key", FieldType::Text).group("Web Push"),
        f("push", "vapid_private_key", "VAPID private key", FieldType::Password).group("Web Push").secret(),
        f("push", "vapid_subject", "VAPID subject (mailto:/url)", FieldType::Text).group("Web Push"),
        f("push", "default_icon", "Default notification icon URL", FieldType::MediaUrl),
        f("push", "default_sound", "Default notification sound", FieldType::Text).default("default"),

        // ── social_login ───────────────────────────────────────────────────
        f("social_login", "google_enabled", "Enable Google", FieldType::Boolean).group("Google"),
        f("social_login", "google_client_id", "Google client ID", FieldType::Text).group("Google"),
        f("social_login", "google_client_secret", "Google client secret", FieldType::Password).group("Google").secret(),
        f("social_login", "facebook_enabled", "Enable Facebook", FieldType::Boolean).group("Facebook"),
        f("social_login", "facebook_app_id", "Facebook app ID", FieldType::Text).group("Facebook"),
        f("social_login", "facebook_app_secret", "Facebook app secret", FieldType::Password).group("Facebook").secret(),
        f("social_login", "twitter_enabled", "Enable Twitter / X", FieldType::Boolean).group("Twitter / X"),
        f("social_login", "twitter_client_id", "Twitter client ID", FieldType::Text).group("Twitter / X"),
        f("social_login", "twitter_client_secret", "Twitter client secret", FieldType::Password).group("Twitter / X").secret(),
        f("social_login", "linkedin_enabled", "Enable LinkedIn", FieldType::Boolean).group("LinkedIn"),
        f("social_login", "linkedin_client_id", "LinkedIn client ID", FieldType::Text).group("LinkedIn"),
        f("social_login", "linkedin_client_secret", "LinkedIn client secret", FieldType::Password).group("LinkedIn").secret(),
        f("social_login", "apple_enabled", "Enable Apple", FieldType::Boolean).group("Apple"),
        f("social_login", "apple_client_id", "Apple client ID (service ID)", FieldType::Text).group("Apple"),
        f("social_login", "apple_team_id", "Apple team ID", FieldType::Text).group("Apple"),
        f("social_login", "apple_key_id", "Apple key ID", FieldType::Text).group("Apple"),
        f("social_login", "apple_p8_key", "Apple .p8 key (PEM)", FieldType::Textarea).group("Apple").secret(),
        f("social_login", "github_enabled", "Enable GitHub", FieldType::Boolean).group("GitHub"),
        f("social_login", "github_client_id", "GitHub client ID", FieldType::Text).group("GitHub"),
        f("social_login", "github_client_secret", "GitHub client secret", FieldType::Password).group("GitHub").secret(),
        f("social_login", "discord_enabled", "Enable Discord", FieldType::Boolean).group("Discord"),
        f("social_login", "discord_client_id", "Discord client ID", FieldType::Text).group("Discord"),
        f("social_login", "discord_client_secret", "Discord client secret", FieldType::Password).group("Discord").secret(),
        f("social_login", "microsoft_enabled", "Enable Microsoft", FieldType::Boolean).group("Microsoft"),
        f("social_login", "microsoft_client_id", "Microsoft client ID", FieldType::Text).group("Microsoft"),
        f("social_login", "microsoft_client_secret", "Microsoft client secret", FieldType::Password).group("Microsoft").secret(),
        f("social_login", "vk_enabled", "Enable VKontakte", FieldType::Boolean).group("VKontakte"),
        f("social_login", "vk_client_id", "VK client ID", FieldType::Text).group("VKontakte"),
        f("social_login", "vk_client_secret", "VK client secret", FieldType::Password).group("VKontakte").secret(),
        f("social_login", "mailru_enabled", "Enable Mail.ru", FieldType::Boolean).group("Mail.ru"),
        f("social_login", "mailru_client_id", "Mail.ru client ID", FieldType::Text).group("Mail.ru"),
        f("social_login", "mailru_client_secret", "Mail.ru client secret", FieldType::Password).group("Mail.ru").secret(),
        f("social_login", "yandex_enabled", "Enable Yandex", FieldType::Boolean).group("Yandex"),
        f("social_login", "yandex_client_id", "Yandex client ID", FieldType::Text).group("Yandex"),
        f("social_login", "yandex_client_secret", "Yandex client secret", FieldType::Password).group("Yandex").secret(),
        f("social_login", "wordpress_enabled", "Enable WordPress.com", FieldType::Boolean).group("WordPress.com"),
        f("social_login", "wordpress_client_id", "WordPress client ID", FieldType::Text).group("WordPress.com"),
        f("social_login", "wordpress_client_secret", "WordPress client secret", FieldType::Password).group("WordPress.com").secret(),
        f("social_login", "dropbox_enabled", "Enable Dropbox", FieldType::Boolean).group("Dropbox"),
        f("social_login", "dropbox_client_id", "Dropbox client ID", FieldType::Text).group("Dropbox"),
        f("social_login", "dropbox_client_secret", "Dropbox client secret", FieldType::Password).group("Dropbox").secret(),
        f("social_login", "instagram_enabled", "Enable Instagram", FieldType::Boolean).group("Instagram"),
        f("social_login", "instagram_client_id", "Instagram client ID", FieldType::Text).group("Instagram"),
        f("social_login", "instagram_client_secret", "Instagram client secret", FieldType::Password).group("Instagram").secret(),

        // ── ads ────────────────────────────────────────────────────────────
        f("ads", "user_ads_enabled", "Enable user ads", FieldType::Boolean).default("true"),
        f("ads", "review_mode", "Require admin review before serving", FieldType::Boolean).default("true"),
        f("ads", "min_budget", "Minimum ad budget", FieldType::Number).default("5"),
        f("ads", "default_cpc", "Default cost-per-click", FieldType::Number).default("0.10"),
        f("ads", "default_cpm", "Default cost-per-mille", FieldType::Number).default("2.00"),
        f("ads", "allowed_formats", "Allowed ad formats", FieldType::Text).default("image,video,carousel"),
        f("ads", "blocked_keywords", "Blocked keywords (one per line)", FieldType::Textarea),
        f("ads", "pro_free_ads_per_month", "Free ads for Pro members / month", FieldType::Number).default("0"),

        // ── pro ────────────────────────────────────────────────────────────
        f("pro", "enabled", "Enable Pro subscriptions", FieldType::Boolean).default("true"),
        f("pro", "trial_days", "Free trial days", FieldType::Number).default("0"),
        f("pro", "grace_period_days", "Grace period on failed renewal", FieldType::Number).default("3"),
        f("pro", "auto_renew_enabled", "Auto-renew from wallet balance", FieldType::Boolean).default("true"),
        f("pro", "default_storage_gb", "Default storage quota (GB)", FieldType::Number).default("5"),
        f("pro", "pro_storage_gb", "Pro storage quota (GB)", FieldType::Number).default("50"),

        // ── affiliates ─────────────────────────────────────────────────────
        f("affiliates", "enabled", "Enable affiliate program", FieldType::Boolean).default("false"),
        f("affiliates", "commission_percent", "Commission (%)", FieldType::Number).default("10"),
        f("affiliates", "min_payout", "Minimum payout", FieldType::Number).default("50"),
        f("affiliates", "payout_cycle_days", "Payout cycle (days)", FieldType::Number).default("30"),
        f("affiliates", "cookie_ttl_days", "Referral cookie TTL (days)", FieldType::Number).default("30"),

        // ── appearance / design ───────────────────────────────────────────
        f("appearance", "primary_color", "Primary color", FieldType::Text).default("#3b82f6").placeholder("#rrggbb"),
        f("appearance", "secondary_color", "Secondary color", FieldType::Text).default("#6b7280"),
        f("appearance", "dark_mode_default", "Dark mode by default", FieldType::Boolean).default("false"),
        f("appearance", "logo_url", "Logo URL", FieldType::MediaUrl),
        f("appearance", "logo_dark_url", "Logo URL (dark mode)", FieldType::MediaUrl),
        f("appearance", "favicon_url", "Favicon URL", FieldType::MediaUrl),
        f("appearance", "hero_background_url", "Landing hero background URL", FieldType::MediaUrl),
        f("appearance", "custom_css", "Custom CSS", FieldType::Textarea),

        // ── seo ────────────────────────────────────────────────────────────
        f("seo", "meta_title_suffix", "Meta title suffix", FieldType::Text).placeholder(" | MyCommunity"),
        f("seo", "meta_description", "Default meta description", FieldType::Textarea),
        f("seo", "og_image_url", "Default og:image URL", FieldType::MediaUrl),
        f("seo", "twitter_card", "Twitter card type", FieldType::Select).options(&["summary", "summary_large_image"]).default("summary_large_image"),
        f("seo", "google_site_verification", "Google site verification token", FieldType::Text),
        f("seo", "bing_site_verification", "Bing site verification token", FieldType::Text),
        f("seo", "robots_txt_extra", "robots.txt additional rules", FieldType::Textarea),
        f("seo", "sitemap_enabled", "Expose /sitemap.xml", FieldType::Boolean).default("true"),

        // ── live ───────────────────────────────────────────────────────────
        f("live", "provider", "Live streaming provider", FieldType::Select).options(&["rtmp", "agora", "millicast"]).default("rtmp"),
        f("live", "agora_app_id", "Agora App ID", FieldType::Text).group("Agora"),
        f("live", "agora_app_certificate", "Agora App Certificate", FieldType::Password).group("Agora").secret(),
        f("live", "millicast_publisher_token", "Millicast publisher token", FieldType::Password).group("Millicast").secret(),
        f("live", "millicast_account_id", "Millicast account ID", FieldType::Text).group("Millicast"),
        f("live", "max_concurrent_per_user", "Max concurrent streams per user", FieldType::Number).default("1"),
        f("live", "max_concurrent_site", "Max concurrent streams (site)", FieldType::Number).default("100"),
        f("live", "max_viewers_per_stream", "Max viewers per stream", FieldType::Number).default("10000"),
        f("live", "recording_enabled", "Enable recording", FieldType::Boolean).default("false"),
        f("live", "recording_retention_days", "Recording retention (days)", FieldType::Number).default("30"),

        // ── store ──────────────────────────────────────────────────────────
        f("store", "enabled", "Enable marketplace", FieldType::Boolean).default("true"),
        f("store", "min_order_amount", "Minimum order amount", FieldType::Number).default("1"),
        f("store", "commission_percent", "Platform commission (%)", FieldType::Number).default("5"),
        f("store", "tax_rate_percent", "Default tax rate (%)", FieldType::Number).default("0"),
        f("store", "shipping_zones_json", "Shipping zones (JSON)", FieldType::Json).describe("Array of { name, countries[], flat_rate }."),
        f("store", "auto_approve_products", "Auto-approve new products", FieldType::Boolean).default("true"),
        f("store", "featured_product_ids", "Featured product IDs", FieldType::Text).placeholder("1,2,3"),

        // ── custom_code ────────────────────────────────────────────────────
        f("custom_code", "head_html", "Custom HTML in <head>", FieldType::Textarea).describe("Analytics snippets, verification tags, preloaded fonts, etc."),
        f("custom_code", "body_html", "Custom HTML before </body>", FieldType::Textarea),
        f("custom_code", "custom_js", "Custom JavaScript", FieldType::Textarea),

        // ── third_party (pixels & analytics) ───────────────────────────────
        f("third_party", "google_analytics_id", "Google Analytics / GA4 ID", FieldType::Text),
        f("third_party", "gtm_container_id", "Google Tag Manager ID", FieldType::Text).placeholder("GTM-…"),
        f("third_party", "facebook_pixel_id", "Facebook Pixel ID", FieldType::Text),
        f("third_party", "hotjar_id", "Hotjar site ID", FieldType::Text),
        f("third_party", "tawk_to_widget_id", "Tawk.to widget ID", FieldType::Text),
        f("third_party", "sentry_dsn", "Sentry DSN (frontend)", FieldType::Text),
        f("third_party", "recaptcha_enabled", "Enable reCAPTCHA on auth", FieldType::Boolean).default("false"),
        f("third_party", "recaptcha_site_key", "reCAPTCHA site key", FieldType::Text),
        f("third_party", "recaptcha_secret_key", "reCAPTCHA secret key", FieldType::Password).secret(),
        f("third_party", "giphy_api_key", "GIPHY API key", FieldType::Password).secret(),
        f("third_party", "tenor_api_key", "Tenor GIF API key", FieldType::Password).secret(),
        f("third_party", "mapbox_token", "Mapbox access token", FieldType::Password).secret(),
        f("third_party", "google_maps_api_key", "Google Maps API key", FieldType::Password).secret(),
    ]
}

/// GET /v1/admin/config/catalog
pub async fn get_catalog(
    State(_state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    let fields = catalog();

    // Group by category for a nicer shape on the wire.
    let mut categories: Vec<String> = fields
        .iter()
        .map(|f| f.category.to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    categories.sort();

    let mut by_category: serde_json::Map<String, Value> = serde_json::Map::new();
    for cat in &categories {
        let items: Vec<&FieldSpec> = fields.iter().filter(|f| f.category == cat.as_str()).collect();
        by_category.insert(cat.clone(), serde_json::to_value(items).unwrap_or(Value::Null));
    }

    Ok(Json(json!({
        "data": {
            "categories": categories,
            "fields": fields,
            "by_category": by_category,
            "total": fields.len(),
        }
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_no_duplicate_keys_per_category() {
        let fields = catalog();
        let mut seen = std::collections::HashSet::new();
        for f in &fields {
            let k = format!("{}:{}", f.category, f.key);
            assert!(seen.insert(k.clone()), "duplicate key: {}", k);
        }
    }

    #[test]
    fn select_fields_have_options() {
        for f in &catalog() {
            if matches!(f.r#type, FieldType::Select) {
                assert!(
                    f.options.is_some() && !f.options.unwrap().is_empty(),
                    "Select field {}:{} must have options",
                    f.category, f.key,
                );
            }
        }
    }

    #[test]
    fn catalog_covers_expected_categories() {
        let fields = catalog();
        let categories: std::collections::HashSet<&str> =
            fields.iter().map(|f| f.category).collect();

        // The frontend settings tabs depend on these categories existing.
        for required in &[
            "general", "features", "posts", "video", "website_mode", "email",
            "email_security", "sms", "push", "social_login", "ads", "pro",
            "affiliates", "appearance", "seo", "live", "store", "custom_code",
            "third_party",
        ] {
            assert!(
                categories.contains(*required),
                "missing required category: {}",
                required,
            );
        }
    }

    #[test]
    fn catalog_is_non_trivial_size() {
        // Guards against an accidental wipe; we expect at least this many
        // keys after the initial blitz.
        assert!(catalog().len() >= 150, "catalog shrunk: only {} keys", catalog().len());
    }
}
