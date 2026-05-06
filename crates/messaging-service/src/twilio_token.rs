//! Twilio Video Access Token (HS256, JWT FPA v1) builder.
//!
//! Twilio video tokens are JWTs where the header carries `cty=twilio-fpa;v=1`
//! and the payload includes a `grants` object describing which Twilio products
//! the token unlocks (here: Video, scoped to a single room).
//!
//! Reference: https://www.twilio.com/docs/iam/access-tokens

use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
struct VideoGrant<'a> {
    room: &'a str,
}

#[derive(Debug, Serialize)]
struct Grants<'a> {
    identity: &'a str,
    video: VideoGrant<'a>,
}

#[derive(Debug, Serialize)]
struct Claims<'a> {
    jti: String,
    iss: &'a str,
    sub: &'a str,
    iat: u64,
    exp: u64,
    grants: Grants<'a>,
}

/// Mint a Twilio Video Access Token. Returns the encoded JWT.
///
/// - `account_sid` / `api_key_sid` / `api_key_secret` come from the Twilio
///   console (Account SID + an API Key created with the Twilio CLI/REST).
/// - `identity` is the Twilio "user" identity — we use the database user id.
/// - `room` scopes the token to a single room (matches the calls.room_name).
/// - `ttl_secs` is the token lifetime (Twilio caps this at 24h).
pub fn build(
    account_sid: &str,
    api_key_sid: &str,
    api_key_secret: &str,
    identity: &str,
    room: &str,
    ttl_secs: u64,
) -> Result<(String, u64), String> {
    if account_sid.is_empty() || api_key_sid.is_empty() || api_key_secret.is_empty() {
        return Err("Twilio account_sid, api_key_sid and api_key_secret are required".into());
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let exp = now + ttl_secs.min(24 * 3600);

    let claims = Claims {
        jti: format!("{}-{}", api_key_sid, now),
        iss: api_key_sid,
        sub: account_sid,
        iat: now,
        exp,
        grants: Grants {
            identity,
            video: VideoGrant { room },
        },
    };

    let mut header = Header::new(Algorithm::HS256);
    header.cty = Some("twilio-fpa;v=1".into());
    header.typ = Some("JWT".into());

    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_secret(api_key_secret.as_bytes()),
    )
    .map_err(|e| format!("Twilio JWT encode: {e}"))?;

    Ok((token, exp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_has_three_segments() {
        let (token, _) = build("ACxxxx", "SKxxxx", "secret", "user-42", "room-1", 3600).unwrap();
        assert_eq!(token.split('.').count(), 3);
    }

    #[test]
    fn rejects_empty_credentials() {
        assert!(build("", "SK", "sec", "u", "r", 60).is_err());
        assert!(build("AC", "", "sec", "u", "r", 60).is_err());
        assert!(build("AC", "SK", "", "u", "r", 60).is_err());
    }

    #[test]
    fn ttl_is_capped_at_24h() {
        let (_, exp) = build("AC", "SK", "sec", "u", "r", 999_999).unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(exp <= now + 24 * 3600 + 5);
    }
}
