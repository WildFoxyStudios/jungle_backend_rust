use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub uuid: Uuid,
    pub username: String,
    pub email: String,
    pub phone_number: Option<String>,
    /// NULL for accounts created via OAuth/social login that have not yet set a
    /// local password via `POST /v1/auth/social/set-password`.
    pub password_hash: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub cover: String,
    pub about: String,
    pub gender: String,
    pub birthday: Option<time::Date>,
    pub country_id: Option<i32>,
    pub city: String,
    pub address: String,
    pub website: String,
    pub school: String,
    pub working: String,
    pub working_link: String,
    pub language: String,
    pub is_active: bool,
    pub is_admin: bool,
    /// Sunshine/WoWonder `admin=2` — may access admin panel (JWT + UI).
    pub is_moderator: bool,
    pub is_pro: i16,
    pub is_verified: bool,
    pub email_verified: bool,
    pub phone_verified: bool,
    pub email_code: String,
    pub privacy_settings: serde_json::Value,
    pub notification_settings: serde_json::Value,
    pub balance: rust_decimal::Decimal,
    pub wallet: rust_decimal::Decimal,
    pub points: i64,
    pub social_logins: serde_json::Value,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub last_seen: Option<OffsetDateTime>,
    pub is_online: bool,
    pub two_factor_enabled: bool,
    pub two_factor_method: Option<String>,
    pub two_factor_secret: Option<String>,
    pub deleted_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub is_fake: bool,
    pub monetization_enabled: bool,
    pub subscription_price: rust_decimal::Decimal,
    pub is_live: Option<bool>,
    pub live_stream_id: Option<String>,
    pub monetization_settings: serde_json::Value,
    pub android_device_id: Option<String>,
    pub ios_device_id: Option<String>,
    pub android_notification_id: Option<String>,
    pub ios_notification_id: Option<String>,
    pub social_links: serde_json::Value,
    pub start_up_info: bool,
    pub startup_image: bool,
    pub startup_follow: bool,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PublicUserRow {
    pub uuid: Uuid,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub cover: String,
    pub about: String,
    pub is_verified: bool,
    pub is_pro: i16,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicUser {
    pub uuid: Uuid,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub name: String,
    pub avatar: String,
    pub cover: String,
    pub about: String,
    pub is_verified: bool,
    pub is_pro: i16,
    pub is_online: bool,
}

impl From<&User> for PublicUser {
    fn from(u: &User) -> Self {
        let name = if u.first_name.is_empty() {
            u.username.clone()
        } else if u.last_name.is_empty() {
            u.first_name.clone()
        } else {
            format!("{} {}", u.first_name, u.last_name)
        };

        PublicUser {
            uuid: u.uuid,
            username: u.username.clone(),
            first_name: u.first_name.clone(),
            last_name: u.last_name.clone(),
            name,
            avatar: u.avatar.clone(),
            cover: u.cover.clone(),
            about: u.about.clone(),
            is_verified: u.is_verified,
            is_pro: u.is_pro,
            is_online: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthUserResponse {
    pub id: i64,
    pub uuid: Uuid,
    pub username: String,
    pub email: String,
    pub phone: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub name: String,
    pub avatar: String,
    pub cover: String,
    pub about: String,
    pub gender: String,
    pub birthday: Option<String>,
    pub location: String,
    pub website: String,
    pub school: String,
    pub working: String,
    pub working_link: String,
    pub social_links: serde_json::Value,
    pub is_verified: bool,
    pub is_pro: i16,
    pub is_admin: bool,
    pub is_moderator: bool,
    pub two_factor_enabled: bool,
    pub email_verified: bool,
    /// `true` if the account has a local email+password credential. OAuth-only
    /// users (`password_hash IS NULL`) should hit `POST /v1/auth/social/set-password`
    /// to set one before they can use `PUT /v1/auth/password`.
    pub has_password: bool,
}

impl From<&User> for AuthUserResponse {
    fn from(u: &User) -> Self {
        let name = if u.first_name.is_empty() {
            u.username.clone()
        } else {
            format!("{} {}", u.first_name, u.last_name)
                .trim()
                .to_string()
        };

        AuthUserResponse {
            id: u.id,
            uuid: u.uuid,
            username: u.username.clone(),
            email: u.email.clone(),
            phone: u.phone_number.clone(),
            first_name: u.first_name.clone(),
            last_name: u.last_name.clone(),
            name,
            avatar: u.avatar.clone(),
            cover: u.cover.clone(),
            about: u.about.clone(),
            gender: u.gender.clone(),
            birthday: u.birthday.map(|d| d.to_string()),
            location: u.city.clone(),
            website: u.website.clone(),
            school: u.school.clone(),
            working: u.working.clone(),
            working_link: u.working_link.clone(),
            social_links: u.social_links.clone(),
            is_verified: u.is_verified,
            is_pro: u.is_pro,
            is_admin: u.is_admin,
            is_moderator: u.is_moderator,
            two_factor_enabled: u.two_factor_enabled,
            email_verified: u.email_verified,
            has_password: u
                .password_hash
                .as_deref()
                .map(|h| !h.is_empty())
                .unwrap_or(false),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginAttempt {
    pub id: i64,
    pub ip_address: String,
    pub user_id: Option<i64>,
    pub success: bool,
    pub attempted_at: OffsetDateTime,
}
