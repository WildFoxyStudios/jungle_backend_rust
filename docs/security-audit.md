# Security audit — CSRF, cookies, headers (QA-4)

This audit covers the surface area touched by browser clients: the
api-gateway, the auth flow, and the cookies/credentials behaviour of
the Next.js apps.

## 1. Authentication transport

- All Rust services accept JWTs through the `Authorization: Bearer
  <token>` header. No service issues a `Set-Cookie` response anywhere
  in the codebase (verified by ripgrep on `set_cookie` /
  `Set-Cookie` across `backend/`).
- The web client persists the access token in `localStorage` and the
  refresh token through a `/v1/auth/refresh` POST that takes the
  refresh token in the JSON body. There is no httpOnly session
  cookie today.
- Because authentication is not cookie-based, **CSRF is not currently
  exploitable** on JSON POSTs — a cross-site form cannot attach a
  Bearer header and the browser will not auto-attach an
  `Authorization` header.

## 2. CORS

`backend/crates/api-gateway/src/main.rs` configures
`tower_http::cors::CorsLayer` with:

- `allow_origin: AllowOrigin::list(allowed_origins)` — strict origin
  allowlist driven by the `ALLOWED_ORIGINS` env var.
- `allow_credentials: true` — required because the web client uses
  `credentials: "include"` for forward compatibility (e.g. future
  refresh-token cookies). Safe today because no service emits
  cookies.
- `allow_headers` is restricted to `Content-Type`, `Authorization`,
  `Accept`, `Origin`, `Cookie`.

If a future change introduces a session cookie, the cookie MUST be
issued with `HttpOnly; Secure; SameSite=Lax` and a CSRF middleware
must be added to the gateway before merging.

## 3. Security response headers

A `security_headers` Axum middleware is registered in the api-gateway
and sets the following on every response:

- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Permissions-Policy: accelerometer=(), camera=(self),
  geolocation=(self), gyroscope=(), microphone=(self),
  payment=(self)`
- `Strict-Transport-Security: max-age=31536000; includeSubDomains`
  when `ENABLE_HSTS=1` (recommended in production behind TLS).

## 4. Refresh-token storage

The refresh token is currently stored in `localStorage`. This is
acceptable for the web-only mandate but exposes the token to XSS.
Mitigations in place:

- Strict CSP can be added in the Next.js app via
  `next.config.ts` `headers()` if needed (not currently active).
- The token is only used by the api-client over HTTPS once
  `ENABLE_HSTS=1` is set.

## 5. Recommended follow-ups

- Add Next.js `headers()` in `frontend/apps/web/next.config.ts` with
  CSP, `X-Frame-Options: DENY`, and `Referrer-Policy` mirrored from
  the gateway, so static assets and pages also gain the same defaults.
- Migrate the refresh token to an httpOnly cookie + add a double-submit
  CSRF token for `/v1/auth/refresh` once the cookie is in place.
- Audit third-party scripts (Stripe.js, Sentry, PostHog) and add them
  to a strict `script-src` allowlist.

## 6. Findings & status

| Item                              | Status |
|-----------------------------------|--------|
| Bearer-only auth                  | ok     |
| CORS allowlist                    | ok     |
| `allow_credentials: true`         | ok (defensive — no cookies today) |
| Security headers in gateway       | added (this PR) |
| HSTS opt-in via env               | added (this PR) |
| CSP in Next.js app                | recommended, not yet enforced |
| Refresh token in httpOnly cookie  | future work |
