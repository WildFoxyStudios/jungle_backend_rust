use axum::{
    extract::{FromRef, FromRequestParts},
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

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

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

        // Peek at the `kid` header to select the right secret. Falls back to
        // the current secret for tokens issued before rotation was enabled.
        let kid = jsonwebtoken::decode_header(token)
            .ok()
            .and_then(|h| h.kid);

        let secret = match kid.as_deref() {
            Some("previous") => app_state
                .config
                .jwt_secret_previous
                .as_deref()
                .unwrap_or(&app_state.config.jwt_secret),
            _ => app_state.config.jwt_secret.as_str(),
        };

        let key = DecodingKey::from_secret(secret.as_bytes());
        let mut validation = Validation::default();
        validation.set_required_spec_claims(&["exp", "iat"]);

        let token_data = match decode::<Claims>(token, &key, &validation) {
            Ok(data) => data,
            Err(e) => {
                // If current secret failed and we have a previous secret, try
                // it as a last-resort fallback (for tokens issued without a
                // `kid`).
                if let (None, Some(prev)) = (&kid, app_state.config.jwt_secret_previous.as_deref()) {
                    let prev_key = DecodingKey::from_secret(prev.as_bytes());
                    if let Ok(data) = decode::<Claims>(token, &prev_key, &validation) {
                        let claims = data.claims;
                        if claims.exp < OffsetDateTime::now_utc().unix_timestamp() {
                            return Err(ApiError::Unauthorized);
                        }
                        return Ok(AuthUser {
                            user_id: claims.sub,
                            uuid: claims.uuid,
                            is_admin: claims.is_admin,
                        });
                    }
                }
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

impl<S> FromRequestParts<S> for OptionalAuth
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
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

    // Stamp a `kid` so the decoder can pick the right secret during rotation.
    // Tokens minted before this change had no kid; the decoder falls back to
    // the current secret in that case and to JWT_SECRET_PREVIOUS on failure.
    let header = jsonwebtoken::Header {
        kid: Some("current".to_string()),
        ..jsonwebtoken::Header::default()
    };

    jsonwebtoken::encode(
        &header,
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

    #[test]
    fn test_encoded_token_carries_kid_current() {
        let token = encode_access_token(7, Uuid::new_v4(), false, "s").unwrap();
        let header = jsonwebtoken::decode_header(&token).unwrap();
        assert_eq!(header.kid.as_deref(), Some("current"));
    }

    #[test]
    fn test_previous_secret_validates_legacy_tokens() {
        let uuid = Uuid::new_v4();
        // Manually encode a token WITHOUT kid using the "previous" secret —
        // this simulates a token minted before rotation was enabled.
        let legacy = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(), // no kid
            &Claims {
                sub: 99,
                uuid,
                is_admin: false,
                exp: (OffsetDateTime::now_utc() + time::Duration::minutes(10)).unix_timestamp(),
                iat: OffsetDateTime::now_utc().unix_timestamp(),
            },
            &jsonwebtoken::EncodingKey::from_secret(b"old-secret"),
        )
        .unwrap();

        // Simulate the decoder logic: try current, then previous.
        let current = DecodingKey::from_secret(b"new-secret");
        let previous = DecodingKey::from_secret(b"old-secret");
        let mut validation = Validation::default();
        validation.set_required_spec_claims(&["exp", "iat"]);

        assert!(decode::<Claims>(&legacy, &current, &validation).is_err());
        let data = decode::<Claims>(&legacy, &previous, &validation).unwrap();
        assert_eq!(data.claims.sub, 99);
    }
}
