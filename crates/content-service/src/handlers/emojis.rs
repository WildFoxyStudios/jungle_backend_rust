//! Public emoji catalog endpoint.
//!
//! Mirrors WoWonder's `load-emojies.php`. Returns the full list of emojis the
//! server ships with (unicode + optional custom images uploaded by admin),
//! grouped by category. The response is safe to cache aggressively on the
//! client — emojis change rarely.

use axum::{extract::State, Json};
use serde::Serialize;
use serde_json::{json, Value};
use shared::{auth::AppState, errors::ApiError};
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow)]
pub struct EmojiRow {
    pub id: i64,
    pub shortcode: String,
    pub unicode: Option<String>,
    pub image_url: Option<String>,
    pub category: String,
    pub is_custom: bool,
    pub sort_order: i32,
}

/// GET /v1/emojis
///
/// Returns an array of emoji rows and a convenience map grouped by category.
/// Returns an empty set gracefully if the `emojis` table does not yet exist.
pub async fn list_emojis(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, EmojiRow>(
        r#"SELECT id, shortcode, unicode, image_url, category, is_custom, sort_order
             FROM emojis
         ORDER BY category ASC, sort_order ASC, id ASC"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // Group by category for convenient client rendering
    let mut by_category: std::collections::BTreeMap<String, Vec<&EmojiRow>> =
        std::collections::BTreeMap::new();
    for row in &rows {
        by_category
            .entry(row.category.clone())
            .or_default()
            .push(row);
    }

    Ok(Json(json!({
        "data": {
            "items": rows,
            "by_category": by_category,
        }
    })))
}
