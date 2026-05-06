//! Admin endpoints for live-stream moderation and bulk email campaigns.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    permissions::Permission,
};
use sqlx::FromRow;
use time::OffsetDateTime;
use validator::Validate;

// ═══════════════════════════════════════════════════════════════════
// Live streams
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, FromRow)]
pub struct LiveStreamRow {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub avatar: Option<String>,
    pub title: Option<String>,
    pub viewer_count: i64,
    pub started_at: OffsetDateTime,
    pub duration_seconds: Option<i64>,
}

/// GET /v1/admin/live-streams — currently active broadcasts
pub async fn list_live_streams(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManagePosts, &state).await?;
    let rows = sqlx::query_as::<_, LiveStreamRow>(
        r#"
        SELECT ls.id, ls.user_id, u.username, u.first_name, u.last_name, u.avatar,
               ls.title,
               COALESCE(
                   (SELECT COUNT(*) FROM live_viewers v WHERE v.stream_id = ls.id AND v.is_watching = TRUE),
                   ls.viewer_count::bigint
               ) AS viewer_count,
               ls.created_at AS started_at,
               EXTRACT(EPOCH FROM (NOW() - ls.created_at))::bigint AS duration_seconds
          FROM live_streams ls
          JOIN users u ON u.id = ls.user_id
         WHERE ls.ended_at IS NULL
           AND ls.status = 'live'
         ORDER BY ls.created_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": rows })))
}

/// GET /v1/admin/live/stats — aggregate live-streaming metrics
///
/// Returns: number of currently-live streams, peak concurrent viewers across
/// all active streams, total streams started in the last 24h, and the top
/// streamers by viewer-hours over the last 7 days.
pub async fn live_stats(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ViewDashboard, &state).await?;
    // Currently-live count + viewers sum
    let (active_streams, active_viewers): (i64, i64) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::bigint,
               COALESCE(SUM(viewer_count), 0)::bigint
          FROM live_streams
         WHERE ended_at IS NULL AND status = 'live'
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    // 24h totals
    let started_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM live_streams WHERE created_at > NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(&state.db)
    .await?;

    let ended_24h: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM live_streams WHERE ended_at > NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(&state.db)
    .await?;

    // Top streamers 7d by total seconds broadcast
    let top_streamers = sqlx::query_as::<_, (i64, String, i64, i64)>(
        r#"
        SELECT ls.user_id,
               u.username,
               COUNT(ls.id)::bigint AS stream_count,
               COALESCE(SUM(
                 EXTRACT(EPOCH FROM (COALESCE(ls.ended_at, NOW()) - ls.created_at))
               )::bigint, 0) AS total_seconds
          FROM live_streams ls
          JOIN users u ON u.id = ls.user_id
         WHERE ls.created_at > NOW() - INTERVAL '7 days'
         GROUP BY ls.user_id, u.username
         ORDER BY total_seconds DESC
         LIMIT 10
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    let top = top_streamers
        .into_iter()
        .map(|(uid, uname, count, secs)| {
            json!({
                "user_id": uid,
                "username": uname,
                "stream_count": count,
                "total_seconds": secs,
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(json!({
        "data": {
            "active": {
                "streams": active_streams,
                "viewers": active_viewers,
            },
            "last_24h": {
                "started": started_24h,
                "ended": ended_24h,
            },
            "top_streamers_7d": top,
        }
    })))
}

/// DELETE /v1/admin/live-streams/{id} — force-end an active stream
pub async fn force_end_live_stream(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManagePosts, &state).await?;
    let updated = sqlx::query(
        "UPDATE live_streams SET ended_at = NOW(), status = 'ended' WHERE id = $1 AND ended_at IS NULL",
    )
    .bind(id)
    .execute(&state.db)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(ApiError::NotFound("Stream not found or already ended".into()));
    }

    Ok(Json(json!({ "data": { "ended": true } })))
}

// ═══════════════════════════════════════════════════════════════════
// Email campaigns (bulk)
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, Validate)]
pub struct EmailCampaignRequest {
    #[validate(length(min = 1, max = 300))]
    pub subject: String,
    #[validate(length(min = 1))]
    pub body: String,
    /// "all" | "pro" | "verified" | "recent" | "specific"
    pub audience: String,
    /// When audience = "specific", list of usernames to send to
    pub usernames: Option<Vec<String>>,
    /// When audience = "recent", registration cutoff date (YYYY-MM-DD)
    pub registered_after: Option<String>,
}

