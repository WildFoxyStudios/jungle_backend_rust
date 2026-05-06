use std::collections::HashSet;

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts},
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::events::EventBus;
use crate::permissions::Permission;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,
    pub uuid: Uuid,
    pub is_admin: bool,
    /// Sunshine / WoWonder moderator (`admin=2`). Omitted in legacy JWTs → false.
    #[serde(default)]
    pub is_moderator: bool,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i64,
    pub uuid: Uuid,
    pub is_admin: bool,
    pub is_moderator: bool,
}

impl AuthUser {
    /// Full site admin or moderator — may open the admin Next app (PHP `Wo_IsAdmin` / `Wo_IsModerator`).
    #[inline]
    pub fn can_access_admin_panel(&self) -> bool {
        self.is_admin || self.is_moderator
    }

    /// Load effective permissions for this user (Redis-cached, TTL 60s).
    /// Super admins (`is_admin = true`) get the full permission set.
    pub async fn permissions(&self, state: &AppState) -> Result<HashSet<String>, ApiError> {
        let cache_key = format!("perms:{}", self.user_id);

        // 1. Try Redis cache
        let mut conn = state.redis.clone();
        let cached: Option<String> = conn.get(&cache_key).await.map_err(|e| {
            tracing::error!(error = %e, user_id = self.user_id, "Redis error loading permissions");
            ApiError::Internal("Cache error".into())
        })?;

        if let Some(json) = cached {
            let perms: HashSet<String> =
                serde_json::from_str(&json).unwrap_or_default();
            return Ok(perms);
        }

        // 2. Cache miss — load from DB
        let perms = if self.is_admin {
            // Super admin: all permissions
            Permission::all()
                .into_iter()
                .map(|p| p.as_key().to_string())
                .collect()
        } else {
            let row: Option<(serde_json::Value,)> =
                sqlx::query_as("SELECT permissions FROM users WHERE id = $1")
                    .bind(self.user_id)
                    .fetch_optional(&state.db)
                    .await
                    .map_err(|e| {
                        tracing::error!(error = %e, user_id = self.user_id, "DB error loading permissions");
                        ApiError::Internal("Database error".into())
                    })?;

            match row {
                Some((serde_json::Value::Object(obj),)) => obj
                    .into_iter()
                    .filter_map(|(k, v)| {
                        if v.as_str() == Some("true") || v.as_bool() == Some(true) {
                            Some(k)
                        } else {
                            None
                        }
                    })
                    .collect(),
                _ => HashSet::new(),
            }
        };

        // 3. Cache in Redis (TTL 60s)
        let json = serde_json::to_string(&perms).unwrap_or_default();
        let _: () = conn.set_ex(&cache_key, &json, 60).await.map_err(|e| {
            tracing::warn!(error = %e, "Redis cache write failed (non-fatal)");
        }).unwrap_or(());

        Ok(perms)
    }

    /// Require a specific permission. Returns `ApiError::Forbidden` if missing.
    pub async fn require_permission(
        &self,
        perm: Permission,
        state: &AppState,
    ) -> Result<(), ApiError> {
        let perms = self.permissions(state).await?;
        if perms.contains(perm.as_key()) {
            Ok(())
        } else {
            Err(ApiError::Forbidden(format!(
                "Missing permission: {}",
                perm.as_key()
            )))
        }
    }
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

        let auth_header = match parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
        {
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
        let kid = jsonwebtoken::decode_header(token).ok().and_then(|h| h.kid);

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
                // If decode failed and we have a previous secret, try it as a
                // fallback — regardless of `kid`. This covers both tokens
                // issued before `kid` was introduced (kid=None) and tokens
                // signed with the old secret during a rotation where the new
                // secret is now primary (kid="current" but signed by previous).
                if let Some(prev) = app_state.config.jwt_secret_previous.as_deref() {
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
                            is_moderator: claims.is_moderator,
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
            is_moderator: claims.is_moderator,
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
    is_moderator: bool,
    secret: &str,
) -> Result<String, ApiError> {
    let now = OffsetDateTime::now_utc();
    let exp = now + time::Duration::minutes(15);

    let claims = Claims {
        sub: user_id,
        uuid,
        is_admin,
        is_moderator,
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
        let token = encode_access_token(42, uuid, false, false, "test-secret").unwrap();
        assert!(!token.is_empty());

        let key = DecodingKey::from_secret(b"test-secret");
        let mut validation = Validation::default();
        validation.set_required_spec_claims(&["exp", "iat"]);
        let data = decode::<Claims>(&token, &key, &validation).unwrap();
        assert_eq!(data.claims.sub, 42);
        assert_eq!(data.claims.uuid, uuid);
        assert!(!data.claims.is_admin);
        assert!(!data.claims.is_moderator);
    }

    #[test]
    fn test_encode_access_token_admin_flag() {
        let uuid = Uuid::new_v4();
        let token = encode_access_token(1, uuid, true, false, "admin-secret").unwrap();
        let key = DecodingKey::from_secret(b"admin-secret");
        let mut validation = Validation::default();
        validation.set_required_spec_claims(&["exp", "iat"]);
        let data = decode::<Claims>(&token, &key, &validation).unwrap();
        assert!(data.claims.is_admin);
        assert!(!data.claims.is_moderator);
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
        let token = encode_access_token(7, Uuid::new_v4(), false, false, "s").unwrap();
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
                is_moderator: false,
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
