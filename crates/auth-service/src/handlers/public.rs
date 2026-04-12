use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};
use shared::{auth::AppState, errors::ApiError};

/// GET /v1/translations/{lang} — Public endpoint for frontend i18n
/// Returns all key-value translation pairs for the given language.
pub async fn get_translations(
    State(state): State<AppState>,
    Path(lang): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM translations WHERE lang = $1 ORDER BY key",
    )
    .bind(&lang)
    .fetch_all(&state.db)
    .await?;

    let translations: serde_json::Map<String, Value> = rows
        .into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect();

    Ok(Json(json!({ "data": translations })))
}

/// GET /v1/config/public — Public endpoint returning non-sensitive site configuration
/// Frontend uses this to know which features are enabled, site name, social providers, etc.
pub async fn get_public_config(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        r#"SELECT category, key, value FROM site_config
        WHERE category IN ('general', 'features', 'auth', 'appearance', 'limits')
        ORDER BY category, key"#,
    )
    .fetch_all(&state.db)
    .await?;

    let mut config: serde_json::Map<String, Value> = serde_json::Map::new();
    for (category, key, value) in rows {
        let cat = config
            .entry(category)
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if let Value::Object(map) = cat {
            map.insert(key, Value::String(value));
        }
    }

    Ok(Json(json!({ "data": config })))
}

/// GET /v1/site-settings — Full site settings for mobile/web clients
/// Matches PHP get-site-settings.php: returns config + categories + genders + currencies + colored posts
pub async fn get_site_settings(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let config_rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT category, key, value FROM site_config WHERE category NOT IN ('payment','email','sms') ORDER BY category, key",
    )
    .fetch_all(&state.db)
    .await?;

    let mut config: serde_json::Map<String, Value> = serde_json::Map::new();
    for (category, key, value) in config_rows {
        let cat = config.entry(category).or_insert_with(|| Value::Object(serde_json::Map::new()));
        if let Value::Object(map) = cat { map.insert(key, Value::String(value)); }
    }

    let page_cats = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, name_key FROM categories WHERE type = 'page' AND active = TRUE ORDER BY sort_order, id",
    ).fetch_all(&state.db).await?;

    let group_cats = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, name_key FROM categories WHERE type = 'group' AND active = TRUE ORDER BY sort_order, id",
    ).fetch_all(&state.db).await?;

    let blog_cats = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, name_key FROM categories WHERE type = 'blog' AND active = TRUE ORDER BY sort_order, id",
    ).fetch_all(&state.db).await?;

    let product_cats = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, name_key FROM categories WHERE type = 'product' AND active = TRUE ORDER BY sort_order, id",
    ).fetch_all(&state.db).await?;

    let job_cats = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, name_key FROM categories WHERE type = 'job' AND active = TRUE ORDER BY sort_order, id",
    ).fetch_all(&state.db).await?;

    let genders = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, name FROM genders ORDER BY id",
    ).fetch_all(&state.db).await?;

    let currencies = sqlx::query_as::<_, (String, String)>(
        "SELECT code, symbol FROM currencies WHERE is_active = TRUE ORDER BY code",
    ).fetch_all(&state.db).await?;

    let colored_posts = sqlx::query_as::<_, (i64, String, String, String)>(
        "SELECT id, color_1, color_2, text_color FROM colored_post_templates ORDER BY id",
    ).fetch_all(&state.db).await?;

    let reaction_types = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, icon FROM reaction_types WHERE is_active = TRUE ORDER BY id",
    ).fetch_all(&state.db).await?;

    Ok(Json(json!({
        "data": {
            "config": config,
            "page_categories": page_cats.iter().map(|(id, name)| json!({"id": id, "name": name})).collect::<Vec<_>>(),
            "group_categories": group_cats.iter().map(|(id, name)| json!({"id": id, "name": name})).collect::<Vec<_>>(),
            "blog_categories": blog_cats.iter().map(|(id, name)| json!({"id": id, "name": name})).collect::<Vec<_>>(),
            "products_categories": product_cats.iter().map(|(id, name)| json!({"id": id, "name": name})).collect::<Vec<_>>(),
            "job_categories": job_cats.iter().map(|(id, name)| json!({"id": id, "name": name})).collect::<Vec<_>>(),
            "genders": genders.iter().map(|(id, name)| json!({"id": id, "name": name})).collect::<Vec<_>>(),
            "currencies": currencies.iter().map(|(code, sym)| json!({"code": code, "symbol": sym})).collect::<Vec<_>>(),
            "post_colors": colored_posts.iter().map(|(id, c1, c2, tc)| json!({"id": id, "color_1": c1, "color_2": c2, "text_color": tc})).collect::<Vec<_>>(),
            "reaction_types": reaction_types.iter().map(|(id, name, icon)| json!({"id": id, "name": name, "icon": icon})).collect::<Vec<_>>()
        }
    })))
}

/// GET /v1/auth/check?field=username&value=john — Check username/email/phone availability
/// Matches PHP check_username.php and check_type.php
pub async fn check_availability(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<CheckAvailabilityParams>,
) -> Result<Json<Value>, ApiError> {
    let field = params.field.as_deref().unwrap_or("username");
    let value = params.value.trim();

    if value.is_empty() {
        return Err(ApiError::BadRequest("value is required".into()));
    }

    let taken = match field {
        "username" => {
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE LOWER(username) = LOWER($1) AND deleted_at IS NULL)")
                .bind(value)
                .fetch_one(&state.db)
                .await?
        }
        "email" => {
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE LOWER(email) = LOWER($1) AND deleted_at IS NULL)")
                .bind(value)
                .fetch_one(&state.db)
                .await?
        }
        "phone" => {
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE phone_number = $1 AND deleted_at IS NULL)")
                .bind(value)
                .fetch_one(&state.db)
                .await?
        }
        _ => return Err(ApiError::BadRequest("field must be 'username', 'email', or 'phone'".into())),
    };

    Ok(Json(json!({ "data": { "field": field, "value": value, "available": !taken } })))
}

#[derive(serde::Deserialize)]
pub struct CheckAvailabilityParams {
    pub field: Option<String>,
    pub value: String,
}

/// GET /v1/auth/is-active — Check if the current token/user is active (PHP: is-active.php)
pub async fn is_active(
    State(state): State<AppState>,
    opt_auth: shared::auth::OptionalAuth,
) -> Result<Json<Value>, ApiError> {
    match opt_auth.0 {
        Some(user) => {
            let active: bool = sqlx::query_scalar(
                "SELECT is_active FROM users WHERE id = $1 AND deleted_at IS NULL",
            )
            .bind(user.user_id)
            .fetch_optional(&state.db)
            .await?
            .unwrap_or(false);

            Ok(Json(json!({ "data": { "active": active, "user_id": user.user_id } })))
        }
        None => Ok(Json(json!({ "data": { "active": false } }))),
    }
}
