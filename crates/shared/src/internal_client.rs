use std::collections::HashMap;

use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;

use crate::errors::ApiError;
use crate::resilience::CircuitBreaker;

/// HTTP client for synchronous inter-service communication.
///
/// Each microservice that needs to call another (e.g. post-service calling
/// user-service for publisher data) uses this client.  It carries an internal
/// shared key so downstream services can distinguish internal calls from
/// external user requests.
pub struct InternalClient {
    client: reqwest::Client,
    service_urls: HashMap<String, String>,
    circuit_breakers: HashMap<String, CircuitBreaker>,
}

#[derive(serde::Deserialize)]
struct ApiResponse<T> {
    data: T,
}

impl InternalClient {
    /// Build from environment variables.
    ///
    /// Expected env vars per service: `<SERVICE>_URL`, e.g.
    /// `USER_SERVICE_URL=http://user-service:3002`.
    pub fn from_env() -> Self {
        let services = [
            ("auth-service", "AUTH_SERVICE_URL"),
            ("user-service", "USER_SERVICE_URL"),
            ("post-service", "POST_SERVICE_URL"),
            ("messaging-service", "MESSAGING_SERVICE_URL"),
            ("media-service", "MEDIA_SERVICE_URL"),
            ("notification-service", "NOTIFICATION_SERVICE_URL"),
            ("group-page-service", "GROUP_PAGE_SERVICE_URL"),
            ("content-service", "CONTENT_SERVICE_URL"),
            ("commerce-service", "COMMERCE_SERVICE_URL"),
            ("admin-service", "ADMIN_SERVICE_URL"),
            ("payment-service", "PAYMENT_SERVICE_URL"),
            ("realtime-service", "REALTIME_SERVICE_URL"),
            ("ai-service", "AI_SERVICE_URL"),
        ];

        let port_defaults: HashMap<&str, &str> = [
            ("AUTH_SERVICE_URL", "http://127.0.0.1:3001"),
            ("USER_SERVICE_URL", "http://127.0.0.1:3002"),
            ("POST_SERVICE_URL", "http://127.0.0.1:3003"),
            ("MESSAGING_SERVICE_URL", "http://127.0.0.1:3004"),
            ("MEDIA_SERVICE_URL", "http://127.0.0.1:3005"),
            ("NOTIFICATION_SERVICE_URL", "http://127.0.0.1:3006"),
            ("GROUP_PAGE_SERVICE_URL", "http://127.0.0.1:3007"),
            ("CONTENT_SERVICE_URL", "http://127.0.0.1:3008"),
            ("COMMERCE_SERVICE_URL", "http://127.0.0.1:3009"),
            ("ADMIN_SERVICE_URL", "http://127.0.0.1:3010"),
            ("PAYMENT_SERVICE_URL", "http://127.0.0.1:3011"),
            ("REALTIME_SERVICE_URL", "http://127.0.0.1:3012"),
            ("AI_SERVICE_URL", "http://127.0.0.1:3013"),
        ]
        .into();

        let mut service_urls = HashMap::new();
        let mut circuit_breakers = HashMap::new();

        for (name, env_key) in services {
            let url = std::env::var(env_key)
                .unwrap_or_else(|_| port_defaults.get(env_key).unwrap_or(&"").to_string());
            if !url.is_empty() {
                service_urls.insert(name.to_string(), url);
                circuit_breakers.insert(name.to_string(), CircuitBreaker::new(5, 30));
            }
        }

        let internal_key =
            std::env::var("INTERNAL_SERVICE_KEY").unwrap_or_else(|_| "internal-dev-key".into());

        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Internal-Key",
            HeaderValue::from_str(&internal_key).unwrap_or_else(|_| HeaderValue::from_static("")),
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .default_headers(headers)
            .build()
            .expect("Failed to build internal HTTP client");

        let _ = &internal_key; // already set in default headers

        Self {
            client,
            service_urls,
            circuit_breakers,
        }
    }