/// POST /v1/admin/email-campaigns
///
/// Queues emails by inserting rows into `newsletter_queue`. The
/// `newsletter_dispatcher` job (in `jobs-runner`) will batch-send them with
/// placeholder substitution and respect `email_rate_limit` from site_config.
pub async fn create_email_campaign(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<EmailCampaignRequest>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::SendNewsletter, &state).await?;
    req.validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let (user_filter_sql, user_filter_bind) = match req.audience.as_str() {
        "all" => ("WHERE email IS NOT NULL AND email_verified = TRUE AND is_active = TRUE", None),
        "pro" => ("WHERE email IS NOT NULL AND is_pro > 0 AND is_active = TRUE", None),
        "verified" => ("WHERE email IS NOT NULL AND is_verified = TRUE AND is_active = TRUE", None),
        "recent" => {
            let after = req
                .registered_after
                .as_deref()
                .ok_or_else(|| ApiError::BadRequest("registered_after required for audience='recent'".into()))?;
            ("WHERE email IS NOT NULL AND is_active = TRUE AND created_at >= $1::date", Some(after.to_string()))
        }
        "specific" => {
            let list = req
                .usernames
                .as_ref()
                .ok_or_else(|| ApiError::BadRequest("usernames required for audience='specific'".into()))?;
            if list.is_empty() {
                return Err(ApiError::BadRequest("usernames list empty".into()));
            }

            // Direct path for specific list — queue one row per username
            let mut queued = 0i64;
            for uname in list.iter().take(10_000) {
                let clean = uname.trim().trim_start_matches('@');
                if clean.is_empty() {
                    continue;
                }
                let res = sqlx::query(
                    r#"INSERT INTO newsletter_queue
                         (recipient_user_id, recipient_email, subject, body, status, created_at)
                       SELECT id, email, $2, $3, 'pending', NOW()
                         FROM users
                        WHERE LOWER(username) = LOWER($1)
                          AND email IS NOT NULL
                          AND is_active = TRUE"#,
                )
                .bind(clean)
                .bind(&req.subject)
                .bind(&req.body)
                .execute(&state.db)
                .await?;
                queued += res.rows_affected() as i64;
            }
            return Ok(Json(json!({ "data": { "queued": queued } })));
        }
        other => {
            return Err(ApiError::BadRequest(format!(
                "audience must be one of all|pro|verified|recent|specific (got {other:?})",
            )));
        }
    };

    let insert_sql = format!(
        r#"INSERT INTO newsletter_queue
             (recipient_user_id, recipient_email, subject, body, status, created_at)
           SELECT id, email, $1, $2, 'pending', NOW()
             FROM users
             {user_filter_sql}"#
    );

    let query = sqlx::query(&insert_sql)
        .bind(&req.subject)
        .bind(&req.body);

    let res = if let Some(bind) = user_filter_bind {
        query.bind(bind).execute(&state.db).await?
    } else {
        query.execute(&state.db).await?
    };

    Ok(Json(json!({ "data": { "queued": res.rows_affected() } })))
}

// ═══════════════════════════════════════════════════════════════════
// Cronjobs status (best-effort — reads from cronjob_runs table)
// ═══════════════════════════════════════════════════════════════════

