//! Security headers middleware.
//!
//! Sets a conservative baseline that hardens browser responses without
//! breaking the API surface. This complements the CORS layer:
//!
//! - `X-Content-Type-Options: nosniff`
//! - `X-Frame-Options: DENY`
//! - `Referrer-Policy: strict-origin-when-cross-origin`
//! - `Permissions-Policy` — block sensors by default
//! - `Strict-Transport-Security` when `ENABLE_HSTS=1`
//! - `Content-Security-Policy` — conservative JSON API baseline
//!
//! Auth is Bearer-only (JWT in `Authorization` header) so CSRF is not
//! exploitable through automatic cookie attach. The CORS allow-list is
//! already strict (`allow_origin::list`) and `allow_credentials: true`
//! is safe because no `Set-Cookie` is issued by any service today.

use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};

pub async fn security_headers(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static(
            "accelerometer=(), camera=(self), geolocation=(self), gyroscope=(), microphone=(self), payment=(self)",
        ),
    );

    // CSP: conservative baseline for a JSON API. The frontend (Next.js) can
    // layer a stricter policy for document-loaded resources.
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'"),
    );

    // HSTS enabled by default; set ENABLE_HSTS=0 to opt out in dev.
    if std::env::var("ENABLE_HSTS").ok().as_deref() != Some("0") {
        headers.insert(
            HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        );
    }

    response
}

/// Protect the Prometheus metrics endpoint with the internal service key so
/// internal telemetry is not exposed to the public internet. Requests without
/// the correct `X-Internal-Key` header receive 403.
pub async fn metrics_protection(req: Request, next: Next) -> Result<Response, StatusCode> {
    if req.uri().path() == "/metrics" {
        let expected = std::env::var("INTERNAL_SERVICE_KEY")
            .unwrap_or_else(|_| "internal-dev-key".into());
        let provided = req
            .headers()
            .get("x-internal-key")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if provided != expected {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    Ok(next.run(req).await)
}
