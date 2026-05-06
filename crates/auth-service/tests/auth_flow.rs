//! Integration smoke test for the auth flow.
//!
//! Requires a running PostgreSQL + Redis:
//!     docker compose -f backend/docker-compose.yml up -d postgres redis
//!
//! Sets `DATABASE_URL`, `REDIS_URL`, `JWT_SECRET` via env and hits the real
//! handlers through `reqwest` (no mocks). Each test creates a throwaway user
//! whose username is prefixed with `ci_`.
//!
//! These tests are skipped when `DATABASE_URL` is unset so `cargo test` in CI
//! without a DB does not fail spuriously.

#![cfg(feature = "integration")]

use serde_json::json;

const BASE: &str = "http://127.0.0.1:3001";

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap()
}

#[tokio::test]
async fn register_and_login_flow() {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let username = format!("ci_user_{ts}");
    let email = format!("ci_user_{ts}@example.test");
    let password = "VerySecurePassword!1";

    let http = client();

    // ── Register ──
    let reg = http
        .post(format!("{BASE}/v1/auth/register"))
        .json(&json!({
            "username": username,
            "email": email,
            "password": password,
            "first_name": "CI",
            "last_name": "Test"
        }))
        .send()
        .await
        .expect("register req");

    assert!(
        reg.status().is_success(),
        "register failed: {} {}",
        reg.status(),
        reg.text().await.unwrap_or_default()
    );

    let body: serde_json::Value = reg.json().await.unwrap();
    assert!(body["data"]["access_token"].is_string());
    assert!(body["data"]["refresh_token"].is_string());

    // ── Login ──
    let login = http
        .post(format!("{BASE}/v1/auth/login"))
        .json(&json!({ "identifier": username, "password": password }))
        .send()
        .await
        .expect("login req");

    assert!(
        login.status().is_success(),
        "login failed: {}",
        login.status()
    );
    let body: serde_json::Value = login.json().await.unwrap();
    let token = body["data"]["access_token"].as_str().unwrap();

    // ── GET /v1/users/me ──
    let me = http
        .get(format!("{BASE}/v1/users/me"))
        .bearer_auth(token)
        .send()
        .await
        .expect("me req");

    assert!(me.status().is_success());
    let body: serde_json::Value = me.json().await.unwrap();
    assert_eq!(body["data"]["username"], username);
}

#[tokio::test]
async fn wrong_password_returns_401() {
    let http = client();
    let resp = http
        .post(format!("{BASE}/v1/auth/login"))
        .json(&json!({ "identifier": "nobody", "password": "nope" }))
        .send()
        .await
        .expect("login req");
    assert_eq!(resp.status().as_u16(), 401);
}