/// GET /v1/admin/changelog
///
/// Returns the list of database migrations applied to the backend, ordered
/// newest-first. This is the canonical source of truth for "what has been
/// deployed" because every feature lands with a migration. No invented data.
pub async fn list_changelog(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ViewDashboard, &state).await?;
    #[derive(Debug, Serialize, FromRow)]
    struct MigrationRow {
        version: i64,
        description: String,
        installed_on: OffsetDateTime,
        success: bool,
        execution_time: i64,
    }

    // sqlx creates this table itself; rely on it as source of truth. Returns
    // empty list if migrations have never been run on this DB.
    let rows = sqlx::query_as::<_, MigrationRow>(
        r#"SELECT version, description, installed_on, success, execution_time
             FROM _sqlx_migrations
            ORDER BY installed_on DESC"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let items: Vec<Value> = rows
        .into_iter()
        .map(|m| {
            json!({
                "version": m.version.to_string(),
                "description": m.description,
                "installed_on": m.installed_on.to_string(),
                "success": m.success,
                "execution_time_ms": m.execution_time / 1_000_000, // sqlx stores ns
            })
        })
        .collect();

    Ok(Json(json!({
        "data": {
            "backend_version": env!("CARGO_PKG_VERSION"),
            "migrations": items,
        }
    })))
}

/// GET /v1/admin/cronjobs/status
///
/// Returns `{ job_name: { last_run, status } }` for every job that has been
/// recorded in `cronjob_runs`. Returns an empty object if the table does not
/// exist (graceful degradation).
pub async fn cronjobs_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageCronjobs, &state).await?;
    let rows = sqlx::query_as::<_, (String, OffsetDateTime, String)>(
        r#"
        SELECT DISTINCT ON (name) name, ran_at, status
          FROM cronjob_runs
         ORDER BY name, ran_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut out = serde_json::Map::new();
    for (name, ran_at, status) in rows {
        out.insert(
            name,
            json!({ "last_run": ran_at.to_string(), "status": status }),
        );
    }

    Ok(Json(json!(out)))
}

// ═══════════════════════════════════════════════════════════════════
// Cronjob config (enable/disable + schedule + last run per job)
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, FromRow)]
pub struct CronjobConfigRow {
    pub job_name: String,
    pub schedule: String,
    pub enabled: bool,
    pub last_run_at: Option<OffsetDateTime>,
    pub last_status: Option<String>,
    pub description: Option<String>,
    pub updated_at: OffsetDateTime,
}

/// GET /v1/admin/cronjob-config — full catalog + enabled flag + last run
pub async fn list_cronjob_config(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageCronjobs, &state).await?;
    let rows = sqlx::query_as::<_, CronjobConfigRow>(
        r#"
        SELECT c.job_name, c.schedule, c.enabled,
               r.ran_at  AS last_run_at,
               r.status  AS last_status,
               c.description,
               c.updated_at
          FROM cronjob_config c
          LEFT JOIN LATERAL (
              SELECT ran_at, status
                FROM cronjob_runs
               WHERE name = c.job_name
            ORDER BY ran_at DESC
               LIMIT 1
          ) r ON TRUE
         ORDER BY c.job_name ASC
        "#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": rows })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateCronjobConfig {
    pub enabled: Option<bool>,
    pub schedule: Option<String>,
    pub description: Option<String>,
}

/// PUT /v1/admin/cronjob-config/{name} — toggle / reschedule a single job
pub async fn update_cronjob_config(
    State(state): State<AppState>,
    auth: AuthUser,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(req): Json<UpdateCronjobConfig>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageCronjobs, &state).await?;
    // Upsert — if admin enables a never-seen-before job, create the row.
    let row = sqlx::query_as::<_, CronjobConfigRow>(
        r#"
        INSERT INTO cronjob_config (job_name, schedule, enabled, description, updated_at)
        VALUES ($1,
                COALESCE($3, '@manual'),
                COALESCE($2, TRUE),
                $4,
                NOW())
        ON CONFLICT (job_name) DO UPDATE
           SET enabled     = COALESCE(EXCLUDED.enabled,     cronjob_config.enabled),
               schedule    = COALESCE(EXCLUDED.schedule,    cronjob_config.schedule),
               description = COALESCE(EXCLUDED.description, cronjob_config.description),
               updated_at  = NOW()
        RETURNING job_name, schedule, enabled, NULL::timestamptz AS last_run_at,
                  NULL::varchar AS last_status, description, updated_at
        "#,
    )
    .bind(&name)
    .bind(req.enabled)
    .bind(req.schedule.as_deref())
    .bind(req.description.as_deref())
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": row })))
}
