use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    permissions::Permission,
};
use sqlx::Row;

#[derive(Deserialize)]
pub struct CreateArticleRequest {
    pub slug: String,
    pub title: String,
    pub content: String,
    pub category: Option<String>,
    pub locale: Option<String>,
}

pub async fn list_articles(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<Value>>, ApiError> {
    auth.require_permission(Permission::ManageSettings, &state).await?;

    let rows = sqlx::query(
        "SELECT id, slug, title, category, locale, is_published, sort_order, created_at, updated_at
         FROM help_articles ORDER BY sort_order, created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    let items: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.get::<i64, _>("id"),
                "slug": r.get::<String, _>("slug"),
                "title": r.get::<String, _>("title"),
                "category": r.get::<Option<String>, _>("category"),
                "locale": r.get::<String, _>("locale"),
                "is_published": r.get::<bool, _>("is_published"),
                "sort_order": r.get::<i32, _>("sort_order"),
            })
        })
        .collect();
    Ok(Json(items))
}

pub async fn create_article(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateArticleRequest>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageSettings, &state).await?;

    let row = sqlx::query(
        "INSERT INTO help_articles (slug, title, content, category, locale) VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(&body.slug)
    .bind(&body.title)
    .bind(&body.content)
    .bind(&body.category)
    .bind(body.locale.as_deref().unwrap_or("en"))
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;

    Ok(Json(json!({ "id": row.get::<i64, _>("id"), "slug": body.slug })))
}

pub async fn update_article(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(article_id): Path<i64>,
    Json(body): Json<CreateArticleRequest>,
) -> Result<Json<()>, ApiError> {
    auth.require_permission(Permission::ManageSettings, &state).await?;

    sqlx::query(
        "UPDATE help_articles SET slug=$1, title=$2, content=$3, category=$4, locale=$5, updated_at=NOW() WHERE id=$6",
    )
    .bind(&body.slug)
    .bind(&body.title)
    .bind(&body.content)
    .bind(&body.category)
    .bind(body.locale.as_deref().unwrap_or("en"))
    .bind(article_id)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e);
        ApiError::Internal("DB error".into())
    })?;
    Ok(Json(()))
}

pub async fn delete_article(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(article_id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    auth.require_permission(Permission::ManageSettings, &state).await?;

    sqlx::query("DELETE FROM help_articles WHERE id = $1")
        .bind(article_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = %e);
            ApiError::Internal("DB error".into())
        })?;
    Ok(Json(()))
}
