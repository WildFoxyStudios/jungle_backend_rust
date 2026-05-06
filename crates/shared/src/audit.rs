//! Admin audit log middleware.
//!
//! Intercepts mutating HTTP requests to `/v1/admin/*`, reads the JWT to identify
//! the admin user, captures the request body, and writes an entry to
//! `admin_audit_log` after the handler runs. Sensitive fields are redacted.

use axum::{
    body::{Body, Bytes, to_bytes},
    extract::{Request, State},
    http::{HeaderMap, Method, header},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde_json::Value;
use sqlx::PgPool;

use crate::auth::{AppState, Claims};

const MAX_BODY_BYTES: usize = 1024 * 1024; // 1 MB
const REDACTED: &str = "***REDACTED***";
const REDACT_KEYS: &[&str] = &[
    "password",
    "current_password",
    "new_password",
    "api_key",
    "apikey",
    "secret",
    "client_secret",
    "access_token",
    "refresh_token",
    "private_key",
];

/// Axum middleware that records every admin mutation.
pub async fn audit_admin(State(app_state): State<AppState>, req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path().to_string();

    // Only care about mutating verbs on /v1/admin/*
    let is_admin_mutation = path.starts_with("/v1/admin/")
        && matches!(
            method,
            Method::POST | Method::PUT | Method::PATCH | Method::DELETE
        );

    if !is_admin_mutation {
        return next.run(req).await;
    }

    // Extract identity + context from headers before the body is consumed
    let headers = req.headers().clone();
    let admin_user_id = extract_admin_id(
        &headers,
        &app_state.config.jwt_secret,
        app_state.config.jwt_secret_previous.as_deref(),
    );
    let ip = extract_ip(&headers);
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Split: read body bytes, reconstruct request so the handler still sees it
    let (parts, body) = req.into_parts();
    let body_bytes = match to_bytes(body, MAX_BODY_BYTES).await {
        Ok(b) => b,
        Err(_) => Bytes::new(),
    };

    let redacted_body = sanitize_body_json(&body_bytes);

    let new_req = Request::from_parts(parts, Body::from(body_bytes.clone()));
    let response = next.run(new_req).await;
    let status = response.status();

    // Fire-and-forget DB write; never block the response.
    if let Some(uid) = admin_user_id {
        let db = app_state.db.clone();
        let (resource_type, resource_id) = parse_resource(&path);
        let action = method.to_string();
        let endpoint = path.clone();
        let status_code = status.as_u16() as i32;
        let ip_clone = ip.clone();
        let ua = user_agent.clone();

        tokio::spawn(async move {
            let _ = write_audit_row(
                &db,
                uid,
                &action,
                &resource_type,
                resource_id.as_deref(),
                &endpoint,
                status_code,
                redacted_body.as_ref(),
                ip_clone.as_deref(),
                ua.as_deref(),
            )
            .await;
        });
    }

    response
}

fn extract_admin_id(
    headers: &HeaderMap,
    jwt_secret: &str,
    jwt_secret_previous: Option<&str>,
) -> Option<i64> {
    let auth = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let token = auth.strip_prefix("Bearer ")?;

    let kid = jsonwebtoken::decode_header(token).ok().and_then(|h| h.kid);
    let primary = match kid.as_deref() {
        Some("previous") => jwt_secret_previous.unwrap_or(jwt_secret),
        _ => jwt_secret,
    };

    let mut validation = Validation::default();
    validation.set_required_spec_claims(&["exp", "iat"]);

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(primary.as_bytes()),
        &validation,
    )
    .or_else(|e| {
        // On failure, attempt the previous secret as a rotation fallback
        // (covers un-kid'd legacy tokens).
        if let Some(prev) = jwt_secret_previous {
            decode::<Claims>(
                token,
                &DecodingKey::from_secret(prev.as_bytes()),
                &validation,
            )
        } else {
            Err(e)
        }
    })
    .ok()?;

    if data.claims.is_admin {
        Some(data.claims.sub)
    } else {
        None
    }
}

fn extract_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_resource(path: &str) -> (String, Option<String>) {
    // /v1/admin/users/42/ban → ("users", Some("42"))
    // /v1/admin/settings/email → ("settings", Some("email"))
    let parts: Vec<&str> = path
        .trim_start_matches('/')
        .trim_end_matches('/')
        .split('/')
        .collect();
    if parts.len() < 3 {
        return ("unknown".into(), None);
    }
    let resource_type = parts.get(2).copied().unwrap_or("unknown").to_string();
    let resource_id = parts.get(3).map(|s| s.to_string());
    (resource_type, resource_id)
}

fn sanitize_body_json(bytes: &[u8]) -> Option<Value> {
    if bytes.is_empty() {
        return None;
    }
    let parsed: Value = serde_json::from_slice(bytes).ok()?;
    Some(redact(parsed))
}

fn redact(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (k, v) in map {
                let lower = k.to_lowercase();
                if REDACT_KEYS.iter().any(|r| lower.contains(r)) {
                    out.insert(k, Value::String(REDACTED.into()));
                } else {
                    out.insert(k, redact(v));
                }
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(redact).collect()),
        other => other,
    }
}

#[allow(clippy::too_many_arguments)]
async fn write_audit_row(
    db: &PgPool,
    admin_user_id: i64,
    action: &str,
    resource_type: &str,
    resource_id: Option<&str>,
    endpoint: &str,
    status: i32,
    changes: Option<&Value>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO admin_audit_log
            (admin_user_id, action, resource_type, resource_id, endpoint,
             status, changes, ip_address, user_agent)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8::inet, $9)"#,
    )
    .bind(admin_user_id)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(endpoint)
    .bind(status)
    .bind(changes)
    .bind(ip_address)
    .bind(user_agent)
    .execute(db)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_top_level_password() {
        let v: Value = serde_json::from_str(r#"{"username":"x","password":"p"}"#).unwrap();
        let r = redact(v);
        assert_eq!(r["password"], REDACTED);
        assert_eq!(r["username"], "x");
    }

    #[test]
    fn redact_nested() {
        let v: Value = serde_json::from_str(
            r#"{"user":{"api_key":"secret","name":"a"}, "list": [{"secret":"x"}]}"#,
        )
        .unwrap();
        let r = redact(v);
        assert_eq!(r["user"]["api_key"], REDACTED);
        assert_eq!(r["user"]["name"], "a");
        assert_eq!(r["list"][0]["secret"], REDACTED);
    }

    #[test]
    fn parse_resource_with_id() {
        let (t, id) = parse_resource("/v1/admin/users/42/ban");
        assert_eq!(t, "users");
        assert_eq!(id.as_deref(), Some("42"));
    }

    #[test]
    fn parse_resource_without_id() {
        let (t, id) = parse_resource("/v1/admin/settings");
        assert_eq!(t, "settings");
        assert!(id.is_none());
    }

    #[test]
    fn extract_ip_from_xff() {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        assert_eq!(extract_ip(&h), Some("1.2.3.4".to_string()));
    }
}
