//! VAPID Web Push (W3C Push API) sender.
//!
//! Uses the pure-rust `web-push-native` crate to build the encrypted/VAPID
//! signed `http::Request`, then delivers it through the workspace
//! `reqwest` client (rustls). This avoids OpenSSL entirely on Windows
//! while still implementing RFC8030/RFC8291/RFC8292 correctly.
//!
//! VAPID config is read from `site_config` (`vapid_private_key`,
//! `vapid_public_key`, `vapid_subject`) with env-var fallback for
//! local/dev runs.

use base64ct::{Base64UrlUnpadded, Encoding};
use serde::Serialize;
use sqlx::PgPool;
use web_push_native::{
    Auth, WebPushBuilder, jwt_simple::algorithms::ES256KeyPair, p256::PublicKey,
};

#[derive(Debug, Clone, Serialize)]
pub struct WebPushPayload<'a> {
    pub title: &'a str,
    pub body: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<&'a str>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct StoredSubscription {
    pub id: i64,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}

/// Outcome of a single delivery attempt.
#[derive(Debug, Clone, Copy)]
pub enum DeliveryStatus {
    /// 2xx — accepted by the push service.
    Delivered,
    /// 404/410 — subscription is gone, the caller should drop it.
    Gone,
}

pub struct WebPushSender {
    client: reqwest::Client,
    key_pair_bytes: Vec<u8>,
    subject: String,
}

impl WebPushSender {
    pub async fn from_db_or_env(db: &PgPool) -> Option<Self> {
        let (priv_key_b64, subject) = load_vapid_config(db).await?;

        // Validate the key once on startup so we fail fast on misconfig.
        let bytes = Base64UrlUnpadded::decode_vec(&priv_key_b64).ok()?;
        if ES256KeyPair::from_bytes(&bytes).is_err() {
            tracing::warn!("VAPID private key is invalid; web push disabled");
            return None;
        }

        Some(Self {
            client: reqwest::Client::new(),
            key_pair_bytes: bytes,
            subject,
        })
    }

    pub async fn send(
        &self,
        sub: &StoredSubscription,
        payload: &WebPushPayload<'_>,
    ) -> Result<DeliveryStatus, String> {
        let endpoint = sub
            .endpoint
            .parse::<http::Uri>()
            .map_err(|e| format!("invalid endpoint: {e}"))?;
        let p256dh_bytes = Base64UrlUnpadded::decode_vec(&sub.p256dh)
            .map_err(|e| format!("invalid p256dh: {e}"))?;
        let auth_bytes =
            Base64UrlUnpadded::decode_vec(&sub.auth).map_err(|e| format!("invalid auth: {e}"))?;
        if auth_bytes.len() != 16 {
            return Err("auth key must be 16 bytes".into());
        }
        let pubkey = PublicKey::from_sec1_bytes(&p256dh_bytes)
            .map_err(|e| format!("invalid p256dh pubkey: {e}"))?;
        let auth = Auth::clone_from_slice(&auth_bytes);

        let key_pair = ES256KeyPair::from_bytes(&self.key_pair_bytes)
            .map_err(|e| format!("invalid VAPID key: {e}"))?;

        let builder =
            WebPushBuilder::new(endpoint, pubkey, auth).with_vapid(&key_pair, &self.subject);

        let body = serde_json::to_vec(payload).map_err(|e| format!("serialize payload: {e}"))?;
        let request: http::Request<Vec<u8>> = builder
            .build(body)
            .map_err(|e| format!("build push request: {e}"))?;

        // Convert http::Request -> reqwest::Request.
        let (parts, body) = request.into_parts();
        let url = parts.uri.to_string();
        let mut req = self.client.post(&url).body(body);
        for (name, value) in parts.headers.iter() {
            req = req.header(name.as_str(), value.as_bytes());
        }

        let resp = req.send().await.map_err(|e| format!("send push: {e}"))?;
        let status = resp.status();
        if status.is_success() {
            return Ok(DeliveryStatus::Delivered);
        }
        if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::GONE {
            return Ok(DeliveryStatus::Gone);
        }
        let text = resp.text().await.unwrap_or_default();
        Err(format!("push service error {status}: {text}"))
    }
}

async fn load_vapid_config(db: &PgPool) -> Option<(String, String)> {
    let priv_db: Option<String> =
        sqlx::query_scalar("SELECT value FROM site_config WHERE key = 'vapid_private_key'")
            .fetch_optional(db)
            .await
            .ok()
            .flatten();

    let subject_db: Option<String> =
        sqlx::query_scalar("SELECT value FROM site_config WHERE key = 'vapid_subject'")
            .fetch_optional(db)
            .await
            .ok()
            .flatten();

    let priv_key = priv_db
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("VAPID_PRIVATE_KEY").ok())
        .filter(|s| !s.trim().is_empty())?;

    let subject = subject_db
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("VAPID_SUBJECT").ok())
        .unwrap_or_else(|| "mailto:admin@example.com".into());

    Some((priv_key, subject))
}
