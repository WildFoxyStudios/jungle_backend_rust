use once_cell::sync::OnceCell;
use std::sync::Arc;

static SMS_SENDER: OnceCell<Arc<SmsSender>> = OnceCell::new();

#[derive(Debug, Clone)]
pub enum SmsProvider {
    Twilio {
        account_sid: String,
        auth_token: String,
        from_phone: String,
    },
    Infobip {
        api_key: String,
        base_url: String,
        from_sender: String,
    },
    Msg91 {
        auth_key: String,
        sender_id: String,
    },
}

pub struct SmsSender {
    provider: SmsProvider,
    client: reqwest::Client,
}

impl SmsSender {
    pub fn from_env() -> Option<Self> {
        let provider_name = std::env::var("SMS_PROVIDER").unwrap_or_default();
        let provider = match provider_name.as_str() {
            "twilio" => {
                let sid = std::env::var("TWILIO_ACCOUNT_SID").ok()?;
                let token = std::env::var("TWILIO_AUTH_TOKEN").ok()?;
                let from = std::env::var("TWILIO_PHONE_FROM").ok()?;
                if sid.is_empty() || token.is_empty() {
                    return None;
                }
                SmsProvider::Twilio {
                    account_sid: sid,
                    auth_token: token,
                    from_phone: from,
                }
            }
            "infobip" => {
                let key = std::env::var("INFOBIP_API_KEY").ok()?;
                let url = std::env::var("INFOBIP_BASE_URL")
                    .unwrap_or_else(|_| "https://api.infobip.com".into());
                let from = std::env::var("INFOBIP_SENDER").unwrap_or_else(|_| "WoWonder".into());
                SmsProvider::Infobip {
                    api_key: key,
                    base_url: url,
                    from_sender: from,
                }
            }
            "msg91" => {
                let key = std::env::var("MSG91_AUTH_KEY").ok()?;
                let sender = std::env::var("MSG91_SENDER_ID").unwrap_or_else(|_| "WOWNDR".into());
                SmsProvider::Msg91 {
                    auth_key: key,
                    sender_id: sender,
                }
            }
            _ => return None,
        };

        Some(Self {
            provider,
            client: reqwest::Client::new(),
        })
    }

    pub fn init_global() -> Result<(), String> {
        match Self::from_env() {
            Some(sender) => SMS_SENDER
                .set(Arc::new(sender))
                .map_err(|_| "SmsSender already initialized".to_string()),
            None => {
                tracing::warn!("SMS provider not configured — SMS sending disabled");
                Ok(())
            }
        }
    }

    pub fn global() -> Option<&'static Arc<SmsSender>> {
        SMS_SENDER.get()
    }

    pub async fn send(&self, to: &str, body: &str) -> Result<(), String> {
        match &self.provider {
            SmsProvider::Twilio {
                account_sid,
                auth_token,
                from_phone,
            } => {
                let url = format!(
                    "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
                    account_sid
                );
                let resp = self
                    .client
                    .post(&url)
                    .basic_auth(account_sid, Some(auth_token))
                    .form(&[("To", to), ("From", from_phone.as_str()), ("Body", body)])
                    .send()
                    .await
                    .map_err(|e| format!("Twilio request failed: {}", e))?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    return Err(format!("Twilio error {}: {}", status, text));
                }

                tracing::info!(to, "SMS sent via Twilio");
                Ok(())
            }
            SmsProvider::Infobip {
                api_key,
                base_url,
                from_sender,
            } => {
                let url = format!("{}/sms/2/text/advanced", base_url);
                let payload = serde_json::json!({
                    "messages": [{
                        "from": from_sender,
                        "destinations": [{"to": to}],
                        "text": body,
                    }]
                });
                let resp = self
                    .client
                    .post(&url)
                    .header("Authorization", format!("App {}", api_key))
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| format!("Infobip request failed: {}", e))?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    return Err(format!("Infobip error {}: {}", status, text));
                }

                tracing::info!(to, "SMS sent via Infobip");
                Ok(())
            }
            SmsProvider::Msg91 {
                auth_key,
                sender_id,
            } => {
                let url = "https://api.msg91.com/api/v5/flow/";
                let payload = serde_json::json!({
                    "sender": sender_id,
                    "route": "4",
                    "country": "0",
                    "sms": [{"message": body, "to": [to]}],
                });
                let resp = self
                    .client
                    .post(url)
                    .header("authkey", auth_key.as_str())
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| format!("MSG91 request failed: {}", e))?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    return Err(format!("MSG91 error {}: {}", status, text));
                }

                tracing::info!(to, "SMS sent via MSG91");
                Ok(())
            }
        }
    }
}

pub async fn send_sms(to: &str, body: &str) -> Result<(), String> {
    match SmsSender::global() {
        Some(sender) => sender.send(to, body).await,
        None => {
            tracing::warn!(to, "SMS not sent — no SMS provider configured");
            Ok(())
        }
    }
}
