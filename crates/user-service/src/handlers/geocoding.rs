//! Geocoding & reverse-geocoding façade.
//!
//! The frontend calls these endpoints when it needs to translate between
//! free-text addresses and `(lat, lng)` pairs (used by the address picker,
//! "nearby users" UI, and the live-stream location label). The actual
//! upstream is selected at request time from `site_config.third_party`:
//!
//! - `maps_provider = "google"`   → Google Geocoding API
//! - `maps_provider = "yandex"`   → Yandex Geocoder HTTP API
//! - anything else                → 503 with a descriptive error
//!
//! Provider responses are normalised into `{ lat, lng, formatted_address,
//! provider }` so the frontend never has to branch on the upstream shape.

use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

#[derive(Debug, Deserialize)]
pub struct GeocodeQuery {
    /// Address to forward-geocode. Mutually exclusive with `lat`+`lng`.
    pub q: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    /// Optional override; defaults to `site_config.third_party.maps_provider`.
    pub provider: Option<String>,
}

#[derive(Debug, Serialize)]
struct GeocodeHit {
    lat: f64,
    lng: f64,
    formatted_address: String,
    provider: &'static str,
}

/// GET /v1/users/geocode — forward or reverse geocode in a single endpoint.
///
/// - `?q=Madrid+Plaza+Mayor` → forward geocode
/// - `?lat=40.4&lng=-3.7`    → reverse geocode
pub async fn geocode(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<GeocodeQuery>,
) -> Result<Json<Value>, ApiError> {
    let has_query = params
        .q
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let has_coords = params.lat.is_some() && params.lng.is_some();
    if !has_query && !has_coords {
        return Err(ApiError::BadRequest(
            "Provide either `q` (forward) or `lat`+`lng` (reverse)".into(),
        ));
    }

    let provider = match params.provider.as_deref() {
        Some(p) if !p.is_empty() => p.to_string(),
        _ => read_config(&state, "maps_provider")
            .await?
            .unwrap_or_else(|| "google".into()),
    };

    let http = reqwest::Client::new();
    let hit = match provider.as_str() {
        "yandex" => {
            let key = read_config(&state, "yandex_geocoder_api_key")
                .await?
                .or(read_config(&state, "yandex_maps_api_key").await?)
                .ok_or_else(|| {
                    ApiError::BadRequest("Yandex Maps API key is not configured".into())
                })?;
            yandex_geocode(&http, &key, &params).await?
        }
        "google" => {
            let key = read_config(&state, "google_maps_api_key")
                .await?
                .ok_or_else(|| {
                    ApiError::BadRequest("Google Maps API key is not configured".into())
                })?;
            google_geocode(&http, &key, &params).await?
        }
        other => {
            return Err(ApiError::BadRequest(format!(
                "Unsupported maps provider: {other}"
            )));
        }
    };

    Ok(Json(json!({ "data": hit })))
}

async fn read_config(state: &AppState, key: &str) -> Result<Option<String>, ApiError> {
    let v = sqlx::query_scalar::<_, Option<String>>(
        "SELECT value FROM site_config WHERE category = 'third_party' AND key = $1",
    )
    .bind(key)
    .fetch_optional(&state.db)
    .await?;
    Ok(v.flatten().filter(|v| !v.trim().is_empty()))
}

async fn yandex_geocode(
    http: &reqwest::Client,
    api_key: &str,
    params: &GeocodeQuery,
) -> Result<GeocodeHit, ApiError> {
    // Yandex expects `geocode=lng,lat` for reverse and a free-text string for forward.
    let geocode_param = if let (Some(lat), Some(lng)) = (params.lat, params.lng) {
        format!("{lng},{lat}")
    } else {
        params.q.clone().unwrap_or_default()
    };

    let resp = http
        .get("https://geocode-maps.yandex.ru/1.x/")
        .query(&[
            ("apikey", api_key),
            ("geocode", geocode_param.as_str()),
            ("format", "json"),
            ("results", "1"),
            ("lang", "en_US"),
        ])
        .send()
        .await
        .map_err(|e| ApiError::Internal(format!("Yandex Geocoder request: {e}")))?;

    if !resp.status().is_success() {
        return Err(ApiError::Internal(format!(
            "Yandex Geocoder HTTP {}",
            resp.status()
        )));
    }

    let body: Value = resp
        .json()
        .await
        .map_err(|e| ApiError::Internal(format!("Yandex Geocoder JSON: {e}")))?;

    let member = body
        .pointer("/response/GeoObjectCollection/featureMember/0/GeoObject")
        .ok_or_else(|| ApiError::BadRequest("No geocoding results".into()))?;
    let pos = member
        .pointer("/Point/pos")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::Internal("Yandex result missing Point.pos".into()))?;
    let mut parts = pos.split_whitespace();
    let lng: f64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let lat: f64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let formatted = member
        .pointer("/metaDataProperty/GeocoderMetaData/text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    Ok(GeocodeHit {
        lat,
        lng,
        formatted_address: formatted,
        provider: "yandex",
    })
}

async fn google_geocode(
    http: &reqwest::Client,
    api_key: &str,
    params: &GeocodeQuery,
) -> Result<GeocodeHit, ApiError> {
    let mut query: Vec<(&str, String)> = vec![("key", api_key.to_string())];
    if let (Some(lat), Some(lng)) = (params.lat, params.lng) {
        query.push(("latlng", format!("{lat},{lng}")));
    } else if let Some(q) = params.q.as_ref() {
        query.push(("address", q.clone()));
    }

    let resp = http
        .get("https://maps.googleapis.com/maps/api/geocode/json")
        .query(&query)
        .send()
        .await
        .map_err(|e| ApiError::Internal(format!("Google Geocoder request: {e}")))?;

    if !resp.status().is_success() {
        return Err(ApiError::Internal(format!(
            "Google Geocoder HTTP {}",
            resp.status()
        )));
    }

    let body: Value = resp
        .json()
        .await
        .map_err(|e| ApiError::Internal(format!("Google Geocoder JSON: {e}")))?;

    let first = body
        .get("results")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .ok_or_else(|| ApiError::BadRequest("No geocoding results".into()))?;

    let lat = first
        .pointer("/geometry/location/lat")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let lng = first
        .pointer("/geometry/location/lng")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let formatted = first
        .get("formatted_address")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    Ok(GeocodeHit {
        lat,
        lng,
        formatted_address: formatted,
        provider: "google",
    })
}
