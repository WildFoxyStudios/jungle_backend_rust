use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};
use sqlx::FromRow;

fn require_admin(auth: &AuthUser) -> Result<(), ApiError> {
    if !auth.is_admin { return Err(ApiError::Forbidden("".into())); }
    Ok(())
}

#[derive(Debug, Serialize, FromRow)]
pub struct ConfigRow {
    pub id: i64,
    pub category: String,
    pub key: String,
    pub value: String,
    pub value_type: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub items: Vec<ConfigItem>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigItem {
    pub category: String,
    pub key: String,
    pub value: String,
}

pub async fn list_config(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = sqlx::query_as::<_, ConfigRow>(
        "SELECT id, category, key, value, value_type FROM site_config ORDER BY category, key",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": rows })))
}

pub async fn get_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(category): Path<String>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let rows = sqlx::query_as::<_, ConfigRow>(
        "SELECT id, category, key, value, value_type FROM site_config WHERE category = $1 ORDER BY key",
    )
    .bind(&category)
    .fetch_all(&state.db)
    .await?;

    // Return as a flat { key: value } map — easier for the settings-form UI,
    // which looks up `data[field.key]` by key. Booleans are coerced based on
    // `value_type` so the frontend's `<Switch>` binds cleanly without
    // string parsing. Numbers stay as strings and the form coerces on save.
    let mut map = serde_json::Map::new();
    for row in rows {
        let value: Value = match row.value_type.as_str() {
            "boolean" | "bool" => {
                let truthy = matches!(row.value.to_lowercase().as_str(), "true" | "1" | "yes" | "on");
                Value::Bool(truthy)
            }
            "json" => serde_json::from_str(&row.value).unwrap_or(Value::String(row.value.clone())),
            _ => Value::String(row.value),
        };
        map.insert(row.key, value);
    }

    Ok(Json(json!({ "data": map })))
}

pub async fn update_config(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateConfigRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let mut updated = 0i64;
    for item in &req.items {
        let result = sqlx::query(
            r#"
            INSERT INTO site_config (category, key, value)
            VALUES ($1, $2, $3)
            ON CONFLICT (category, key) DO UPDATE SET value = EXCLUDED.value
            "#,
        )
        .bind(&item.category)
        .bind(&item.key)
        .bind(&item.value)
        .execute(&state.db)
        .await?;
        updated += result.rows_affected() as i64;
    }

    Ok(Json(json!({ "data": { "updated": updated } })))
}

/// PUT /v1/admin/config/{category}
///
/// Accepts a flat `{ key: value }` JSON body and upserts every key under the
/// given category. Used by the catalog-driven settings forms in the admin UI
/// (`Admin A1`); the shape matches `GET /v1/admin/config/{category}` so a
/// client can round-trip without reshaping.
///
/// Values are always stored as strings (the column type is `text`). The
/// frontend is responsible for stringifying booleans as "true"/"false" and
/// numbers as their decimal representation — the read side reverses that.
pub async fn update_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(category): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let obj = body
        .as_object()
        .ok_or_else(|| ApiError::BadRequest("Body must be a JSON object".into()))?;

    let mut tx = state.db.begin().await?;
    let mut updated = 0i64;

    for (key, value) in obj {
        let value_str = match value {
            Value::Bool(b) => if *b { "true".into() } else { "false".into() },
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            other => other.to_string(), // arrays / objects → JSON text
        };

        let value_type = match value {
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::Array(_) | Value::Object(_) => "json",
            _ => "text",
        };

        sqlx::query(
            r#"
            INSERT INTO site_config (category, key, value, value_type)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (category, key) DO UPDATE
                SET value = EXCLUDED.value,
                    value_type = EXCLUDED.value_type
            "#,
        )
        .bind(&category)
        .bind(key)
        .bind(&value_str)
        .bind(value_type)
        .execute(&mut *tx)
        .await?;
        updated += 1;
    }

    tx.commit().await?;

    Ok(Json(json!({ "data": { "updated": updated, "category": category } })))
}
