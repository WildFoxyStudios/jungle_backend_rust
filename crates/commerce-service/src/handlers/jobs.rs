use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    events::DomainEvent,
    pagination::PaginationParams,
};
use sqlx::{FromRow, Row};
use time::OffsetDateTime;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateJobRequest {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub salary_min: Option<rust_decimal::Decimal>,
    pub salary_max: Option<rust_decimal::Decimal>,
    pub salary_period: Option<String>,
    pub job_type: Option<String>,
    pub category_id: Option<i64>,
    pub image: Option<String>,
    pub currency: Option<String>,
    pub questions: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateJobRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub salary_min: Option<rust_decimal::Decimal>,
    pub salary_max: Option<rust_decimal::Decimal>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApplyRequest {
    pub answers: Option<Value>,
    pub cover_letter: Option<String>,
    pub resume_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApplicationStatusRequest {
    pub status: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct JobRow {
    pub id: i64,
    pub uuid: uuid::Uuid,
    pub user_id: i64,
    pub page_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub location: String,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub salary_min: Option<rust_decimal::Decimal>,
    pub salary_max: Option<rust_decimal::Decimal>,
    pub salary_period: String,
    pub job_type: String,
    pub category_id: Option<i64>,
    pub image: String,
    pub currency: String,
    pub questions: Value,
    pub status: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ApplicationRow {
    pub id: i64,
    pub job_id: i64,
    pub user_id: i64,
    pub answers: Value,
    pub cover_letter: String,
    pub resume_url: String,
    pub status: String,
    pub created_at: OffsetDateTime,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
}

pub async fn list_jobs(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let jobs = sqlx::query_as::<_, JobRow>(
        "SELECT * FROM jobs WHERE status = 'active' AND ($1::bigint IS NULL OR id < $1) ORDER BY id DESC LIMIT $2",
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = jobs.len() as i64 > limit;
    let jobs: Vec<_> = jobs.into_iter().take(limit as usize).collect();
    let next_cursor = jobs.last().map(|j| j.id.to_string());

    Ok(Json(
        json!({ "data": jobs, "meta": { "cursor": next_cursor, "has_more": has_more } }),
    ))
}

pub async fn create_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateJobRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(ApiError::from)?;

    let job = sqlx::query_as::<_, JobRow>(
        r#"
        INSERT INTO jobs (user_id, title, description, location, lat, lng, salary_min, salary_max, salary_period, job_type, category_id, image, currency, questions)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        RETURNING *
        "#,
    )
    .bind(auth.user_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.location.as_deref().unwrap_or(""))
    .bind(req.lat)
    .bind(req.lng)
    .bind(req.salary_min)
    .bind(req.salary_max)
    .bind(req.salary_period.as_deref().unwrap_or("monthly"))
    .bind(req.job_type.as_deref().unwrap_or("full_time"))
    .bind(req.category_id)
    .bind(req.image.as_deref().unwrap_or(""))
    .bind(req.currency.as_deref().unwrap_or("USD"))
    .bind(req.questions.as_ref().unwrap_or(&json!([])))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": job })))
}

#[derive(Debug, FromRow)]
struct JobPosterRow {
    id: i64,
    uuid: uuid::Uuid,
    username: String,
    first_name: String,
    last_name: String,
    avatar: String,
    is_verified: bool,
    is_pro: i16,
}

pub async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let job = sqlx::query_as::<_, JobRow>(
        r#"SELECT id, uuid, user_id, page_id, title, description,
                  COALESCE(location, '') AS location,
                  lat, lng, salary_min, salary_max,
                  COALESCE(salary_period, 'monthly') AS salary_period,
                  COALESCE(job_type, 'full_time') AS job_type,
                  category_id,
                  COALESCE(image, '') AS image,
                  COALESCE(currency, 'USD') AS currency,
                  COALESCE(questions, '[]'::jsonb) AS questions,
                  COALESCE(status, 'active') AS status,
                  created_at
           FROM jobs WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Job not found".into()))?;

    let poster = sqlx::query_as::<_, JobPosterRow>(
        r#"
        SELECT u.id, u.uuid, u.username, u.first_name, u.last_name, u.avatar,
               COALESCE(u.is_verified, FALSE) AS is_verified,
               COALESCE(u.is_pro, 0::smallint) AS is_pro
        FROM users u
        WHERE u.id = $1 AND u.deleted_at IS NULL
        "#,
    )
    .bind(job.user_id)
    .fetch_optional(&state.db)
    .await?;

    let category: Option<String> = match job.category_id {
        Some(cid) => sqlx::query_scalar::<_, String>(
            "SELECT name_key FROM categories WHERE id = $1 AND active = TRUE",
        )
        .bind(cid)
        .fetch_optional(&state.db)
        .await?,
        None => None,
    };

    let application_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM job_applications WHERE job_id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let mut job_val = serde_json::to_value(&job).unwrap_or(json!({}));
    if let Some(obj) = job_val.as_object_mut() {
        obj.insert(
            "category".to_string(),
            json!(category.unwrap_or_default()),
        );
        obj.insert(
            "poster".to_string(),
            match poster {
                Some(p) => json!({
                    "id": p.id,
                    "uuid": p.uuid,
                    "username": p.username,
                    "first_name": p.first_name,
                    "last_name": p.last_name,
                    "avatar": p.avatar,
                    "is_verified": p.is_verified,
                    "is_pro": p.is_pro,
                    "is_online": false,
                }),
                None => Value::Null,
            },
        );
        obj.insert("application_count".to_string(), json!(application_count));
        obj.insert(
            "is_active".to_string(),
            json!(job.status == "active"),
        );
    }

    Ok(Json(json!({ "data": job_val })))
}

pub async fn update_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateJobRequest>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM jobs WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Job not found".into()))?;

    if owner != auth.user_id {
        return Err(ApiError::Forbidden("".into()));
    }

    let job = sqlx::query_as::<_, JobRow>(
        r#"
        UPDATE jobs SET
            title = COALESCE($2, title),
            description = COALESCE($3, description),
            location = COALESCE($4, location),
            salary_min = COALESCE($5, salary_min),
            salary_max = COALESCE($6, salary_max),
            status = COALESCE($7, status)
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(&req.location)
    .bind(req.salary_min)
    .bind(req.salary_max)
    .bind(&req.status)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": job })))
}

pub async fn delete_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM jobs WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Job not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    sqlx::query("DELETE FROM jobs WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn my_jobs(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let jobs = sqlx::query_as::<_, JobRow>(
        "SELECT * FROM jobs WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": jobs })))
}

