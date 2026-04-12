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
    pub password_hash: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub cover: String,
    pub about: String,
    pub gender: String,
    pub birthday: Option<time::Date>,
    pub country_id: Option<i32>,
    pub city: String,
    pub language: String,
    pub is_active: bool,
    pub is_admin: bool,
    pub is_pro: i16,
    pub is_verified: bool,
    pub email_verified: bool,
    pub phone_verified: bool,
    pub two_factor_enabled: bool,
    pub two_factor_method: Option<String>,
    pub two_factor_secret: Option<String>,
    pub last_seen: Option<OffsetDateTime>,
    pub deleted_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
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
    pub uuid: Uuid,
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub name: String,
    pub avatar: String,
    pub is_verified: bool,
    pub is_pro: i16,
    pub is_admin: bool,
    pub two_factor_enabled: bool,
    pub email_verified: bool,
}

impl From<&User> for AuthUserResponse {
    fn from(u: &User) -> Self {
        let name = if u.first_name.is_empty() {
            u.username.clone()
        } else {
            format!("{} {}", u.first_name, u.last_name).trim().to_string()
        };

        AuthUserResponse {
            uuid: u.uuid,
            username: u.username.clone(),
            email: u.email.clone(),
            first_name: u.first_name.clone(),
            last_name: u.last_name.clone(),
            name,
            avatar: u.avatar.clone(),
            is_verified: u.is_verified,
            is_pro: u.is_pro,
            is_admin: u.is_admin,
            two_factor_enabled: u.two_factor_enabled,
            email_verified: u.email_verified,
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
