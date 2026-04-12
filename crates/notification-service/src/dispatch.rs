use serde::Serialize;
use shared::{email, email_templates, push};
use sqlx::PgPool;

/// Multi-channel notification dispatcher.
/// Routes notifications to in-app DB, WebSocket, Email, and Push based on user preferences.
pub struct NotificationDispatcher {
    db: PgPool,
    http: reqwest::Client,
    realtime_url: String,
    site_name: String,
    site_url: String,
}

#[derive(Debug, Serialize)]
pub struct NotificationPayload {
    pub recipient_id: i64,
    pub sender_id: Option<i64>,
    pub notification_type: String,
    pub target_type: Option<String>,
    pub target_id: Option<i64>,
    pub text: String,
}

impl NotificationDispatcher {
    pub fn new(db: PgPool) -> Self {
        let realtime_url =
            std::env::var("REALTIME_SERVICE_URL").unwrap_or_else(|_| "http://127.0.0.1:3012".into());
        Self {
            db,
            http: reqwest::Client::new(),
            realtime_url,
            site_name: std::env::var("SITE_NAME").unwrap_or_else(|_| "WoWonder".into()),
            site_url: std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:3000".into()),
        }
    }

    /// Dispatch a notification to all enabled channels for the user
    pub async fn dispatch(&self, payload: NotificationPayload) -> Result<(), String> {
        // 1. Always persist to DB (in-app)
        let notif_id = self.persist_to_db(&payload).await?;

        // 2. Check user preferences
        let prefs = self.get_user_prefs(payload.recipient_id).await;

        // 3. Push to WebSocket (realtime)
        if prefs.web_push {
            self.push_websocket(&payload, notif_id).await;
        }

        // 4. Send email if enabled
        if prefs.email {
            self.send_email_notification(&payload).await;
        }

        // 5. Mobile push (FCM/APNs) if enabled
        if prefs.mobile_push {
            self.send_mobile_push(&payload).await;
        }

        Ok(())
    }

    async fn persist_to_db(&self, p: &NotificationPayload) -> Result<i64, String> {
        sqlx::query_scalar::<_, i64>(
            r#"INSERT INTO notifications (recipient_id, sender_id, type, target_type, target_id, text)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id"#,
        )
        .bind(p.recipient_id)
        .bind(p.sender_id)
        .bind(&p.notification_type)
        .bind(&p.target_type)
        .bind(p.target_id)
        .bind(&p.text)
        .fetch_one(&self.db)
        .await
        .map_err(|e| e.to_string())
    }

    async fn push_websocket(&self, p: &NotificationPayload, notif_id: i64) {
        let ws_payload = serde_json::json!({
            "type": "notification",
            "data": {
                "id": notif_id,
                "notification_type": p.notification_type,
                "sender_id": p.sender_id,
                "text": p.text,
                "target_type": p.target_type,
                "target_id": p.target_id,
            }
        });

        let url = format!(
            "{}/internal/send/{}",
            self.realtime_url, p.recipient_id
        );
        let _ = self
            .http
            .post(&url)
            .json(&ws_payload)
            .send()
            .await;
    }

    async fn send_email_notification(&self, p: &NotificationPayload) {
        let user_email: Option<String> = sqlx::query_scalar(
            "SELECT email FROM users WHERE id = $1 AND is_active = TRUE",
        )
        .bind(p.recipient_id)
        .fetch_optional(&self.db)
        .await
        .ok()
        .flatten();

        if let Some(to_email) = user_email {
            let sender_name = if let Some(sid) = p.sender_id {
                sqlx::query_scalar::<_, String>(
                    "SELECT COALESCE(first_name || ' ' || last_name, username) FROM users WHERE id = $1",
                )
                .bind(sid)
                .fetch_optional(&self.db)
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| "Someone".into())
            } else {
                self.site_name.clone()
            };

            let (subject, html_body) = email_templates::notification_email(
                &p.notification_type,
                &sender_name,
                &p.text,
                &self.site_name,
                &self.site_url,
            );

            if let Err(e) = email::send_email(&to_email, &subject, &html_body).await {
                tracing::error!(to = %to_email, error = %e, "Failed to send email notification");
            }
        }
    }

    async fn send_mobile_push(&self, p: &NotificationPayload) {
        #[derive(sqlx::FromRow)]
        struct PushToken {
            token: String,
            platform: String,
        }

        let tokens: Vec<PushToken> = sqlx::query_as(
            "SELECT token, platform FROM push_tokens WHERE user_id = $1",
        )
        .bind(p.recipient_id)
        .fetch_all(&self.db)
        .await
        .unwrap_or_default();

        let mut data = std::collections::HashMap::new();
        data.insert("notification_type".into(), p.notification_type.clone());
        if let Some(tt) = &p.target_type {
            data.insert("target_type".into(), tt.clone());
        }
        if let Some(tid) = p.target_id {
            data.insert("target_id".into(), tid.to_string());
        }

        for pt in tokens {
            if let Err(e) = push::send_push(
                &pt.token,
                &pt.platform,
                &self.site_name,
                &p.text,
                Some(data.clone()),
            )
            .await
            {
                tracing::error!(
                    token = %pt.token,
                    platform = %pt.platform,
                    error = %e,
                    "Failed to send mobile push"
                );
            }
        }
    }

    async fn get_user_prefs(&self, user_id: i64) -> UserNotifPrefs {
        let settings: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT notification_settings FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .ok()
        .flatten();

        match settings {
            Some(v) => serde_json::from_value(v).unwrap_or_default(),
            None => UserNotifPrefs::default(),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct UserNotifPrefs {
    #[serde(default = "default_true")]
    pub web_push: bool,
    #[serde(default = "default_true")]
    pub email: bool,
    #[serde(default = "default_true")]
    pub mobile_push: bool,
}

impl Default for UserNotifPrefs {
    fn default() -> Self {
        Self {
            web_push: true,
            email: true,
            mobile_push: true,
        }
    }
}

fn default_true() -> bool {
    true
}
