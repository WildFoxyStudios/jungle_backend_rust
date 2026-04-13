use once_cell::sync::OnceCell;
use serde::Serialize;
use std::sync::Arc;

static PUSH_SENDER: OnceCell<Arc<PushSender>> = OnceCell::new();

pub struct PushSender {
    client: reqwest::Client,
    fcm_config: Option<FcmConfig>,
    apns_config: Option<ApnsConfig>,
}

struct FcmConfig {
    project_id: String,
    service_account_json: String,
}

struct ApnsConfig {
    key_id: String,
    team_id: String,
    private_key_pem: String,
    topic: String,
    production: bool,
}

#[derive(Debug, Serialize)]
struct FcmMessage {
    message: FcmMessageBody,
}

#[derive(Debug, Serialize)]
struct FcmMessageBody {
    token: String,
    notification: FcmNotification,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
struct FcmNotification {
    title: String,
    body: String,
}

impl PushSender {
    pub fn from_env() -> Self {
        let fcm_config = match (
            std::env::var("FCM_PROJECT_ID").ok(),
            std::env::var("FCM_SERVICE_ACCOUNT_JSON").ok(),
        ) {
            (Some(pid), Some(sa)) if !pid.is_empty() && !sa.is_empty() => Some(FcmConfig {
                project_id: pid,
                service_account_json: sa,
            }),
            _ => None,
        };

        let apns_config = match (
            std::env::var("APNS_KEY_ID").ok(),
            std::env::var("APNS_TEAM_ID").ok(),
            std::env::var("APNS_PRIVATE_KEY_PATH").ok(),
        ) {
            (Some(kid), Some(tid), Some(pk_path)) if !kid.is_empty() => {
                let pem = std::fs::read_to_string(&pk_path).unwrap_or_default();
                if pem.is_empty() {
                    tracing::warn!("APNs private key file not found at {}", pk_path);
                    None
                } else {
                    Some(ApnsConfig {
                        key_id: kid,
                        team_id: tid,
                        private_key_pem: pem,
                        topic: std::env::var("APNS_TOPIC")
                            .unwrap_or_else(|_| "com.example.Jungle".into()),
                        production: std::env::var("APNS_PRODUCTION")
                            .unwrap_or_else(|_| "false".into())
                            == "true",
                    })
                }
            }
            _ => None,
        };

        Self {
            client: reqwest::Client::new(),
            fcm_config,
            apns_config,
        }
    }

    pub fn init_global() {
        let sender = Self::from_env();
        if sender.fcm_config.is_some() {
            tracing::info!("FCM push configured");
        }
        if sender.apns_config.is_some() {
            tracing::info!("APNs push configured");
        }
        let _ = PUSH_SENDER.set(Arc::new(sender));
    }

    pub fn global() -> Option<&'static Arc<PushSender>> {
        PUSH_SENDER.get()
    }

    pub async fn send_fcm(
        &self,
        token: &str,
        title: &str,
        body: &str,
        data: Option<std::collections::HashMap<String, String>>,
    ) -> Result<(), String> {
        let config = self
            .fcm_config
            .as_ref()
            .ok_or_else(|| "FCM not configured".to_string())?;

        let access_token = self.get_fcm_access_token(config).await?;

        let url = format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            config.project_id
        );

        let payload = FcmMessage {
            message: FcmMessageBody {
                token: token.to_string(),
                notification: FcmNotification {
                    title: title.to_string(),
                    body: body.to_string(),
                },
                data,
            },
        };

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&access_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("FCM request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("FCM error {}: {}", status, text));
        }

        Ok(())
    }

    pub async fn send_apns(
        &self,
        device_token: &str,
        title: &str,
        body: &str,
        data: Option<serde_json::Value>,
    ) -> Result<(), String> {
        let config = self
            .apns_config
            .as_ref()
            .ok_or_else(|| "APNs not configured".to_string())?;

        let base = if config.production {
            "https://api.push.apple.com"
        } else {
            "https://api.sandbox.push.apple.com"
        };
        let url = format!("{}/3/device/{}", base, device_token);

        let jwt = self.create_apns_jwt(config)?;

        let mut payload = serde_json::json!({
            "aps": {
                "alert": {
                    "title": title,
                    "body": body,
                },
                "sound": "default",
                "badge": 1,
            }
        });

        if let Some(extra) = data
            && let Some(obj) = payload.as_object_mut() {
                obj.insert("data".to_string(), extra);
            }

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&jwt)
            .header("apns-topic", &config.topic)
            .header("apns-push-type", "alert")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("APNs request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("APNs error {}: {}", status, text));
        }

        Ok(())
    }

    async fn get_fcm_access_token(&self, config: &FcmConfig) -> Result<String, String> {
        let sa: serde_json::Value = serde_json::from_str(&config.service_account_json)
            .map_err(|e| format!("Invalid service account JSON: {}", e))?;

        let client_email = sa["client_email"]
            .as_str()
            .ok_or("Missing client_email in service account")?;
        let private_key = sa["private_key"]
            .as_str()
            .ok_or("Missing private_key in service account")?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let header = serde_json::json!({"alg": "RS256", "typ": "JWT"});
        let claims = serde_json::json!({
            "iss": client_email,
            "scope": "https://www.googleapis.com/auth/firebase.messaging",
            "aud": "https://oauth2.googleapis.com/token",
            "iat": now,
            "exp": now + 3600,
        });

        let header_b64 = base64_url_encode(&serde_json::to_vec(&header).unwrap());
        let claims_b64 = base64_url_encode(&serde_json::to_vec(&claims).unwrap());
        let signing_input = format!("{}.{}", header_b64, claims_b64);

        let key = jsonwebtoken::EncodingKey::from_rsa_pem(private_key.as_bytes())
            .map_err(|e| format!("Invalid RSA key: {}", e))?;

        let jwt_header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        let token = jsonwebtoken::encode(
            &jwt_header,
            &claims,
            &key,
        )
        .map_err(|e| format!("JWT sign error: {}", e))?;

        let _ = signing_input;

        let resp = self
            .client
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &token),
            ])
            .send()
            .await
            .map_err(|e| format!("Token exchange failed: {}", e))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Token parse error: {}", e))?;

        body["access_token"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("No access_token in response: {}", body))
    }

    fn create_apns_jwt(&self, config: &ApnsConfig) -> Result<String, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::ES256);
        header.kid = Some(config.key_id.clone());

        let claims = serde_json::json!({
            "iss": config.team_id,
            "iat": now,
        });

        let key = jsonwebtoken::EncodingKey::from_ec_pem(config.private_key_pem.as_bytes())
            .map_err(|e| format!("Invalid APNs key: {}", e))?;

        jsonwebtoken::encode(&header, &claims, &key)
            .map_err(|e| format!("APNs JWT error: {}", e))
    }
}

fn base64_url_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

pub async fn send_push(
    token: &str,
    platform: &str,
    title: &str,
    body: &str,
    data: Option<std::collections::HashMap<String, String>>,
) -> Result<(), String> {
    let sender = match PushSender::global() {
        Some(s) => s,
        None => {
            tracing::warn!("Push not sent — PushSender not initialized");
            return Ok(());
        }
    };

    match platform {
        "fcm" | "android" | "web" => sender.send_fcm(token, title, body, data).await,
        "apns" | "ios" => {
            let json_data = data.map(|d| serde_json::to_value(d).unwrap_or_default());
            sender.send_apns(token, title, body, json_data).await
        }
        _ => {
            tracing::warn!(platform, "Unknown push platform");
            Ok(())
        }
    }
}