pub async fn apply_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ApplyRequest>,
) -> Result<Json<Value>, ApiError> {
    // Can't apply to own job
    let owner = sqlx::query_scalar::<_, i64>(
        "SELECT user_id FROM jobs WHERE id = $1 AND status = 'active'",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Job not found or inactive".into()))?;

    if owner == auth.user_id {
        return Err(ApiError::BadRequest("Cannot apply to your own job".into()));
    }

    let result = sqlx::query(
        r#"
        INSERT INTO job_applications (job_id, user_id, answers, cover_letter, resume_url)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (job_id, user_id) DO NOTHING
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(req.answers.as_ref().unwrap_or(&json!({})))
    .bind(req.cover_letter.as_deref().unwrap_or(""))
    .bind(req.resume_url.as_deref().unwrap_or(""))
    .execute(&state.db)
    .await?;

    if result.rows_affected() > 0 {
        let _ = state.event_bus.publish(&DomainEvent::JobApplicationSubmitted {
            job_id: id,
            applicant_id: auth.user_id,
            employer_id: owner,
        }).await;
    }

    Ok(Json(json!({ "data": { "applied": true } })))
}

pub async fn list_applications(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    // Only job owner can see applications
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM jobs WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Job not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    let apps = sqlx::query_as::<_, ApplicationRow>(
        r#"
        SELECT ja.id, ja.job_id, ja.user_id, ja.answers, ja.cover_letter, ja.resume_url, ja.status, ja.created_at,
            u.username, u.first_name, u.last_name, u.avatar
        FROM job_applications ja JOIN users u ON u.id = ja.user_id
        WHERE ja.job_id = $1
        ORDER BY ja.created_at DESC
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": apps })))
}

pub async fn update_application_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateApplicationStatusRequest>,
) -> Result<Json<Value>, ApiError> {
    let valid = ["pending", "reviewing", "accepted", "rejected"];
    if !valid.contains(&req.status.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid status. Use: {}",
            valid.join(", ")
        )));
    }

    // Verify caller owns the job
    let app_row = sqlx::query("SELECT job_id, user_id FROM job_applications WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Application not found".into()))?;

    let job_id: i64 = app_row.get("job_id");
    let applicant_id: i64 = app_row.get("user_id");

    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM jobs WHERE id = $1")
        .bind(job_id)
        .fetch_one(&state.db)
        .await?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    sqlx::query("UPDATE job_applications SET status = $1 WHERE id = $2")
        .bind(&req.status)
        .bind(id)
        .execute(&state.db)
        .await?;

    let _ = state.event_bus.publish(&DomainEvent::ApplicationStatusChanged {
        application_id: id,
        job_id,
        applicant_id,
        new_status: req.status.clone(),
    }).await;

    Ok(Json(json!({ "data": { "status": req.status } })))
}

