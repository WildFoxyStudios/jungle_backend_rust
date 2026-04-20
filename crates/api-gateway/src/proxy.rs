use axum::{
    body::Body,
    extract::State,
    http::{HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::{rate_limit::RateLimiter, routing::ServiceMap};

#[derive(Clone)]
pub struct GatewayState {
    pub client: reqwest::Client,
    pub services: Arc<ServiceMap>,
    pub rate_limiter: Arc<RateLimiter>,
}

pub async fn proxy_request(
    State(state): State<GatewayState>,
    req: Request<Body>,
) -> Response {
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();

    // ── Rate Limiting ────────────────────────────────────────────────────
    let client_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .split(',')
        .next()
        .unwrap_or("unknown")
        .trim()
        .to_string();

    let (max_req, window) = RateLimiter::config_for_path(&path);
    let rate_key = format!("rl:{}:{}", client_ip, path.split('/').take(4).collect::<Vec<_>>().join("/"));

    match state.rate_limiter.check(&rate_key, max_req, window).await {
        Err(retry_after) => {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                [
                    ("retry-after", retry_after.to_string()),
                    ("x-ratelimit-limit", max_req.to_string()),
                    ("x-ratelimit-remaining", "0".to_string()),
                ],
                "Rate limit exceeded",
            )
                .into_response();
        }
        Ok(remaining) => {
            // Will add headers to response later
            let _ = remaining;
        }
    }

    // ── Resolve upstream ─────────────────────────────────────────────────
    let upstream_base = match state.services.resolve(&path) {
        Some(url) => url,
        None => {
            return (StatusCode::NOT_FOUND, "No service found for this path").into_response();
        }
    };

    let upstream_url = format!("{}{}{}", upstream_base, path, query);

    // ── Forward request ──────────────────────────────────────────────────
    let method = req.method().clone();
    let mut headers = req.headers().clone();
    headers.remove("host");

    // Add forwarded headers
    if let Ok(val) = HeaderValue::from_str(&client_ip) {
        headers.insert("x-forwarded-for", val);
    }

    let max_body = if path.starts_with("/v1/media/") || path.starts_with("/v1/stories") || path.starts_with("/v1/reels") || path.starts_with("/v1/albums") {
        100 * 1024 * 1024 // 100 MB for media uploads
    } else {
        10 * 1024 * 1024 // 10 MB for everything else
    };

    let body_bytes = match axum::body::to_bytes(req.into_body(), max_body).await {
        Ok(b) => b,
        Err(_) => {
            return (StatusCode::PAYLOAD_TOO_LARGE, "Request body too large").into_response();
        }
    };

    let upstream_req = state
        .client
        .request(
            reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET),
            &upstream_url,
        )
        .headers(convert_headers(&headers))
        .body(body_bytes.to_vec());

    match upstream_req.send().await {
        Ok(resp) => {
            let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let resp_headers = resp.headers().clone();
            let body = resp.bytes().await.unwrap_or_default();

            let mut response = (status, body.to_vec()).into_response();
            for (k, v) in resp_headers.iter() {
                if let Ok(name) = axum::http::HeaderName::from_bytes(k.as_str().as_bytes())
                    && let Ok(val) = axum::http::HeaderValue::from_bytes(v.as_bytes()) {
                        response.headers_mut().insert(name, val);
                    }
            }
            response
        }
        Err(e) => {
            let is_connect = e.is_connect() || e.is_timeout();
            tracing::error!(upstream = %upstream_url, error = %e, connect_err = is_connect, "Upstream request failed");
            let msg = if is_connect {
                format!("Service unavailable – could not connect to upstream ({})", path.split('/').take(4).collect::<Vec<_>>().join("/"))
            } else {
                format!("Upstream error: {}", e)
            };
            (StatusCode::BAD_GATEWAY, msg).into_response()
        }
    }
}

fn convert_headers(headers: &axum::http::HeaderMap) -> reqwest::header::HeaderMap {
    let mut out = reqwest::header::HeaderMap::new();
    for (k, v) in headers.iter() {
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(k.as_str().as_bytes())
            && let Ok(val) = reqwest::header::HeaderValue::from_bytes(v.as_bytes()) {
                out.insert(name, val);
            }
    }
    out
}
