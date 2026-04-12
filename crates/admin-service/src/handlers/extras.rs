use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;

// ── Dashboard Charts ───────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct ChartPoint {
    pub date: String,
    pub count: i64,
}

/// GET /v1/admin/dashboard/charts — daily registrations and posts over last 30 days
pub async fn charts(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    let user_chart = sqlx::query_as::<_, ChartPoint>(
        r#"SELECT TO_CHAR(created_at::date, 'YYYY-MM-DD') as date, COUNT(*) as count
           FROM users
           WHERE created_at >= NOW() - INTERVAL '30 days'
           GROUP BY created_at::date
           ORDER BY created_at::date ASC"#,
    )
    .fetch_all(&state.db)
    .await?;

    let post_chart = sqlx::query_as::<_, ChartPoint>(
        r#"SELECT TO_CHAR(created_at::date, 'YYYY-MM-DD') as date, COUNT(*) as count
           FROM posts
           WHERE created_at >= NOW() - INTERVAL '30 days'
           GROUP BY created_at::date
           ORDER BY created_at::date ASC"#,
    )
    .fetch_all(&state.db)
    .await?;

    let revenue_chart = sqlx::query_as::<_, ChartPoint>(
        r#"SELECT TO_CHAR(created_at::date, 'YYYY-MM-DD') as date, COALESCE(SUM(amount)::bigint, 0) as count
           FROM payment_transactions
           WHERE status = 'completed' AND created_at >= NOW() - INTERVAL '30 days'
           GROUP BY created_at::date
           ORDER BY created_at::date ASC"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Ok(Json(json!({
        "data": {
            "users": user_chart,
            "posts": post_chart,
            "revenue": revenue_chart,
        }
    })))
}

/// GET /v1/admin/dashboard/top-countries — top 20 countries by user count
pub async fn top_countries(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    let rows = sqlx::query_as::<_, (String, i64)>(
        r#"SELECT COALESCE(country_id, 'unknown') as country, COUNT(*) as count
           FROM users WHERE deleted_at IS NULL
           GROUP BY country_id ORDER BY count DESC LIMIT 20"#,
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows.into_iter().map(|(c, n)| json!({ "country": c, "count": n })).collect();
    Ok(Json(json!({ "data": data })))
}

/// GET /v1/admin/system-info — database size, Redis info, version
pub async fn system_info(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    // Database size
    let db_size: String = sqlx::query_scalar(
        "SELECT pg_size_pretty(pg_database_size(current_database()))",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or_else(|_| "unknown".into());

    // Table count
    let table_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    // PostgreSQL version
    let pg_version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&state.db)
        .await
        .unwrap_or_else(|_| "unknown".into());

    // Redis info
    let redis_info = match redis::cmd("INFO")
        .arg("server")
        .query_async::<String>(&mut state.redis.clone())
        .await
    {
        Ok(info) => {
            let version = info.lines()
                .find(|l| l.starts_with("redis_version:"))
                .map(|l| l.trim_start_matches("redis_version:").trim().to_string())
                .unwrap_or_else(|| "unknown".into());
            let uptime = info.lines()
                .find(|l| l.starts_with("uptime_in_seconds:"))
                .and_then(|l| l.trim_start_matches("uptime_in_seconds:").trim().parse::<i64>().ok())
                .unwrap_or(0);
            json!({ "version": version, "uptime_seconds": uptime, "connected": true })
        }
        Err(_) => json!({ "connected": false }),
    };

    // Disk usage (count of uploaded media)
    let media_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM uploaded_media")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    Ok(Json(json!({
        "data": {
            "version": env!("CARGO_PKG_VERSION"),
            "database": {
                "size": db_size,
                "tables": table_count,
                "version": pg_version,
            },
            "redis": redis_info,
            "storage": {
                "media_files": media_count,
                "provider": std::env::var("STORAGE_PROVIDER").unwrap_or_else(|_| "local".into()),
            },
        }
    })))
}

// ── Admin Ads Management ───────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct AdminAdRow {
    pub id: i64,
    pub user_id: i64,
    pub post_id: i64,
    pub audience: String,
    pub budget: f64,
    pub impressions: i64,
    pub clicks: i64,
    pub status: String,
    pub created_at: OffsetDateTime,
}

/// GET /v1/admin/ads
pub async fn list_ads(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let ads = sqlx::query_as::<_, AdminAdRow>(
        r#"SELECT id, user_id, post_id, audience, budget::float8 as budget,
                  impressions, clicks, status, created_at
           FROM user_ads WHERE id < $1
           ORDER BY id DESC LIMIT $2"#,
    )
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = ads.len() as i64 > limit;
    let data: Vec<_> = ads.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|a| a.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateAdStatusRequest {
    pub status: String,
}

/// PUT /v1/admin/ads/{id}
pub async fn update_ad(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateAdStatusRequest>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    let valid = ["active", "paused", "rejected", "completed"];
    if !valid.contains(&req.status.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid status. Must be one of: {}",
            valid.join(", ")
        )));
    }

    let result = sqlx::query("UPDATE user_ads SET status = $2 WHERE id = $1")
        .bind(id)
        .bind(&req.status)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Ad not found".into()));
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}
