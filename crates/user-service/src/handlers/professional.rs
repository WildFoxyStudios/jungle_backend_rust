use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

// ── Experience ──

#[derive(Debug, Deserialize)]
pub struct ExperienceRequest {
    pub title: String,
    pub company: String,
    pub location: Option<String>,
    pub description: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub is_current: Option<bool>,
}

pub async fn list_experience(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<
        _,
        (
            i64,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<time::Date>,
            Option<time::Date>,
            bool,
        ),
    >(
        r#"SELECT id, title, company, location, description, start_date, end_date, is_current
        FROM user_experience WHERE user_id = $1 ORDER BY start_date DESC NULLS FIRST"#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(
            |(id, title, company, location, desc, start, end, current)| {
                json!({
                    "id": id,
                    "title": title,
                    "company": company,
                    "location": location,
                    "description": desc,
                    "start_date": start.map(|d| d.to_string()),
                    "end_date": end.map(|d| d.to_string()),
                    "is_current": current
                })
            },
        )
        .collect();

    Ok(Json(json!({ "data": data })))
}

pub async fn list_my_experience(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<
        _,
        (
            i64,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<time::Date>,
            Option<time::Date>,
            bool,
        ),
    >(
        r#"SELECT id, title, company, location, description, start_date, end_date, is_current
        FROM user_experience WHERE user_id = $1 ORDER BY start_date DESC NULLS FIRST"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(
            |(id, title, company, location, desc, start, end, current)| {
                json!({
                    "id": id,
                    "title": title,
                    "company": company,
                    "location": location,
                    "description": desc,
                    "start_date": start.map(|d| d.to_string()),
                    "end_date": end.map(|d| d.to_string()),
                    "is_current": current
                })
            },
        )
        .collect();

    Ok(Json(json!({ "data": data })))
}

pub async fn add_experience(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ExperienceRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO user_experience (user_id, title, company, location, description, start_date, end_date, is_current)
        VALUES ($1, $2, $3, $4, $5, $6::date, $7::date, $8)
        RETURNING id"#,
    )
    .bind(auth.user_id)
    .bind(&req.title)
    .bind(&req.company)
    .bind(&req.location)
    .bind(&req.description)
    .bind(&req.start_date)
    .bind(&req.end_date)
    .bind(req.is_current.unwrap_or(false))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id } })))
}

