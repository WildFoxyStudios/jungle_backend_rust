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

    Ok(Json(json!({ "data": jobs, "meta": { "cursor": next_cursor, "has_more": has_more } })))
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

pub async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let job = sqlx::query_as::<_, JobRow>("SELECT * FROM jobs WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Job not found".into()))?;

    Ok(Json(json!({ "data": job })))
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

    sqlx::query("DELETE FROM jobs WHERE id = $1").bind(id).execute(&state.db).await?;
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
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM jobs WHERE id = $1 AND status = 'active'")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Job not found or inactive".into()))?;

    if owner == auth.user_id {
        return Err(ApiError::BadRequest("Cannot apply to your own job".into()));
    }

    sqlx::query(
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
        return Err(ApiError::BadRequest(format!("Invalid status. Use: {}", valid.join(", "))));
    }

    // Verify caller owns the job
    let job_id = sqlx::query_scalar::<_, i64>("SELECT job_id FROM job_applications WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Application not found".into()))?;

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
    .bind(params.location.as_ref().map(|l| format!("%{l}%")).unwrap_or_default())
    .bind(params.job_type.as_deref())
    .bind(params.category_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = jobs.len() as i64 > limit;
    let data: Vec<_> = jobs.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|j| j.id.to_string());

    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

/// GET /v1/jobs/categories
pub async fn job_categories(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let cats = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, name FROM categories WHERE type = 'job' ORDER BY name",
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = cats.into_iter().map(|(id, name)| json!({"id": id, "name": name})).collect();
    Ok(Json(json!({ "data": data })))
}
