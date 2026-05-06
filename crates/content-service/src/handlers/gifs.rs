use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{auth::AppState, errors::ApiError};

#[derive(Debug, Deserialize)]
pub struct GifSearchParams {
    pub q: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

fn default_limit() -> u32 {
    25
}

#[derive(Debug, Serialize, Clone)]
pub struct GifResult {
    pub id: String,
    pub title: String,
    pub url: String,
    pub preview_url: String,
    pub width: u32,
    pub height: u32,
}

/// GET /v1/gifs/search
///
/// Provider-agnostic GIF search proxy. Reads keys from `site_config`
/// (`category = 'gifs'`):
///
/// * `giphy_api_key` — preferred provider (more results, better quality).
/// * `tenor_api_key` — fallback if Giphy is not configured.
/// * `provider` — explicit override (`giphy` | `tenor`); empty = auto.
///
/// Returns a normalized payload regardless of which provider responded:
/// ```json
/// { "data": [{ "id", "title", "url", "preview_url", "width", "height" }],
///   "meta": { "provider": "giphy", "total_count": 0, "offset": 0 } }
/// ```
pub async fn search_gifs(
    State(state): State<AppState>,
    Query(params): Query<GifSearchParams>,
) -> Result<Json<Value>, ApiError> {
    let q = params.q.as_deref().unwrap_or("").trim();
    let limit = params.limit.clamp(1, 50);
    let offset = params.offset.min(500);

    let mut giphy_key: Option<String> = None;
    let mut tenor_key: Option<String> = None;
    let mut provider_pref: Option<String> = None;

    let rows: Vec<(String, Option<String>)> =
        sqlx::query_as("SELECT key, value FROM site_config WHERE category = 'gifs'")
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();

    for (key, value) in rows {
        match key.as_str() {
            "giphy_api_key" => giphy_key = value,
            "tenor_api_key" => tenor_key = value,
            "provider" => provider_pref = value,
            _ => {}
        }
    }

    let provider = match provider_pref.as_deref() {
        Some("giphy") if giphy_key.is_some() => "giphy",
        Some("tenor") if tenor_key.is_some() => "tenor",
        _ => {
            if giphy_key.is_some() {
                "giphy"
            } else if tenor_key.is_some() {
                "tenor"
            } else {
                return Err(ApiError::BadRequest(
                    "No GIF provider configured. Set giphy_api_key or tenor_api_key in admin settings.".into(),
                ));
            }
        }
    };

    let client = reqwest::Client::new();

    let (results, total) = match provider {
        "giphy" => fetch_giphy(&client, giphy_key.as_deref().unwrap_or(""), q, limit, offset).await?,
        "tenor" => fetch_tenor(&client, tenor_key.as_deref().unwrap_or(""), q, limit, offset).await?,
        _ => unreachable!(),
    };

    Ok(Json(json!({
        "data": results,
        "meta": {
            "provider": provider,
            "total_count": total,
            "offset": offset,
            "limit": limit,
        }
    })))
}

async fn fetch_giphy(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    limit: u32,
    offset: u32,
) -> Result<(Vec<GifResult>, u64), ApiError> {
    let url = if query.is_empty() {
        format!(
            "https://api.giphy.com/v1/gifs/trending?api_key={}&limit={}&offset={}",
            api_key, limit, offset
        )
    } else {
        format!(
            "https://api.giphy.com/v1/gifs/search?api_key={}&q={}&limit={}&offset={}",
            api_key,
            urlencoding::encode(query),
            limit,
            offset
        )
    };

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| ApiError::BadGateway(format!("Giphy unreachable: {e}")))?;

    if !resp.status().is_success() {
        return Err(ApiError::BadGateway(format!(
            "Giphy returned HTTP {}",
            resp.status()
        )));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ApiError::BadGateway(format!("Giphy bad JSON: {e}")))?;

    let total = body
        .pointer("/pagination/total_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let mut out = Vec::new();
    if let Some(arr) = body.get("data").and_then(|v| v.as_array()) {
        for it in arr {
            let id = it
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let title = it
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let images = it.get("images");
            let original = images.and_then(|v| v.get("original"));
            let preview = images.and_then(|v| v.get("fixed_width_small")).or(original);

            let url = original
                .and_then(|v| v.get("url"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let preview_url = preview
                .and_then(|v| v.get("url"))
                .and_then(|v| v.as_str())
                .unwrap_or(&url)
                .to_string();
            let width = original
                .and_then(|v| v.get("width"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0u32);
            let height = original
                .and_then(|v| v.get("height"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0u32);

            out.push(GifResult {
                id,
                title,
                url,
                preview_url,
                width,
                height,
            });
        }
    }

    Ok((out, total))
}

async fn fetch_tenor(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    limit: u32,
    offset: u32,
) -> Result<(Vec<GifResult>, u64), ApiError> {
    let url = if query.is_empty() {
        format!(
            "https://tenor.googleapis.com/v2/featured?key={}&limit={}&pos={}",
            api_key, limit, offset
        )
    } else {
        format!(
            "https://tenor.googleapis.com/v2/search?key={}&q={}&limit={}&pos={}",
            api_key,
            urlencoding::encode(query),
            limit,
            offset
        )
    };

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| ApiError::BadGateway(format!("Tenor unreachable: {e}")))?;

    if !resp.status().is_success() {
        return Err(ApiError::BadGateway(format!(
            "Tenor returned HTTP {}",
            resp.status()
        )));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ApiError::BadGateway(format!("Tenor bad JSON: {e}")))?;

    let mut out = Vec::new();
    if let Some(arr) = body.get("results").and_then(|v| v.as_array()) {
        for it in arr {
            let id = it
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let title = it
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let media = it.get("media_formats");
            let gif = media
                .and_then(|v| v.get("gif"))
                .or_else(|| media.and_then(|v| v.get("mediumgif")));
            let preview = media.and_then(|v| v.get("tinygif")).or(gif);

            let url = gif
                .and_then(|v| v.get("url"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let preview_url = preview
                .and_then(|v| v.get("url"))
                .and_then(|v| v.as_str())
                .unwrap_or(&url)
                .to_string();
            let dims = gif.and_then(|v| v.get("dims")).and_then(|v| v.as_array());
            let (width, height): (u32, u32) = dims
                .and_then(|a| Some((a.first()?.as_u64()? as u32, a.get(1)?.as_u64()? as u32)))
                .unwrap_or_default();

            out.push(GifResult {
                id,
                title,
                url,
                preview_url,
                width,
                height,
            });
        }
    }

    let total = out.len() as u64;
    Ok((out, total))
}
