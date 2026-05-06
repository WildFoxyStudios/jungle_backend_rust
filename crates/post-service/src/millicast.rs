//! Thin Millicast Director proxy.
//!
//! Frontends use the official `@millicast/sdk` which expects two pieces of
//! info to start a publish/subscribe session:
//!   1. A list of WebSocket endpoints (`urls`) returned by Millicast Director.
//!   2. A short-lived JWT (`jwt`) to use as bearer in the websocket negotiate
//!      message.
//!
//! Director itself requires a long-lived "publish token" or "subscribe token"
//! which we keep server-side in `site_config`. The browser never sees those
//! secrets — it only receives the negotiated short-lived JWT + URL list.
//!
//! Reference: https://docs.dolby.io/streaming-apis/reference/director_publish

use serde::{Deserialize, Serialize};

const DIRECTOR_PUBLISH: &str = "https://director.millicast.com/api/director/publish";
const DIRECTOR_SUBSCRIBE: &str = "https://director.millicast.com/api/director/subscribe";

#[derive(Debug, Serialize)]
struct DirectorRequest<'a> {
    #[serde(rename = "streamName")]
    stream_name: &'a str,
    #[serde(rename = "streamAccountId", skip_serializing_if = "Option::is_none")]
    stream_account_id: Option<&'a str>,
    #[serde(rename = "streamType")]
    stream_type: &'a str,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DirectorResponse {
    pub jwt: String,
    pub urls: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DirectorEnvelope {
    data: DirectorResponse,
}

/// Mint publish credentials for a given stream. Requires the long-lived
/// `publish_token` that Millicast bound to your account.
pub async fn publish_token(
    http: &reqwest::Client,
    publish_token: &str,
    stream_name: &str,
) -> Result<DirectorResponse, String> {
    director_call(http, DIRECTOR_PUBLISH, publish_token, stream_name, None).await
}

/// Mint subscribe credentials so a viewer can join an existing stream.
pub async fn subscribe_token(
    http: &reqwest::Client,
    subscribe_token: &str,
    stream_name: &str,
    account_id: Option<&str>,
) -> Result<DirectorResponse, String> {
    director_call(
        http,
        DIRECTOR_SUBSCRIBE,
        subscribe_token,
        stream_name,
        account_id,
    )
    .await
}

async fn director_call(
    http: &reqwest::Client,
    url: &str,
    bearer: &str,
    stream_name: &str,
    account_id: Option<&str>,
) -> Result<DirectorResponse, String> {
    if bearer.is_empty() || stream_name.is_empty() {
        return Err("token and stream_name are required".into());
    }

    let req = DirectorRequest {
        stream_name,
        stream_account_id: account_id,
        stream_type: "WebRtc",
    };

    let resp = http
        .post(url)
        .header("Authorization", format!("Bearer {}", bearer))
        .header("Content-Type", "application/json")
        .json(&req)
        .send()
        .await
        .map_err(|e| format!("Millicast Director: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Millicast Director {status}: {body}"));
    }

    let env: DirectorEnvelope = resp
        .json()
        .await
        .map_err(|e| format!("Millicast Director response: {e}"))?;
    Ok(env.data)
}
