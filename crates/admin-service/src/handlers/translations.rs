use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{auth::AppState, errors::ApiError};

#[derive(Debug, Deserialize)]
pub struct TranslationQuery {
    pub lang: String,
    pub q: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn list_translations(
    State(state): State<AppState>,
    Query(q): Query<TranslationQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = (q.page.unwrap_or(1) - 1).max(0) * limit;
    let search = q.q.as_deref().unwrap_or("");
    let ilike = format!("%{}%", search);

    let rows = sqlx::query_as::<_, (i64, String, String, String)>(
        r#"SELECT id, lang, key, value FROM translations
        WHERE lang = $1 AND ($2 = '' OR key ILIKE $3 OR value ILIKE $3)
        ORDER BY key ASC
        LIMIT $4 OFFSET $5"#,
    )
    .bind(&q.lang)
    .bind(search)
    .bind(&ilike)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, lang, key, value)| json!({ "id": id, "lang": lang, "key": key, "value": value }))
        .collect();

    Ok(Json(json!({ "data": data })))
}

#[derive(Debug, Deserialize)]
pub struct UpsertTranslationRequest {
    pub lang: String,
    pub key: String,
    pub value: String,
}

pub async fn upsert_translation(
    State(state): State<AppState>,
    Json(req): Json<UpsertTranslationRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO translations (lang, key, value)
        VALUES ($1, $2, $3)
        ON CONFLICT (lang, key) DO UPDATE SET value = $3
        RETURNING id"#,
    )
    .bind(&req.lang)
    .bind(&req.key)
    .bind(&req.value)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id } })))
}

#[derive(Debug, Deserialize)]
pub struct BulkTranslationRequest {
    pub lang: String,
    pub translations: Vec<KeyValue>,
}

#[derive(Debug, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

pub async fn bulk_upsert_translations(
    State(state): State<AppState>,
    Json(req): Json<BulkTranslationRequest>,
) -> Result<Json<Value>, ApiError> {
    let mut count = 0i64;
    for kv in &req.translations {
        sqlx::query(
            "INSERT INTO translations (lang, key, value) VALUES ($1, $2, $3) ON CONFLICT (lang, key) DO UPDATE SET value = $3",
        )
        .bind(&req.lang)
        .bind(&kv.key)
        .bind(&kv.value)
        .execute(&state.db)
        .await?;
        count += 1;
    }

    Ok(Json(json!({ "data": { "updated": count } })))
}

pub async fn delete_translation(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM translations WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}
