use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{auth::AppState, errors::ApiError};

#[derive(Debug, Deserialize)]
pub struct ActivityLogQuery {
    pub action: Option<String>,
    pub user_id: Option<i64>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn list_activities(
    State(state): State<AppState>,
    Query(q): Query<ActivityLogQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let offset = (q.page.unwrap_or(1) - 1).max(0) * limit;

    let rows = sqlx::query_as::<_, (i64, i64, String, Option<String>, Option<String>, time::OffsetDateTime)>(
        r#"SELECT a.id, a.user_id, a.action, a.target_type, a.details, a.created_at
        FROM activities a
        WHERE ($1::text IS NULL OR a.action = $1)
          AND ($2::bigint IS NULL OR a.user_id = $2)
        ORDER BY a.created_at DESC
        LIMIT $3 OFFSET $4"#,
    )
    .bind(&q.action)
    .bind(q.user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(id, user_id, action, target_type, details, created_at)| {
            json!({
                "id": id, "user_id": user_id, "action": action,
                "target_type": target_type, "details": details,
                "created_at": created_at.to_string()
            })
        })
        .collect();

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM activities
        WHERE ($1::text IS NULL OR action = $1)
          AND ($2::bigint IS NULL OR user_id = $2)"#,
    )
    .bind(&q.action)
    .bind(q.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": data, "meta": { "total": total } })))
}
