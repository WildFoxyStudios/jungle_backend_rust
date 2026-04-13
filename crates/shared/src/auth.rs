use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::events::EventBus;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub uuid: Uuid,
    pub is_admin: bool,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i64,
    pub uuid: Uuid,
    pub is_admin: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub redis: redis::aio::ConnectionManager,
    pub config: crate::config::SharedConfig,
    pub event_bus: std::sync::Arc<dyn EventBus>,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let auth_header = match parts.headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
            Some(h) => h,
            None => {
                tracing::debug!("No Authorization header present");
                return Err(ApiError::Unauthorized);
            }
        };

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(ApiError::Unauthorized)?;

        let key = DecodingKey::from_secret(state.config.jwt_secret.as_bytes());
        let mut validation = Validation::default();
        validation.set_required_spec_claims(&["exp", "iat"]);

        let token_data = match decode::<Claims>(token, &key, &validation) {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    error_kind = ?e.kind(),
                    token_prefix = &token[..token.len().min(20)],
                    "JWT decode failed"
                );
                return Err(ApiError::Unauthorized);
            }
        };
        let claims = token_data.claims;

        if claims.exp < OffsetDateTime::now_utc().unix_timestamp() {
            return Err(ApiError::Unauthorized);
        }

        Ok(AuthUser {
            user_id: claims.sub,
            uuid: claims.uuid,
            is_admin: claims.is_admin,
        })
    }
}

pub struct OptionalAuth(pub Option<AuthUser>);

impl FromRequestParts<AppState> for OptionalAuth {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        match AuthUser::from_request_parts(parts, state).await {
            Ok(user) => Ok(OptionalAuth(Some(user))),
            Err(_) => Ok(OptionalAuth(None)),
        }
    }
}

pub fn encode_access_token(
    user_id: i64,
    uuid: Uuid,
    is_admin: bool,
    secret: &str,
) -> Result<String, ApiError> {
    let now = OffsetDateTime::now_utc();
    let exp = now + time::Duration::minutes(15);

    let claims = Claims {
        sub: user_id,
        uuid,
        is_admin,
        exp: exp.unix_timestamp(),
        iat: now.unix_timestamp(),
    };

    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| ApiError::Internal("Failed to encode token".into()))
}

pub fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_access_token_returns_valid_jwt() {
        let uuid = Uuid::new_v4();
        let token = encode_access_token(42, uuid, false, "test-secret").unwrap();
        assert!(!token.is_empty());

        let key = DecodingKey::from_secret(b"test-secret");
        let mut validation = Validation::default();
        validation.set_required_spec_claims(&["exp", "iat"]);
        let data = decode::<Claims>(&token, &key, &validation).unwrap();
        assert_eq!(data.claims.sub, 42);
        assert_eq!(data.claims.uuid, uuid);
        assert!(!data.claims.is_admin);
    }

    #[test]
    fn test_encode_access_token_admin_flag() {
        let uuid = Uuid::new_v4();
        let token = encode_access_token(1, uuid, true, "admin-secret").unwrap();
        let key = DecodingKey::from_secret(b"admin-secret");
        let mut validation = Validation::default();
        validation.set_required_spec_claims(&["exp", "iat"]);
        let data = decode::<Claims>(&token, &key, &validation).unwrap();
        assert!(data.claims.is_admin);
    }

    #[test]
    fn test_hash_token_deterministic() {
        let hash1 = hash_token("my-refresh-token");
        let hash2 = hash_token("my-refresh-token");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_hash_token_different_inputs() {
        let h1 = hash_token("token-a");
        let h2 = hash_token("token-b");
        assert_ne!(h1, h2);
    }
}
