use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    pub general: GeneralConfig,
    pub auth: AuthConfig,
    pub social: SocialConfig,
    pub features: FeaturesConfig,
    pub media: MediaConfig,
    pub payment: PaymentConfig,
    pub email: EmailConfig,
    pub sms: SmsConfig,
    pub appearance: AppearanceConfig,
    pub limits: LimitsConfig,
    pub seo: SeoConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub site_name: String,
    pub site_title: String,
    pub site_url: String,
    pub site_desc: String,
    pub site_email: String,
    pub default_language: String,
    pub theme: String,
    pub maintenance_mode: bool,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub user_registration: bool,
    pub email_verification: bool,
    pub sms_verification: bool,
    pub invitation_only: bool,
    pub confirm_followers: bool,
    pub two_factor_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    pub groups: bool,
    pub pages: bool,
    pub events: bool,
    pub blogs: bool,
    pub forums: bool,
    pub marketplace: bool,
    pub jobs: bool,
    pub funding: bool,
    pub movies: bool,
    pub games: bool,
    pub stories: bool,
    pub reels: bool,
    pub live_video: bool,
    pub file_sharing: bool,
    pub pokes: bool,
    pub gifts: bool,
    pub colored_posts: bool,
    pub points_system: bool,
    pub affiliate_system: bool,
    pub ads_system: bool,
    pub offers: bool,
    pub monetization: bool,
    pub post_approval: bool,
    pub blog_approval: bool,
    pub ai_system: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialConfig {
    pub google_login: bool,
    pub facebook_login: bool,
    pub twitter_login: bool,
    pub linkedin_login: bool,
    pub apple_login: bool,
    pub google_client_id: String,
    pub facebook_app_id: String,
    pub twitter_consumer_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentConfig {
    pub currency: String,
    pub currency_symbol: String,
    pub stripe_enabled: bool,
    pub paypal_enabled: bool,
    pub stripe_publishable_key: String,
    pub paypal_client_id: String,
    pub pro_packages_enabled: bool,
    pub wallet_enabled: bool,
    pub withdrawal_min: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_encryption: String,
    pub from_email: String,
    pub from_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsConfig {
    pub provider: String,
    pub twilio_sid: String,
    pub twilio_token: String,
    pub twilio_phone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    pub header_background: String,
    pub header_text_color: String,
    pub logo_url: String,
    pub favicon_url: String,
    pub login_bg_image: String,
    pub registration_bg_image: String,
    pub custom_css: String,
    pub custom_js: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoConfig {
    pub site_keywords: String,
    pub google_analytics_id: String,
    pub facebook_pixel_id: String,
    pub robots_txt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaConfig {
    pub max_upload_size: i64,
    pub allowed_extensions: String,
    pub images_quality: i32,
    pub watermark: bool,
    pub storage_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    pub max_characters: i64,
    pub post_limit: i64,
    pub max_multi_images: i32,
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                site_name: "WoWonder".into(),
                site_title: "WoWonder Social Network".into(),
                site_url: "http://localhost".into(),
                site_desc: "The Ultimate PHP Social Network Platform".into(),
                site_email: "admin@example.com".into(),
                default_language: "english".into(),
                theme: "developer".into(),
                maintenance_mode: false,
                timezone: "UTC".into(),
            },
            auth: AuthConfig {
                user_registration: true,
                email_verification: false,
                sms_verification: false,
                invitation_only: false,
                confirm_followers: false,
                two_factor_enabled: true,
            },
            features: FeaturesConfig {
                groups: true,
                pages: true,
                events: true,
                blogs: true,
                forums: true,
                marketplace: true,
                jobs: true,
                funding: true,
                movies: true,
                games: true,
                stories: true,
                reels: true,
                live_video: true,
                file_sharing: true,
                pokes: true,
                gifts: true,
                colored_posts: true,
                points_system: false,
                affiliate_system: false,
                ads_system: true,
                offers: true,
                monetization: true,
                post_approval: false,
                blog_approval: false,
                ai_system: false,
            },
            media: MediaConfig {
                max_upload_size: 10_485_760, // 10 MB
                allowed_extensions: "jpg,jpeg,png,gif,webp,mp4,mp3,pdf,doc,docx,zip".into(),
                images_quality: 80,
                watermark: false,
                storage_provider: "local".into(),
            },
            social: SocialConfig {
                google_login: false,
                facebook_login: false,
                twitter_login: false,
                linkedin_login: false,
                apple_login: false,
                google_client_id: String::new(),
                facebook_app_id: String::new(),
                twitter_consumer_key: String::new(),
            },
            payment: PaymentConfig {
                currency: "USD".into(),
                currency_symbol: "$".into(),
                stripe_enabled: false,
                paypal_enabled: false,
                stripe_publishable_key: String::new(),
                paypal_client_id: String::new(),
                pro_packages_enabled: true,
                wallet_enabled: true,
                withdrawal_min: 50.0,
            },
            email: EmailConfig {
                smtp_host: "localhost".into(),
                smtp_port: 587,
                smtp_username: String::new(),
                smtp_encryption: "tls".into(),
                from_email: "noreply@example.com".into(),
                from_name: "WoWonder".into(),
            },
            sms: SmsConfig {
                provider: "twilio".into(),
                twilio_sid: String::new(),
                twilio_token: String::new(),
                twilio_phone: String::new(),
            },
            appearance: AppearanceConfig {
                header_background: "#2b5876".into(),
                header_text_color: "#ffffff".into(),
                logo_url: String::new(),
                favicon_url: String::new(),
                login_bg_image: String::new(),
                registration_bg_image: String::new(),
                custom_css: String::new(),
                custom_js: String::new(),
            },
            limits: LimitsConfig {
                max_characters: 63206,
                post_limit: 0, // 0 = unlimited
                max_multi_images: 10,
            },
            seo: SeoConfig {
                site_keywords: String::new(),
                google_analytics_id: String::new(),
                facebook_pixel_id: String::new(),
                robots_txt: "User-agent: *\nAllow: /".into(),
            },
        }
    }
}

fn get_str(map: &HashMap<String, HashMap<String, String>>, cat: &str, key: &str, default: &str) -> String {
    map.get(cat)
        .and_then(|m| m.get(key))
        .cloned()
        .unwrap_or_else(|| default.to_string())
}

fn get_bool(map: &HashMap<String, HashMap<String, String>>, cat: &str, key: &str, default: bool) -> bool {
    map.get(cat)
        .and_then(|m| m.get(key))
        .map(|v| v == "1" || v == "true" || v == "yes")
        .unwrap_or(default)
}

fn get_i64(map: &HashMap<String, HashMap<String, String>>, cat: &str, key: &str, default: i64) -> i64 {
    map.get(cat)
        .and_then(|m| m.get(key))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn get_i32(map: &HashMap<String, HashMap<String, String>>, cat: &str, key: &str, default: i32) -> i32 {
    map.get(cat)
        .and_then(|m| m.get(key))
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_site_config_default_general() {
        let cfg = SiteConfig::default();
        assert_eq!(cfg.general.site_name, "WoWonder");
        assert!(!cfg.general.maintenance_mode);
        assert_eq!(cfg.general.timezone, "UTC");
    }

    #[test]
    fn test_site_config_default_features_all_enabled() {
        let cfg = SiteConfig::default();
        assert!(cfg.features.groups);
        assert!(cfg.features.pages);
        assert!(cfg.features.events);
        assert!(cfg.features.blogs);
        assert!(cfg.features.stories);
        assert!(cfg.features.reels);
        assert!(!cfg.features.post_approval);
        assert!(!cfg.features.ai_system);
    }

    #[test]
    fn test_site_config_default_limits() {
        let cfg = SiteConfig::default();
        assert_eq!(cfg.limits.max_characters, 63206);
        assert_eq!(cfg.limits.max_multi_images, 10);
        assert_eq!(cfg.limits.post_limit, 0);
    }

    #[test]
    fn test_get_str_with_value() {
        let mut map = HashMap::new();
        let mut inner = HashMap::new();
        inner.insert("key1".to_string(), "value1".to_string());
        map.insert("cat".to_string(), inner);
        assert_eq!(get_str(&map, "cat", "key1", "default"), "value1");
    }

    #[test]
    fn test_get_str_missing_returns_default() {
        let map: HashMap<String, HashMap<String, String>> = HashMap::new();
        assert_eq!(get_str(&map, "cat", "key1", "fallback"), "fallback");
    }

    #[test]
    fn test_get_bool_true_values() {
        let mut map = HashMap::new();
        let mut inner = HashMap::new();
        inner.insert("a".to_string(), "1".to_string());
        inner.insert("b".to_string(), "true".to_string());
        inner.insert("c".to_string(), "yes".to_string());
        inner.insert("d".to_string(), "no".to_string());
        map.insert("cat".to_string(), inner);
        assert!(get_bool(&map, "cat", "a", false));
        assert!(get_bool(&map, "cat", "b", false));
        assert!(get_bool(&map, "cat", "c", false));
        assert!(!get_bool(&map, "cat", "d", true));
    }

    #[test]
    fn test_get_i64_parses() {
        let mut map = HashMap::new();
        let mut inner = HashMap::new();
        inner.insert("count".to_string(), "42".to_string());
        map.insert("cat".to_string(), inner);
        assert_eq!(get_i64(&map, "cat", "count", 0), 42);
        assert_eq!(get_i64(&map, "cat", "missing", 99), 99);
    }
}

/// Load the full site configuration from the database
pub async fn load_config(db: &PgPool) -> Result<SiteConfig, sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT category, key, value FROM site_config",
    )
    .fetch_all(db)
    .await?;

    let mut map: HashMap<String, HashMap<String, String>> = HashMap::new();
    for (category, key, value) in rows {
        map.entry(category).or_default().insert(key, value);
    }

    let default = SiteConfig::default();

    Ok(SiteConfig {
        general: GeneralConfig {
            site_name: get_str(&map, "general", "site_name", &default.general.site_name),
            site_title: get_str(&map, "general", "site_title", &default.general.site_title),
            site_url: get_str(&map, "general", "site_url", &default.general.site_url),
            site_desc: get_str(&map, "general", "site_desc", &default.general.site_desc),
            site_email: get_str(&map, "general", "site_email", &default.general.site_email),
            default_language: get_str(&map, "general", "default_language", &default.general.default_language),
            theme: get_str(&map, "general", "theme", &default.general.theme),
            maintenance_mode: get_bool(&map, "general", "maintenance_mode", false),
            timezone: get_str(&map, "general", "timezone", &default.general.timezone),
        },
        auth: AuthConfig {
            user_registration: get_bool(&map, "auth", "user_registration", true),
            email_verification: get_bool(&map, "auth", "email_verification", false),
            sms_verification: get_bool(&map, "auth", "sms_verification", false),
            invitation_only: get_bool(&map, "auth", "invitation_only", false),
            confirm_followers: get_bool(&map, "auth", "confirm_followers", false),
            two_factor_enabled: get_bool(&map, "auth", "two_factor_enabled", true),
        },
        social: SocialConfig {
            google_login: get_bool(&map, "social", "google_login", false),
            facebook_login: get_bool(&map, "social", "facebook_login", false),
            twitter_login: get_bool(&map, "social", "twitter_login", false),
            linkedin_login: get_bool(&map, "social", "linkedin_login", false),
            apple_login: get_bool(&map, "social", "apple_login", false),
            google_client_id: get_str(&map, "social", "google_client_id", ""),
            facebook_app_id: get_str(&map, "social", "facebook_app_id", ""),
            twitter_consumer_key: get_str(&map, "social", "twitter_consumer_key", ""),
        },
        features: FeaturesConfig {
            groups: get_bool(&map, "features", "groups", true),
            pages: get_bool(&map, "features", "pages", true),
            events: get_bool(&map, "features", "events", true),
            blogs: get_bool(&map, "features", "blogs", true),
            forums: get_bool(&map, "features", "forums", true),
            marketplace: get_bool(&map, "features", "marketplace", true),
            jobs: get_bool(&map, "features", "jobs", true),
            funding: get_bool(&map, "features", "funding", true),
            movies: get_bool(&map, "features", "movies", true),
            games: get_bool(&map, "features", "games", true),
            stories: get_bool(&map, "features", "stories", true),
            reels: get_bool(&map, "features", "reels", true),
            live_video: get_bool(&map, "features", "live_video", true),
            file_sharing: get_bool(&map, "features", "file_sharing", true),
            pokes: get_bool(&map, "features", "pokes", true),
            gifts: get_bool(&map, "features", "gifts", true),
            colored_posts: get_bool(&map, "features", "colored_posts", true),
            points_system: get_bool(&map, "features", "points_system", false),
            affiliate_system: get_bool(&map, "features", "affiliate_system", false),
            ads_system: get_bool(&map, "features", "ads_system", true),
            offers: get_bool(&map, "features", "offers", true),
            monetization: get_bool(&map, "features", "monetization", true),
            post_approval: get_bool(&map, "features", "post_approval", false),
            blog_approval: get_bool(&map, "features", "blog_approval", false),
            ai_system: get_bool(&map, "features", "ai_system", false),
        },
        media: MediaConfig {
            max_upload_size: get_i64(&map, "media", "max_upload_size", 10_485_760),
            allowed_extensions: get_str(&map, "media", "allowed_extensions", "jpg,jpeg,png,gif,webp,mp4,mp3,pdf"),
            images_quality: get_i32(&map, "media", "images_quality", 80),
            watermark: get_bool(&map, "media", "watermark", false),
            storage_provider: get_str(&map, "media", "storage_provider", "local"),
        },
        payment: PaymentConfig {
            currency: get_str(&map, "payment", "currency", "USD"),
            currency_symbol: get_str(&map, "payment", "currency_symbol", "$"),
            stripe_enabled: get_bool(&map, "payment", "stripe_enabled", false),
            paypal_enabled: get_bool(&map, "payment", "paypal_enabled", false),
            stripe_publishable_key: get_str(&map, "payment", "stripe_publishable_key", ""),
            paypal_client_id: get_str(&map, "payment", "paypal_client_id", ""),
            pro_packages_enabled: get_bool(&map, "payment", "pro_packages_enabled", true),
            wallet_enabled: get_bool(&map, "payment", "wallet_enabled", true),
            withdrawal_min: get_str(&map, "payment", "withdrawal_min", "50").parse().unwrap_or(50.0),
        },
        email: EmailConfig {
            smtp_host: get_str(&map, "email", "smtp_host", "localhost"),
            smtp_port: get_str(&map, "email", "smtp_port", "587").parse().unwrap_or(587),
            smtp_username: get_str(&map, "email", "smtp_username", ""),
            smtp_encryption: get_str(&map, "email", "smtp_encryption", "tls"),
            from_email: get_str(&map, "email", "from_email", "noreply@example.com"),
            from_name: get_str(&map, "email", "from_name", "WoWonder"),
        },
        sms: SmsConfig {
            provider: get_str(&map, "sms", "provider", "twilio"),
            twilio_sid: get_str(&map, "sms", "twilio_sid", ""),
            twilio_token: get_str(&map, "sms", "twilio_token", ""),
            twilio_phone: get_str(&map, "sms", "twilio_phone", ""),
        },
        appearance: AppearanceConfig {
            header_background: get_str(&map, "appearance", "header_background", "#2b5876"),
            header_text_color: get_str(&map, "appearance", "header_text_color", "#ffffff"),
            logo_url: get_str(&map, "appearance", "logo_url", ""),
            favicon_url: get_str(&map, "appearance", "favicon_url", ""),
            login_bg_image: get_str(&map, "appearance", "login_bg_image", ""),
            registration_bg_image: get_str(&map, "appearance", "registration_bg_image", ""),
            custom_css: get_str(&map, "appearance", "custom_css", ""),
            custom_js: get_str(&map, "appearance", "custom_js", ""),
        },
        limits: LimitsConfig {
            max_characters: get_i64(&map, "limits", "max_characters", 63206),
            post_limit: get_i64(&map, "limits", "post_limit", 0),
            max_multi_images: get_i32(&map, "limits", "max_multi_images", 10),
        },
        seo: SeoConfig {
            site_keywords: get_str(&map, "seo", "site_keywords", ""),
            google_analytics_id: get_str(&map, "seo", "google_analytics_id", ""),
            facebook_pixel_id: get_str(&map, "seo", "facebook_pixel_id", ""),
            robots_txt: get_str(&map, "seo", "robots_txt", "User-agent: *\nAllow: /"),
        },
    })
}