/// GET /v1/jobs/applied — jobs the current user has applied to
pub async fn applied_jobs(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let jobs = sqlx::query_as::<_, JobRow>(
        r#"SELECT j.* FROM jobs j
           JOIN job_applications ja ON ja.job_id = j.id
           WHERE ja.user_id = $1
           ORDER BY ja.created_at DESC"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": jobs })))
}

#[derive(Debug, Deserialize)]
pub struct JobSearchParams {
    pub q: Option<String>,
    pub location: Option<String>,
    pub job_type: Option<String>,
    pub category_id: Option<i64>,
    pub cursor: Option<i64>,
    pub limit: Option<i64>,
}

/// GET /v1/jobs/search
pub async fn search_jobs(
    State(state): State<AppState>,
    Query(params): Query<JobSearchParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit.unwrap_or(20).clamp(1, 50);
    let cursor = params.cursor.unwrap_or(i64::MAX);
    let q = params.q.as_deref().unwrap_or("");
    let ilike = format!("%{q}%");

    let jobs = sqlx::query_as::<_, JobRow>(
        r#"SELECT * FROM jobs
           WHERE status = 'active'
             AND id < $1
             AND ($2 = '' OR title ILIKE $3 OR description ILIKE $3)
             AND ($4::text IS NULL OR location ILIKE $5)
             AND ($6::text IS NULL OR job_type = $6)
             AND ($7::bigint IS NULL OR category_id = $7)
           ORDER BY id DESC LIMIT $8"#,
    )
    .bind(cursor)
    .bind(q)
    .bind(&ilike)
    .bind(params.location.as_deref())
    .bind(
        params
            .location
            .as_ref()
            .map(|l| format!("%{l}%"))
            .unwrap_or_default(),
    )
    .bind(params.job_type.as_deref())
    .bind(params.category_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = jobs.len() as i64 > limit;
    let data: Vec<_> = jobs.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|j| j.id.to_string());

    Ok(Json(
        json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } }),
    ))
}

/// GET /v1/jobs/categories
pub async fn job_categories(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let cats = sqlx::query_as::<_, (i64, String)>(
        r#"SELECT id, name_key AS "name!" FROM categories
           WHERE type = 'job' AND active = TRUE
           ORDER BY sort_order, name_key"#,
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = cats
        .into_iter()
        .map(|(id, name)| json!({"id": id, "name": name}))
        .collect();
    Ok(Json(json!({ "data": data })))
}

#[derive(Debug, Deserialize)]
pub struct NearbyParams {
    pub lat: f64,
    pub lng: f64,
    pub radius_km: Option<f64>,
    pub cursor: Option<i64>,
    pub limit: Option<i64>,
}

/// GET /v1/jobs/nearby — jobs within radius_km of (lat, lng).
///
/// Uses the haversine formula evaluated in SQL; avoids a PostGIS dependency.
pub async fn nearby_jobs(
    State(state): State<AppState>,
    Query(params): Query<NearbyParams>,
) -> Result<Json<Value>, ApiError> {
    let radius_km = params.radius_km.unwrap_or(25.0).clamp(1.0, 500.0);
    let limit = params.limit.unwrap_or(30).clamp(1, 100);

    type Row = (
        i64,
        i64,
        String,
        Option<String>,
        String,
        f64,
        f64,
        String,
        f64,
        OffsetDateTime,
    );

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT j.id, j.user_id, j.title, j.description, j.location,
               j.lat, j.lng, j.job_type,
               (6371 * acos(
                   GREATEST(-1.0, LEAST(1.0,
                     cos(radians($1)) * cos(radians(j.lat)) *
                     cos(radians(j.lng) - radians($2)) +
                     sin(radians($1)) * sin(radians(j.lat))
                   ))
               ))::double precision AS distance_km,
               j.created_at
          FROM jobs j
         WHERE j.lat IS NOT NULL AND j.lng IS NOT NULL
           AND j.status = 'open'
           AND (6371 * acos(
                   GREATEST(-1.0, LEAST(1.0,
                     cos(radians($1)) * cos(radians(j.lat)) *
                     cos(radians(j.lng) - radians($2)) +
                     sin(radians($1)) * sin(radians(j.lat))
                   ))
           )) <= $3
           AND ($4::bigint IS NULL OR j.id < $4)
        ORDER BY distance_km ASC, j.id DESC
        LIMIT $5
        "#,
    )
    .bind(params.lat)
    .bind(params.lng)
    .bind(radius_km)
    .bind(params.cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let rows: Vec<Row> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = rows.last().map(|r| r.0);

    let data: Vec<Value> = rows
        .into_iter()
        .map(
            |(id, uid, title, desc, loc, lat, lng, jtype, dist, created)| {
                json!({
                    "id": id,
                    "user_id": uid,
                    "title": title,
                    "description": desc,
                    "location": loc,
                    "lat": lat,
                    "lng": lng,
                    "job_type": jtype,
                    "distance_km": dist,
                    "created_at": created.to_string(),
                })
            },
        )
        .collect();

    Ok(Json(json!({
        "data": data,
        "meta": { "has_more": has_more, "next_cursor": next_cursor }
    })))
}

// ── Saved Jobs ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SaveJobRequest { pub job_id: i64 }

pub async fn save_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<SaveJobRequest>,
) -> Result<Json<()>, ApiError> {
    sqlx::query("INSERT INTO saved_jobs (user_id, job_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(auth.user_id).bind(body.job_id)
        .execute(&state.db).await
        .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;
    Ok(Json(()))
}

pub async fn list_saved_jobs(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let rows = sqlx::query(
        "SELECT j.* FROM jobs j JOIN saved_jobs sj ON sj.job_id = j.id WHERE sj.user_id = $1 ORDER BY sj.saved_at DESC LIMIT 50"
    )
    .bind(auth.user_id).fetch_all(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    let items: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
        "id": r.get::<i64, _>("id"),
        "title": r.get::<String, _>("title"),
        "location": r.get::<Option<String>, _>("location"),
        "created_at": r.get::<String, _>("created_at"),
    })).collect();
    Ok(Json(items))
}

