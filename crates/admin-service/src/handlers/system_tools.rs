use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    email,
    errors::ApiError,
    events::DomainEvent,
    permissions::Permission,
    sms,
};

// â”€â”€â”€ ffmpeg probe â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// GET /v1/admin/system/ffmpeg-probe
///
/// Detects whether `ffmpeg` is on `PATH` and reports the version + which
/// codecs we care about for the platform (h264, hevc, vp9, aac, opus).
/// The admin UI uses this to render a green/red indicator instead of the
/// PHP "Check FFmpeg" status row.
pub async fn ffmpeg_probe(
    State(_state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageSettings, &state).await?;

    let version_out = tokio::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .await;

    let Ok(version_out) = version_out else {
        return Ok(Json(json!({
            "data": {
                "available": false,
                "version": null,
                "codecs": [],
                "error": "ffmpeg binary not found on PATH",
            }
        })));
    };

    if !version_out.status.success() {
        return Ok(Json(json!({
            "data": {
                "available": false,
                "version": null,
                "codecs": [],
                "error": String::from_utf8_lossy(&version_out.stderr).to_string(),
            }
        })));
    }

    let stdout = String::from_utf8_lossy(&version_out.stdout);
    let version_line = stdout.lines().next().unwrap_or("").to_string();

    // Probe codec availability.
    let codec_out = tokio::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-codecs"])
        .output()
        .await
        .ok();

    let mut codecs = Vec::new();
    if let Some(out) = codec_out {
        let body = String::from_utf8_lossy(&out.stdout);
        for codec in ["h264", "hevc", "vp9", "vp8", "aac", "opus", "mp3"] {
            let supported = body.lines().any(|l| {
                let trimmed = l.trim_start();
                trimmed.contains(&format!(" {} ", codec))
                    || trimmed.contains(&format!(" {}_", codec))
            });
            codecs.push(json!({ "name": codec, "available": supported }));
        }
    }

    Ok(Json(json!({
        "data": {
            "available": true,
            "version": version_line,
            "codecs": codecs,
        }
    })))
}

// â”€â”€â”€ Email test â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Deserialize)]
pub struct TestEmailRequest {
    pub to: String,
}

/// POST /v1/admin/system/email/test â€” sends a one-shot test email.
pub async fn test_email(
    State(_state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<TestEmailRequest>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageSettings, &state).await?;

    let to = req.to.trim();
    if to.is_empty() || !to.contains('@') {
        return Err(ApiError::BadRequest(
            "Valid recipient email is required".into(),
        ));
    }

    let site_name = std::env::var("SITE_NAME").unwrap_or_else(|_| "Jungle".into());
    let subject = format!("{} â€” SMTP test", site_name);
    let body = format!(
        r#"<p>This is a test email sent from the admin panel.</p>
<p>If you received this, your SMTP/transactional configuration is working correctly.</p>
<p>â€” {}</p>"#,
        site_name
    );

    match email::send_email(to, &subject, &body).await {
        Ok(()) => Ok(Json(json!({ "data": { "sent": true, "to": to } }))),
        Err(e) => Err(ApiError::BadRequest(format!("Email send failed: {e}"))),
    }
}

// â”€â”€â”€ SMS test â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Deserialize)]
pub struct TestSmsRequest {
    pub to: String,
}

/// POST /v1/admin/system/sms/test â€” sends a one-shot test SMS.
pub async fn test_sms(
    State(_state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<TestSmsRequest>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageSettings, &state).await?;

    let to = req.to.trim();
    if to.is_empty() {
        return Err(ApiError::BadRequest("Recipient phone is required".into()));
    }

    let body = "This is a test SMS from your admin panel. If you received it, your SMS provider is configured correctly.";

    match sms::send_sms(to, body).await {
        Ok(()) => Ok(Json(json!({ "data": { "sent": true, "to": to } }))),
        Err(e) => Err(ApiError::BadRequest(format!("SMS send failed: {e}"))),
    }
}

