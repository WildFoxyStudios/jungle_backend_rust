use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{
    auth::AppState,
    email,
    errors::ApiError,
    events::DomainEvent,
};
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
pub struct NewsletterQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn list_subscribers(
    State(state): State<AppState>,
    Query(q): Query<NewsletterQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = (q.page.unwrap_or(1) - 1).max(0) * limit;

    let rows = sqlx::query_as::<_, (i64, String, bool, time::OffsetDateTime)>(
        "SELECT id, email, is_active, created_at FROM newsletter_subscribers ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, email, active, created_at)| {
            json!({ "id": id, "email": email, "is_active": active, "created_at": created_at.to_string() })
        })
        .collect();

    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM newsletter_subscribers")
        .fetch_one(&state.db)
        .await?;

    Ok(Json(json!({ "data": data, "meta": { "total": total } })))
}

pub async fn remove_subscriber(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM newsletter_subscribers WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

#[derive(Debug, Deserialize)]
pub struct SendNewsletterRequest {
    pub subject: String,
    pub body: String,
}

pub async fn send_newsletter(
    State(state): State<AppState>,
    Json(req): Json<SendNewsletterRequest>,
) -> Result<Json<Value>, ApiError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM newsletter_subscribers WHERE is_active = TRUE",
    )
    .fetch_one(&state.db)
    .await?;

    let body_len = req.body.len();
    tracing::info!(
        subject = %req.subject,
        body_length = body_len,
        recipients = count,
        "Newsletter queued for sending"
    );

    // Record the newsletter dispatch for audit
    sqlx::query(
        "INSERT INTO activities (user_id, action, metadata) VALUES (0, 'newsletter_sent', $1)",
    )
    .bind(serde_json::json!({
        "subject": req.subject,
        "body_preview": &req.body[..req.body.len().min(200)],
        "recipients": count,
    }))
    .execute(&state.db)
    .await
    .ok();

    // Publish newsletter event via NATS for external consumers
    let _ = state.event_bus.publish(&DomainEvent::NewsletterQueued {
        subject: req.subject.clone(),
        recipient_count: count,
    }).await;

    // Spawn background task to actually send emails in batches
    let db = state.db.clone();
    let subject = req.subject.clone();
    let body = req.body.clone();
    tokio::spawn(async move {
        if let Err(e) = send_newsletter_emails(db, &subject, &body).await {
            tracing::error!(error = %e, "Newsletter batch send failed");
        }
    });

    Ok(Json(json!({ "data": { "queued": true, "recipients": count, "body_length": body_len } })))
}

const NEWSLETTER_BATCH_SIZE: i64 = 50;
const NEWSLETTER_BATCH_DELAY_MS: u64 = 500;

async fn send_newsletter_emails(
    db: PgPool,
    subject: &str,
    html_body: &str,
) -> Result<(), String> {
    let mut offset: i64 = 0;
    let mut total_sent: i64 = 0;
    let mut total_failed: i64 = 0;

    loop {
        let emails: Vec<String> = sqlx::query_scalar(
            "SELECT email FROM newsletter_subscribers WHERE is_active = TRUE ORDER BY id LIMIT $1 OFFSET $2",
        )
        .bind(NEWSLETTER_BATCH_SIZE)
        .bind(offset)
        .fetch_all(&db)
        .await
        .map_err(|e| e.to_string())?;

        if emails.is_empty() {
            break;
        }

        for recipient in &emails {
            match email::send_email(recipient, subject, html_body).await {
                Ok(()) => total_sent += 1,
                Err(e) => {
                    tracing::warn!(to = %recipient, error = %e, "Newsletter email failed");
                    total_failed += 1;
                }
            }
        }

        offset += NEWSLETTER_BATCH_SIZE;

        // Rate-limit: small delay between batches to avoid overwhelming SMTP
        tokio::time::sleep(tokio::time::Duration::from_millis(NEWSLETTER_BATCH_DELAY_MS)).await;
    }

    tracing::info!(
        total_sent,
        total_failed,
        "Newsletter send completed"
    );

    Ok(())
}