// ── Job Alerts ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateJobAlertRequest {
    pub query: Option<String>,
    pub frequency: Option<String>,
}

pub async fn create_job_alert(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateJobAlertRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query(
        "INSERT INTO job_alerts (user_id, query, frequency) VALUES ($1, $2, $3) RETURNING id"
    )
    .bind(auth.user_id).bind(&body.query).bind(body.frequency.as_deref().unwrap_or("weekly"))
    .fetch_one(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    Ok(Json(serde_json::json!({ "id": row.get::<i64, _>("id") })))
}

pub async fn list_job_alerts(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, query, frequency, is_active, created_at FROM job_alerts WHERE user_id = $1 ORDER BY created_at DESC"
    )
    .bind(auth.user_id).fetch_all(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    let items: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
        "id": r.get::<i64, _>("id"),
        "query": r.get::<Option<String>, _>("query"),
        "frequency": r.get::<String, _>("frequency"),
        "is_active": r.get::<bool, _>("is_active"),
        "created_at": r.get::<String, _>("created_at"),
    })).collect();
    Ok(Json(items))
}

// ── Resume Upload ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct UploadResumeRequest {
    pub file_url: String,
    pub file_name: Option<String>,
}

pub async fn upload_resume(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<UploadResumeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Deactivate previous resumes
    sqlx::query("UPDATE user_resumes SET is_active = FALSE WHERE user_id = $1")
        .bind(auth.user_id).execute(&state.db).await
        .map_err(|e| { tracing::error!(error = %e, "Failed to deactivate previous resumes"); ApiError::Internal("DB error".into()) })?;

    let row = sqlx::query(
        "INSERT INTO user_resumes (user_id, file_url, file_name, is_active, uploaded_at)
         VALUES ($1, $2, $3, TRUE, NOW()) RETURNING id"
    )
    .bind(auth.user_id).bind(&body.file_url).bind(&body.file_name)
    .fetch_one(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    Ok(Json(serde_json::json!({ "id": row.get::<i64, _>("id") })))
}

pub async fn get_my_resume(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let row = sqlx::query(
        "SELECT id, file_url, file_name, extracted_text, skills, experience_years, uploaded_at
         FROM user_resumes WHERE user_id = $1 AND is_active = TRUE ORDER BY uploaded_at DESC LIMIT 1"
    )
    .bind(auth.user_id).fetch_optional(&state.db).await
    .map_err(|e| { tracing::error!(error = %e); ApiError::Internal("DB error".into()) })?;

    if let Some(r) = row {
        Ok(Json(serde_json::json!({
            "id": r.get::<i64, _>("id"),
            "file_url": r.get::<String, _>("file_url"),
            "file_name": r.get::<Option<String>, _>("file_name"),
            "extracted_text": r.get::<Option<String>, _>("extracted_text"),
            "skills": r.get::<Option<serde_json::Value>, _>("skills"),
            "experience_years": r.get::<Option<i32>, _>("experience_years"),
            "uploaded_at": r.get::<String, _>("uploaded_at"),
        })))
    } else {
        Ok(Json(serde_json::json!(null)))
    }
}