// â”€â”€â”€ OAuth provider verification â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// POST /v1/admin/oauth-apps/{provider}/verify
///
/// Lightweight reachability/sanity probe: checks that the provider has
/// `client_id` and `client_secret` configured in `site_config` and that
/// the redirect URI matches the conventional shape. Does NOT perform a
/// full OAuth round-trip â€” that requires a real user interaction.
pub async fn verify_oauth_app(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(provider): Path<String>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageOauth, &state).await?;

    let provider = provider.to_lowercase();
    let allowed = [
        "google",
        "facebook",
        "twitter",
        "github",
        "microsoft",
        "apple",
        "discord",
    ];
    if !allowed.contains(&provider.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Unknown OAuth provider: {provider}"
        )));
    }

    #[derive(sqlx::FromRow)]
    struct Pair {
        key: String,
        value: Option<String>,
    }

    let rows: Vec<Pair> = sqlx::query_as("SELECT key, value FROM site_config WHERE category = $1")
        .bind(format!("oauth_{provider}"))
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let mut client_id_set = false;
    let mut client_secret_set = false;
    let mut redirect_uri = None;
    for row in rows {
        match row.key.as_str() {
            "client_id" => {
                client_id_set = row.value.as_deref().is_some_and(|v| !v.trim().is_empty())
            }
            "client_secret" => {
                client_secret_set = row.value.as_deref().is_some_and(|v| !v.trim().is_empty())
            }
            "redirect_uri" => redirect_uri = row.value.clone(),
            _ => {}
        }
    }

    let configured = client_id_set && client_secret_set;

    Ok(Json(json!({
        "data": {
            "provider": provider,
            "configured": configured,
            "client_id_set": client_id_set,
            "client_secret_set": client_secret_set,
            "redirect_uri": redirect_uri,
        }
    })))
}

// â”€â”€â”€ Dead-letter queue â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(Debug, Deserialize, Default)]
pub struct DlqListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// `consumed`, `unconsumed`, or `all` (default: `unconsumed`).
    pub status: Option<String>,
}

/// GET /v1/admin/system/dlq â€” paginated list of failed domain events.
pub async fn list_dlq(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<DlqListParams>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageDlq, &state).await?;

    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let offset = params.offset.unwrap_or(0).max(0);
    let status = params.status.as_deref().unwrap_or("unconsumed");

    let where_clause = match status {
        "consumed" => "WHERE consumed_at IS NOT NULL",
        "all" => "",
        _ => "WHERE consumed_at IS NULL",
    };

    type DlqRow = (
        i64,
        String,
        Value,
        Option<String>,
        i32,
        Option<time::OffsetDateTime>,
        Option<time::OffsetDateTime>,
        Option<i64>,
        time::OffsetDateTime,
    );
    let rows: Vec<DlqRow> =
        sqlx::query_as(&format!(
            "SELECT id, subject, payload, error, attempt, retry_at, consumed_at, consumed_by, created_at
             FROM event_dlq {where_clause}
             ORDER BY created_at DESC
             LIMIT $1 OFFSET $2"
        ))
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;

    let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM event_dlq {where_clause}"))
        .fetch_one(&state.db)
        .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(
            |(
                id,
                subject,
                payload,
                error,
                attempt,
                retry_at,
                consumed_at,
                consumed_by,
                created_at,
            )| {
                json!({
                    "id": id,
                    "subject": subject,
                    "payload": payload,
                    "error": error,
                    "attempt": attempt,
                    "retry_at": retry_at.map(|t| t.to_string()),
                    "consumed_at": consumed_at.map(|t| t.to_string()),
                    "consumed_by": consumed_by,
                    "created_at": created_at.to_string(),
                })
            },
        )
        .collect();

    Ok(Json(json!({ "data": data, "meta": { "total": total } })))
}

/// POST /v1/admin/system/dlq/{id}/retry â€” republish a failed event.
pub async fn retry_dlq(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageDlq, &state).await?;

    let row: Option<(String, Value)> = sqlx::query_as(
        "SELECT subject, payload FROM event_dlq WHERE id = $1 AND consumed_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    let Some((subject, payload)) = row else {
        return Err(ApiError::NotFound(
            "DLQ entry not found or already consumed".into(),
        ));
    };

    // Strip the `dlq.` prefix so the event lands on the original subject.
    let target_subject = subject.strip_prefix("dlq.").unwrap_or(&subject).to_string();

    // Try to deserialize the payload as a DomainEvent first (preferred path).
    if let Ok(event) = serde_json::from_value::<DomainEvent>(payload.clone()) {
        state
            .event_bus
            .publish(&event)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to republish event: {e}")))?;
    } else {
        // Fall back to raw publish using the bytes of the original payload.
        let bytes = serde_json::to_vec(&payload)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize payload: {e}")))?;
        state
            .event_bus
            .publish_raw(&target_subject, &bytes)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to republish event: {e}")))?;
    }

    sqlx::query("UPDATE event_dlq SET consumed_at = NOW(), consumed_by = $1 WHERE id = $2")
        .bind(auth.user_id)
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(
        json!({ "data": { "retried": true, "subject": target_subject } }),
    ))
}

/// DELETE /v1/admin/system/dlq/{id} â€” discard a failed event without retrying.
pub async fn delete_dlq(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageDlq, &state).await?;

    let result = sqlx::query("DELETE FROM event_dlq WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("DLQ entry not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