    fn base_url(&self, service: &str) -> Result<&str, ApiError> {
        self.service_urls
            .get(service)
            .map(|s| s.as_str())
            .ok_or_else(|| ApiError::Internal(format!("Service URL not configured: {}", service)))
    }

    /// Perform a GET request to an internal service endpoint.
    pub async fn get<T: DeserializeOwned>(&self, service: &str, path: &str) -> Result<T, ApiError> {
        let base = self.base_url(service)?;
        let url = format!("{}{}", base, path);

        if let Some(cb) = self.circuit_breakers.get(service)
            && cb.is_open()
        {
            return Err(ApiError::Internal(format!(
                "Circuit breaker open for {}",
                service
            )));
        }

        let result = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ApiError::Internal(format!("Internal GET {} failed: {}", url, e)))?;

        if let Some(cb) = self.circuit_breakers.get(service) {
            if result.status().is_server_error() {
                cb.record_failure();
            } else {
                cb.record_success();
            }
        }

        if !result.status().is_success() {
            return Err(ApiError::Internal(format!(
                "Internal GET {} returned {}",
                url,
                result.status()
            )));
        }

        let resp: ApiResponse<T> = result
            .json()
            .await
            .map_err(|e| ApiError::Internal(format!("Deserialize from {} failed: {}", url, e)))?;

        Ok(resp.data)
    }

    /// Perform a POST request with a JSON body to an internal service endpoint.
    pub async fn post<B: serde::Serialize, T: DeserializeOwned>(
        &self,
        service: &str,
        path: &str,
        body: &B,
    ) -> Result<T, ApiError> {
        let base = self.base_url(service)?;
        let url = format!("{}{}", base, path);

        if let Some(cb) = self.circuit_breakers.get(service)
            && cb.is_open()
        {
            return Err(ApiError::Internal(format!(
                "Circuit breaker open for {}",
                service
            )));
        }

        let result = self
            .client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| ApiError::Internal(format!("Internal POST {} failed: {}", url, e)))?;

        if let Some(cb) = self.circuit_breakers.get(service) {
            if result.status().is_server_error() {
                cb.record_failure();
            } else {
                cb.record_success();
            }
        }

        if !result.status().is_success() {
            return Err(ApiError::Internal(format!(
                "Internal POST {} returned {}",
                url,
                result.status()
            )));
        }

        let resp: ApiResponse<T> = result
            .json()
            .await
            .map_err(|e| ApiError::Internal(format!("Deserialize from {} failed: {}", url, e)))?;

        Ok(resp.data)
    }

    /// Send a fire-and-forget POST (ignores response body). Useful for
    /// triggering side-effects like WebSocket pushes.
    pub async fn post_fire_and_forget<B: serde::Serialize>(
        &self,
        service: &str,
        path: &str,
        body: &B,
    ) -> Result<(), ApiError> {
        let base = self.base_url(service)?;
        let url = format!("{}{}", base, path);

        let result = self
            .client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| ApiError::Internal(format!("Internal POST {} failed: {}", url, e)))?;

        if let Some(cb) = self.circuit_breakers.get(service) {
            if result.status().is_server_error() {
                cb.record_failure();
            } else {
                cb.record_success();
            }
        }

        Ok(())
    }

    /// Convenience: fetch a user's public profile from user-service.
    pub async fn get_user(&self, user_id: i64) -> Result<serde_json::Value, ApiError> {
        self.get::<serde_json::Value>("user-service", &format!("/internal/users/{}", user_id))
            .await
    }

    /// Convenience: push a message to a specific user via realtime-service.
    pub async fn send_to_user(
        &self,
        user_id: i64,
        payload: &serde_json::Value,
    ) -> Result<(), ApiError> {
        self.post_fire_and_forget(
            "realtime-service",
            &format!("/internal/send/{}", user_id),
            payload,
        )
        .await
    }

    /// Convenience: broadcast a message to all connected clients.
    pub async fn broadcast(&self, payload: &serde_json::Value) -> Result<(), ApiError> {
        self.post_fire_and_forget("realtime-service", "/internal/broadcast", payload)
            .await
    }
}