pub async fn delete_experience(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM user_experience WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn update_experience(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ExperienceRequest>,
) -> Result<Json<Value>, ApiError> {
    let updated = sqlx::query(
        r#"UPDATE user_experience
           SET title = $3, company = $4, location = $5, description = $6,
               start_date = $7::date, end_date = $8::date, is_current = $9
           WHERE id = $1 AND user_id = $2"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&req.title)
    .bind(&req.company)
    .bind(&req.location)
    .bind(&req.description)
    .bind(&req.start_date)
    .bind(&req.end_date)
    .bind(req.is_current.unwrap_or(false))
    .execute(&state.db)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(ApiError::NotFound("Experience not found".into()));
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

// ── Certifications ──

#[derive(Debug, Deserialize)]
pub struct CertificationRequest {
    pub name: String,
    pub organization: Option<String>,
    pub issue_date: Option<String>,
    pub expiry_date: Option<String>,
    pub credential_url: Option<String>,
}

pub async fn list_certifications(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<
        _,
        (
            i64,
            String,
            Option<String>,
            Option<time::Date>,
            Option<time::Date>,
            Option<String>,
        ),
    >(
        r#"SELECT id, name, organization, issue_date, expiry_date, credential_url
        FROM user_certifications WHERE user_id = $1 ORDER BY issue_date DESC NULLS FIRST"#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, name, org, issue, expiry, url)| {
            json!({
                "id": id,
                "name": name,
                "organization": org,
                "issue_date": issue.map(|d| d.to_string()),
                "expiry_date": expiry.map(|d| d.to_string()),
                "credential_url": url
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

pub async fn list_my_certifications(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<
        _,
        (
            i64,
            String,
            Option<String>,
            Option<time::Date>,
            Option<time::Date>,
            Option<String>,
        ),
    >(
        r#"SELECT id, name, organization, issue_date, expiry_date, credential_url
        FROM user_certifications WHERE user_id = $1 ORDER BY issue_date DESC NULLS FIRST"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, name, org, issue, expiry, url)| {
            json!({
                "id": id,
                "name": name,
                "organization": org,
                "issue_date": issue.map(|d| d.to_string()),
                "expiry_date": expiry.map(|d| d.to_string()),
                "credential_url": url
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

pub async fn add_certification(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CertificationRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO user_certifications (user_id, name, organization, issue_date, expiry_date, credential_url)
        VALUES ($1, $2, $3, $4::date, $5::date, $6)
        RETURNING id"#,
    )
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(&req.organization)
    .bind(&req.issue_date)
    .bind(&req.expiry_date)
    .bind(&req.credential_url)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id } })))
}

pub async fn delete_certification(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM user_certifications WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Projects ──

#[derive(Debug, Deserialize)]
pub struct ProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

pub async fn list_projects(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<
        _,
        (
            i64,
            String,
            Option<String>,
            Option<String>,
            Option<time::Date>,
            Option<time::Date>,
        ),
    >(
        r#"SELECT id, name, description, url, start_date, end_date
        FROM user_projects WHERE user_id = $1 ORDER BY start_date DESC NULLS FIRST"#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, name, desc, url, start, end)| {
            json!({
                "id": id,
                "name": name,
                "description": desc,
                "url": url,
                "start_date": start.map(|d| d.to_string()),
                "end_date": end.map(|d| d.to_string())
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

pub async fn list_my_projects(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<
        _,
        (
            i64,
            String,
            Option<String>,
            Option<String>,
            Option<time::Date>,
            Option<time::Date>,
        ),
    >(
        r#"SELECT id, name, description, url, start_date, end_date
        FROM user_projects WHERE user_id = $1 ORDER BY start_date DESC NULLS FIRST"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, name, desc, url, start, end)| {
            json!({
                "id": id,
                "name": name,
                "description": desc,
                "url": url,
                "start_date": start.map(|d| d.to_string()),
                "end_date": end.map(|d| d.to_string())
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

pub async fn add_project(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ProjectRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO user_projects (user_id, name, description, url, start_date, end_date)
        VALUES ($1, $2, $3, $4, $5::date, $6::date)
        RETURNING id"#,
    )
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.url)
    .bind(&req.start_date)
    .bind(&req.end_date)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id } })))
}

pub async fn delete_project(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM user_projects WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn update_project(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<ProjectRequest>,
) -> Result<Json<Value>, ApiError> {
    let updated = sqlx::query(
        r#"UPDATE user_projects
           SET name = $3, description = $4, url = $5, start_date = $6::date, end_date = $7::date
           WHERE id = $1 AND user_id = $2"#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.url)
    .bind(&req.start_date)
    .bind(&req.end_date)
    .execute(&state.db)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(ApiError::NotFound("Project not found".into()));
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

// ── Mutual friends ──

pub async fn mutual_friends(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (i64, String, Option<String>, Option<String>, bool)>(
        r#"
        SELECT u.id, u.username, u.first_name, u.avatar, u.is_verified
        FROM follows f1
        JOIN follows f2 ON f2.following_id = f1.following_id AND f2.follower_id = $2 AND f2.status = 'active'
        JOIN users u ON u.id = f1.following_id AND u.deleted_at IS NULL
        WHERE f1.follower_id = $1 AND f1.status = 'active'
        ORDER BY u.first_name
        LIMIT 50
        "#,
    )
    .bind(auth.user_id)
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, username, first_name, avatar, verified)| {
            json!({
                "id": id,
                "username": username,
                "first_name": first_name,
                "avatar": avatar,
                "is_verified": verified
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

// ── Birthdays today ──

pub async fn birthdays_today(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (i64, String, Option<String>, Option<String>, bool)>(
        r#"
        SELECT u.id, u.username, u.first_name, u.avatar, u.is_verified
        FROM users u
        JOIN follows f ON f.following_id = u.id AND f.follower_id = $1 AND f.status = 'active'
        WHERE u.deleted_at IS NULL
          AND u.birthday IS NOT NULL
          AND EXTRACT(MONTH FROM u.birthday) = EXTRACT(MONTH FROM NOW())
          AND EXTRACT(DAY FROM u.birthday) = EXTRACT(DAY FROM NOW())
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, username, first_name, avatar, verified)| {
            json!({
                "id": id,
                "username": username,
                "first_name": first_name,
                "avatar": avatar,
                "is_verified": verified
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}
