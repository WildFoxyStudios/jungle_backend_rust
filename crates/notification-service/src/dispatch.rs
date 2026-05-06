use serde::Serialize;
use shared::{
    email, email_templates,
    events::{DomainEvent, EventBus},
    push,
    web_push::{DeliveryStatus, StoredSubscription, WebPushPayload, WebPushSender},
};
use sqlx::PgPool;
use std::sync::Arc;
use time::OffsetDateTime;

/// Multi-channel notification dispatcher.
/// Routes notifications to in-app DB, WebSocket (via NATS DomainEvent), Email, and Push.
pub struct NotificationDispatcher {
    db: PgPool,
    event_bus: Option<Arc<dyn EventBus>>,
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
        Self::with_event_bus(db, None)
    }

    pub fn with_event_bus(db: PgPool, event_bus: Option<Arc<dyn EventBus>>) -> Self {
        let realtime_url = std::env::var("REALTIME_SERVICE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3012".into());
        Self {
            db,
            event_bus,
            http: reqwest::Client::new(),
            realtime_url,
            site_name: std::env::var("SITE_NAME").unwrap_or_else(|_| "Jungle".into()),
            site_url: std::env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
        }
    }

    /// Dispatch a notification to all enabled channels for the user
    pub async fn dispatch(&self, payload: NotificationPayload) -> Result<(), String> {
        // 1. Always persist to DB (in-app)
        let notif_id = self.persist_to_db(&payload).await?;

        // 2. Publish a domain event so realtime-service (and any other consumer)
        //    can relay this to WebSocket clients without a direct HTTP call.
        if let Some(bus) = &self.event_bus {
            let event = DomainEvent::NotificationCreated {
                recipient_id: payload.recipient_id,
                notification_type: payload.notification_type.clone(),
                sender_id: payload.sender_id,
            };
            if let Err(e) = bus.publish(&event).await {
                tracing::warn!(error = %e, "failed to publish NotificationCreated");
            }
        }

        // 3. Check user preferences
        let prefs = self.get_user_prefs(payload.recipient_id).await;

        // 4. HTTP fallback to realtime-service (used when NATS is unavailable).
        //    Safe to coexist with step 2 because realtime-service deduplicates
        //    by (user_id, notif_id) downstream.
        if prefs.web_push {
            self.push_websocket(&payload, notif_id).await;
        }

        // 5. Send email if enabled
        if prefs.email {
            self.send_email_notification(&payload).await;
        }

        // 6. Mobile push (FCM/APNs) if enabled (kept for transitional parity;
        //    web-only stack will eventually switch to VAPID Web Push).
        if prefs.mobile_push {
            self.send_mobile_push(&payload).await;
        }

        // 7. VAPID Web Push for desktop browsers / installed PWAs.
        if prefs.web_push {
            self.send_web_push(&payload, notif_id).await;
        }

        Ok(())
    }

    async fn send_web_push(&self, p: &NotificationPayload, notif_id: i64) {
        let sender = match WebPushSender::from_db_or_env(&self.db).await {
            Some(s) => s,
            None => return,
        };

        let subs: Vec<StoredSubscription> = sqlx::query_as(
            "SELECT id, endpoint, p256dh, auth FROM push_subscriptions WHERE user_id = $1",
        )
        .bind(p.recipient_id)
        .fetch_all(&self.db)
        .await
        .unwrap_or_default();

        if subs.is_empty() {
            return;
        }

        let url = match (&p.target_type, p.target_id) {
            (Some(t), Some(id)) if t == "post" => format!("{}/post/{}", self.site_url, id),
            (Some(t), Some(id)) if t == "user" => format!("{}/user/{}", self.site_url, id),
            (Some(t), Some(id)) if t == "message" || t == "conversation" => {
                format!("{}/messages/{}", self.site_url, id)
            }
            _ => format!("{}/notifications", self.site_url),
        };
        let tag = format!("notif-{notif_id}");
        let payload = WebPushPayload {
            title: &self.site_name,
            body: &p.text,
            icon: None,
            url: Some(&url),
            tag: Some(&tag),
        };

        for sub in subs {
            match sender.send(&sub, &payload).await {
                Ok(DeliveryStatus::Delivered) => {}
                Ok(DeliveryStatus::Gone) => {
                    if let Err(e) = sqlx::query("DELETE FROM push_subscriptions WHERE id = $1")
                        .bind(sub.id)
                        .execute(&self.db)
                        .await
                    {
                        tracing::warn!(id = sub.id, error = %e, "failed to drop stale push sub");
                    }
                }
                Err(e) => tracing::error!(
                    sub_id = sub.id,
                    error = %e,
                    "Web Push delivery failed"
                ),
            }
        }
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

        let url = format!("{}/internal/send/{}", self.realtime_url, p.recipient_id);
        let _ = self.http.post(&url).json(&ws_payload).send().await;
    }

    async fn send_email_notification(&self, p: &NotificationPayload) {
        let user_email: Option<String> =
            sqlx::query_scalar("SELECT email FROM users WHERE id = $1 AND is_active = TRUE")
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

        let tokens: Vec<PushToken> =
            sqlx::query_as("SELECT token, platform FROM push_tokens WHERE user_id = $1")
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

    /// In-app (+ push/email) notifications for new chat messages, one row per recipient.
    /// Skips senders, members with conversation mute, and logs per-recipient dispatch errors.
    pub async fn dispatch_new_chat_messages(
        &self,
        conversation_id: i64,
        sender_id: i64,
        recipient_ids: &[i64],
    ) {
        let preview: Option<String> = sqlx::query_scalar::<_, String>(
            r#"
            SELECT COALESCE(content, '')
            FROM messages
            WHERE conversation_id = $1 AND sender_id = $2 AND deleted_at IS NULL
            ORDER BY id DESC
            LIMIT 1
            "#,
        )
        .bind(conversation_id)
        .bind(sender_id)
        .fetch_optional(&self.db)
        .await
        .unwrap_or(None);

        let snippet: String = preview
            .unwrap_or_default()
            .chars()
            .take(160)
            .collect();
        let text = if snippet.trim().is_empty() {
            "Sent you a message".to_string()
        } else {
            snippet
        };

        for &rid in recipient_ids {
            if rid == sender_id {
                continue;
            }
            if self
                .is_conversation_muted_for_user(conversation_id, rid)
                .await
            {
                continue;
            }
            let payload = NotificationPayload {
                recipient_id: rid,
                sender_id: Some(sender_id),
                notification_type: "new_message".to_string(),
                target_type: Some("conversation".to_string()),
                target_id: Some(conversation_id),
                text: text.clone(),
            };
            if let Err(e) = self.dispatch(payload).await {
                tracing::error!(
                    recipient_id = rid,
                    conversation_id,
                    error = %e,
                    "dispatch new_message failed"
                );
            }
        }
    }

    async fn is_conversation_muted_for_user(&self, conversation_id: i64, user_id: i64) -> bool {
        #[derive(sqlx::FromRow)]
        struct MuteRow {
            muted: bool,
            muted_until: Option<OffsetDateTime>,
        }
        let row = sqlx::query_as::<_, MuteRow>(
            r#"
            SELECT muted, muted_until
            FROM conversation_members
            WHERE conversation_id = $1 AND user_id = $2 AND is_active = TRUE
            "#,
        )
        .bind(conversation_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .ok()
        .flatten();

        let Some(row) = row else {
            return false;
        };
        let now = OffsetDateTime::now_utc();
        row.muted || row.muted_until.is_some_and(|t| t > now)
    }

    async fn get_user_prefs(&self, user_id: i64) -> UserNotifPrefs {
        let settings: Option<serde_json::Value> =
            sqlx::query_scalar("SELECT notification_settings FROM users WHERE id = $1")
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
            email: false,
            mobile_push: false,
        }
    }
}

fn default_true() -> bool {
    true
}
