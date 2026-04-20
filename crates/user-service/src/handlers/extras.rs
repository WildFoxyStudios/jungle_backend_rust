use argon2::PasswordVerifier;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::{
    auth::{AppState, AuthUser, OptionalAuth},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;
use uuid::Uuid;

// ── Delete Account (soft delete) ───────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DeleteAccountRequest {
    pub password: String,
}

/// DELETE /v1/users/me — soft-delete the authenticated user's account
pub async fn delete_account(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<DeleteAccountRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Verify password before deletion
    let password_hash: String = sqlx::query_scalar(
        "SELECT password_hash FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("User not found".into()))?;

    let parsed = argon2::PasswordHash::new(&password_hash)
        .map_err(|_| ApiError::Internal("Password hash error".into()))?;
    let valid = argon2::Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed)
        .is_ok();

    if !valid {
        return Err(ApiError::Unauthorized);
    }

    // Soft delete
    sqlx::query("UPDATE users SET deleted_at = NOW(), is_active = FALSE WHERE id = $1")
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    // Revoke all sessions
    sqlx::query("DELETE FROM sessions WHERE user_id = $1")
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "message": "Account deleted" } })))
}

// ── Follow Requests ────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct FollowRequestRow {
    pub id: i64,
    pub follower_id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub created_at: OffsetDateTime,
}

/// GET /v1/social/follow-requests — list pending follow requests
pub async fn list_follow_requests(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let requests = sqlx::query_as::<_, FollowRequestRow>(
        r#"SELECT f.id, f.follower_id, u.username, u.first_name, u.last_name, u.avatar, f.created_at
           FROM follows f
           JOIN users u ON u.id = f.follower_id
           WHERE f.following_id = $1 AND f.status = 'pending' AND f.id < $2
           ORDER BY f.id DESC
           LIMIT $3"#,
    )
    .bind(auth.user_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = requests.len() as i64 > limit;
    let data: Vec<_> = requests.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|r| r.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

/// POST /v1/social/follow-requests/{id}/accept
pub async fn accept_follow_request(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(follow_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "UPDATE follows SET status = 'active' WHERE id = $1 AND following_id = $2 AND status = 'pending'",
    )
    .bind(follow_id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Follow request not found".into()));
    }

    Ok(Json(json!({ "data": { "accepted": true } })))
}

/// POST /v1/social/follow-requests/{id}/reject
pub async fn reject_follow_request(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(follow_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query(
        "DELETE FROM follows WHERE id = $1 AND following_id = $2 AND status = 'pending'",
    )
    .bind(follow_id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Follow request not found".into()));
    }

    Ok(Json(json!({ "data": { "rejected": true } })))
}

// ── Family Relations ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct FamilyRequest {
    pub relation_type: String,
}

/// POST /v1/social/family/{id} — send family relation request
pub async fn send_family_request(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<FamilyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if auth.user_id == id {
        return Err(ApiError::BadRequest("Cannot add yourself as family".into()));
    }

    sqlx::query(
        r#"INSERT INTO family_relations (user_id, relative_id, relation_type, status)
           VALUES ($1, $2, $3, 'pending')
           ON CONFLICT (user_id, relative_id) DO UPDATE SET relation_type = $3, status = 'pending'"#,
    )
    .bind(auth.user_id)
    .bind(id)
    .bind(&req.relation_type)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "status": "pending" } })))
}

#[derive(Debug, Deserialize)]
pub struct FamilyResponse {
    pub action: String, // "accept" or "reject"
}

/// PUT /v1/social/family/{id} — accept or reject family request
pub async fn respond_family_request(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<FamilyResponse>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if req.action == "accept" {
        let result = sqlx::query(
            "UPDATE family_relations SET status = 'active' WHERE id = $1 AND relative_id = $2 AND status = 'pending'",
        )
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Family request not found".into()));
        }
        Ok(Json(json!({ "data": { "accepted": true } })))
    } else {
        sqlx::query(
            "DELETE FROM family_relations WHERE id = $1 AND relative_id = $2 AND status = 'pending'",
        )
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

        Ok(Json(json!({ "data": { "rejected": true } })))
    }
}

// ── User Content Listings ──────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct UserPostRow {
    pub id: i64,
    pub uuid: Uuid,
    pub content: String,
    pub post_type: String,
    pub media: serde_json::Value,
    pub like_count: i32,
    pub comment_count: i32,
    pub created_at: OffsetDateTime,
}

/// GET /v1/users/{username}/posts — list user's posts
pub async fn user_posts(
    State(state): State<AppState>,
    _auth: OptionalAuth,
    Path(username): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id: i64 = sqlx::query_scalar(
        "SELECT id FROM users WHERE LOWER(username) = $1 AND deleted_at IS NULL",
    )
    .bind(username.to_lowercase())
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("User not found".into()))?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let posts = sqlx::query_as::<_, UserPostRow>(
        r#"SELECT id, uuid, content, post_type, media, like_count, comment_count, created_at
           FROM posts
           WHERE user_id = $1 AND deleted_at IS NULL AND is_approved = TRUE
             AND privacy IN ('everyone', 'followers') AND id < $2
           ORDER BY id DESC LIMIT $3"#,
    )
    .bind(user_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = posts.len() as i64 > limit;
    let data: Vec<_> = posts.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|p| p.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

/// GET /v1/users/{username}/photos — list user's photos
pub async fn user_photos(
    State(state): State<AppState>,
    _auth: OptionalAuth,
    Path(username): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id: i64 = sqlx::query_scalar(
        "SELECT id FROM users WHERE LOWER(username) = $1 AND deleted_at IS NULL",
    )
    .bind(username.to_lowercase())
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("User not found".into()))?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    // Photos are posts with media containing image types
    let photos = sqlx::query_as::<_, UserPostRow>(
        r#"SELECT id, uuid, content, post_type, media, like_count, comment_count, created_at
           FROM posts
           WHERE user_id = $1 AND deleted_at IS NULL AND is_approved = TRUE
             AND privacy IN ('everyone', 'followers')
             AND (post_type = 'photo' OR post_type = 'profile_picture' OR post_type = 'cover_picture'
                  OR (post_type = 'media' AND media::text LIKE '%image%'))
             AND id < $2
           ORDER BY id DESC LIMIT $3"#,
    )
    .bind(user_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = photos.len() as i64 > limit;
    let data: Vec<_> = photos.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|p| p.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

/// GET /v1/users/{username}/videos — list user's videos
pub async fn user_videos(
    State(state): State<AppState>,
    _auth: OptionalAuth,
    Path(username): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id: i64 = sqlx::query_scalar(
        "SELECT id FROM users WHERE LOWER(username) = $1 AND deleted_at IS NULL",
    )
    .bind(username.to_lowercase())
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("User not found".into()))?;

    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let videos = sqlx::query_as::<_, UserPostRow>(
        r#"SELECT id, uuid, content, post_type, media, like_count, comment_count, created_at
           FROM posts
           WHERE user_id = $1 AND deleted_at IS NULL AND is_approved = TRUE
             AND privacy IN ('everyone', 'followers')
             AND (post_type = 'video' OR is_reel = TRUE
                  OR (post_type = 'media' AND media::text LIKE '%video%'))
             AND id < $2
           ORDER BY id DESC LIMIT $3"#,
    )
    .bind(user_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = videos.len() as i64 > limit;
    let data: Vec<_> = videos.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|p| p.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

// ── Pro Users & Nearby ─────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct PublicUserMin {
    pub id: i64,
    pub uuid: Uuid,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub is_verified: bool,
    pub is_pro: i16,
}

/// GET /v1/users/pro-users — list pro users
pub async fn pro_users(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let users = sqlx::query_as::<_, PublicUserMin>(
        r#"SELECT id, uuid, username, first_name, last_name, avatar, is_verified, is_pro
           FROM users
           WHERE is_pro > 0 AND deleted_at IS NULL AND is_active = TRUE AND id < $1
           ORDER BY id DESC LIMIT $2"#,
    )
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = users.len() as i64 > limit;
    let data: Vec<_> = users.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|u| u.id.to_string());

    Ok(Json(json!({
        "data": data,
        "meta": { "cursor": next_cursor, "has_more": has_more }
    })))
}

// ── Skills ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct SkillRow {
    pub id: i64,
    pub name: String,
}

/// GET /v1/users/{username}/skills
pub async fn user_skills(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user_id: i64 = sqlx::query_scalar(
        "SELECT id FROM users WHERE LOWER(username) = $1 AND deleted_at IS NULL",
    )
    .bind(username.to_lowercase())
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("User not found".into()))?;

    let skills = sqlx::query_as::<_, SkillRow>(
        "SELECT id, name FROM user_skills WHERE user_id = $1 ORDER BY name ASC",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": skills })))
}

/// GET /v1/skills/search?q= — autocomplete skills
pub async fn search_skills(
    State(state): State<AppState>,
    Query(params): Query<SkillSearchParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let q = format!("{}%", params.q.unwrap_or_default());

    let skills = sqlx::query_as::<_, SkillRow>(
        r#"SELECT DISTINCT id, name FROM user_skills
           WHERE name ILIKE $1
           ORDER BY name ASC LIMIT 20"#,
    )
    .bind(&q)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": skills })))
}

#[derive(Debug, Deserialize)]
pub struct SkillSearchParams {
    pub q: Option<String>,
}

// ── Stop Notify ────────────────────────────────────────────────────

/// POST /v1/social/stop-notify/{user_id} — mute notifications from user
pub async fn stop_notify(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query(
        r#"INSERT INTO mutes (user_id, muted_id, mute_type)
           VALUES ($1, $2, 'notifications')
           ON CONFLICT DO NOTHING"#,
    )
    .bind(auth.user_id)
    .bind(user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "muted_notifications": true } })))
}

// ── Open to Work / Providing Service ───────────────────────────────

/// POST /v1/users/me/open-to-work
pub async fn set_open_to_work(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query(
        "UPDATE users SET privacy_settings = privacy_settings || '{\"open_to_work\": true}'::jsonb WHERE id = $1",
    )
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "open_to_work": true } })))
}

/// DELETE /v1/users/me/open-to-work
pub async fn unset_open_to_work(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query(
        "UPDATE users SET privacy_settings = privacy_settings || '{\"open_to_work\": false}'::jsonb WHERE id = $1",
    )
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "open_to_work": false } })))
}

/// POST /v1/users/me/providing-service
pub async fn set_providing_service(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query(
        "UPDATE users SET privacy_settings = privacy_settings || '{\"providing_service\": true}'::jsonb WHERE id = $1",
    )
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "providing_service": true } })))
}

/// DELETE /v1/users/me/providing-service
pub async fn unset_providing_service(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query(
        "UPDATE users SET privacy_settings = privacy_settings || '{\"providing_service\": false}'::jsonb WHERE id = $1",
    )
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "providing_service": false } })))
}

// ── Nearby Users (location-based) ────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct NearbyParams {
    pub lat: f64,
    pub lng: f64,
    pub radius_km: Option<f64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct NearbyUserRow {
    pub id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub is_verified: bool,
    pub distance_km: f64,
}

/// GET /v1/users/nearby — find users within a radius using Haversine formula
pub async fn nearby_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<NearbyParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let radius = params.radius_km.unwrap_or(50.0);
    let limit = params.limit.unwrap_or(20).clamp(1, 100);

    let users = sqlx::query_as::<_, NearbyUserRow>(
        r#"
        SELECT * FROM (
            SELECT id, username, first_name, last_name, avatar, is_verified,
                   (6371 * acos(LEAST(1.0,
                        cos(radians($1)) * cos(radians(lat)) *
                        cos(radians(lng) - radians($2)) +
                        sin(radians($1)) * sin(radians(lat))
                   ))) AS distance_km
            FROM users
            WHERE deleted_at IS NULL AND is_active = TRUE
              AND lat IS NOT NULL AND lng IS NOT NULL
              AND id != $3
        ) sub
        WHERE distance_km <= $4
        ORDER BY distance_km ASC
        LIMIT $5
        "#,
    )
    .bind(params.lat)
    .bind(params.lng)
    .bind(auth.user_id)
    .bind(radius)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": users })))
}

// ── Skills CRUD ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AddSkillRequest {
    pub skill: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct UserSkillRow {
    pub id: i64,
    pub user_id: i64,
    pub skill: String,
    pub created_at: OffsetDateTime,
}

/// POST /v1/users/me/skills — add a skill to the current user
pub async fn add_skill(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AddSkillRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let trimmed = req.skill.trim();
    if trimmed.is_empty() || trimmed.len() > 100 {
        return Err(ApiError::BadRequest("Skill must be 1-100 characters".into()));
    }

    let row = sqlx::query_as::<_, UserSkillRow>(
        r#"INSERT INTO user_skills (user_id, skill)
           VALUES ($1, $2)
           ON CONFLICT (user_id, skill) DO UPDATE SET skill = EXCLUDED.skill
           RETURNING *"#,
    )
    .bind(auth.user_id)
    .bind(trimmed)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": row })))
}

/// DELETE /v1/users/me/skills/{id} — remove a skill
pub async fn remove_skill(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let result = sqlx::query("DELETE FROM user_skills WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Skill not found".into()));
    }

    Ok(Json(json!({ "data": { "deleted": true } })))
}

// ── Reset Avatar ─────────────────────────────────────────────────

/// POST /v1/users/me/avatar/reset — reset avatar to default
pub async fn reset_avatar(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("UPDATE users SET avatar = 'default-avatar.jpg' WHERE id = $1")
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "avatar": "default-avatar.jpg" } })))
}

// ── Custom Profile Field Values ──────────────────────────────────

#[derive(Debug, Serialize, FromRow)]
pub struct UserFieldValueRow {
    pub field_id: i64,
    pub field_name: String,
    pub field_type: String,
    pub value: String,
}

/// GET /v1/users/me/fields — list current user's custom profile field values
pub async fn get_my_field_values(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let values = sqlx::query_as::<_, UserFieldValueRow>(
        r#"
        SELECT pf.id AS field_id, pf.name AS field_name, pf.field_type,
               COALESCE(ufv.value, '') AS value
        FROM profile_fields pf
        LEFT JOIN user_field_values ufv ON ufv.field_id = pf.id AND ufv.user_id = $1
        WHERE pf.is_active = true
        ORDER BY pf.sort_order ASC, pf.id ASC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": values })))
}

/// GET /v1/users/{user_id}/fields — list a user's custom profile field values
pub async fn get_user_field_values(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let values = sqlx::query_as::<_, UserFieldValueRow>(
        r#"
        SELECT pf.id AS field_id, pf.name AS field_name, pf.field_type,
               COALESCE(ufv.value, '') AS value
        FROM profile_fields pf
        LEFT JOIN user_field_values ufv ON ufv.field_id = pf.id AND ufv.user_id = $1
        WHERE pf.is_active = true
        ORDER BY pf.sort_order ASC, pf.id ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": values })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateFieldValue {
    pub field_id: i64,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFieldValuesRequest {
    pub fields: Vec<UpdateFieldValue>,
}

/// POST /v1/users/me/download-info — GDPR data export
pub async fn download_my_info(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<DownloadInfoRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let valid_types = ["posts", "pages", "groups", "followers", "following", "my_information", "friends"];
    let requested: Vec<&str> = req.data.iter().map(|s| s.as_str()).filter(|s| valid_types.contains(s)).collect();

    if requested.is_empty() {
        return Err(ApiError::BadRequest("Provide at least one valid data type".into()));
    }

    let mut result = serde_json::Map::new();

    for dtype in &requested {
        match *dtype {
            "my_information" => {
                let row: Option<serde_json::Value> = sqlx::query_scalar(
                    "SELECT row_to_json(u.*) FROM (SELECT id, username, first_name, last_name, email, gender, birthday, about, website, country, city, created_at FROM users WHERE id = $1) u",
                )
                .bind(auth.user_id)
                .fetch_optional(&state.db)
                .await?;
                result.insert("my_information".into(), row.unwrap_or(serde_json::Value::Null));
            }
            "posts" => {
                let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE user_id = $1 AND deleted_at IS NULL")
                    .bind(auth.user_id)
                    .fetch_one(&state.db)
                    .await?;
                result.insert("posts_count".into(), json!(count));
            }
            "followers" => {
                let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM follows WHERE following_id = $1")
                    .bind(auth.user_id)
                    .fetch_one(&state.db)
                    .await?;
                result.insert("followers_count".into(), json!(count));
            }
            "following" => {
                let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM follows WHERE follower_id = $1")
                    .bind(auth.user_id)
                    .fetch_one(&state.db)
                    .await?;
                result.insert("following_count".into(), json!(count));
            }
            "friends" => {
                let count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM follows f1 JOIN follows f2 ON f1.follower_id = f2.following_id AND f1.following_id = f2.follower_id WHERE f1.follower_id = $1",
                )
                .bind(auth.user_id)
                .fetch_one(&state.db)
                .await?;
                result.insert("friends_count".into(), json!(count));
            }
            "pages" => {
                let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pages WHERE user_id = $1 AND deleted_at IS NULL")
                    .bind(auth.user_id)
                    .fetch_one(&state.db)
                    .await?;
                result.insert("pages_count".into(), json!(count));
            }
            "groups" => {
                let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM group_members WHERE user_id = $1")
                    .bind(auth.user_id)
                    .fetch_one(&state.db)
                    .await?;
                result.insert("groups_count".into(), json!(count));
            }
            _ => {}
        }
    }

    Ok(Json(json!({ "data": result })))
}

#[derive(Debug, Deserialize)]
pub struct DownloadInfoRequest {
    pub data: Vec<String>,
}

/// GET /v1/users/{user_id}/common — Users with common attributes (mutual friends, same city, etc)
pub async fn common_things(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if auth.user_id == user_id {
        return Err(ApiError::BadRequest("Cannot compare with yourself".into()));
    }

    // Mutual friends count
    let mutual_friends: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM follows f1
           JOIN follows f2 ON f1.following_id = f2.following_id
           WHERE f1.follower_id = $1 AND f2.follower_id = $2 AND f1.following_id != $1 AND f1.following_id != $2"#,
    )
    .bind(auth.user_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    // Same city
    let same_city: bool = sqlx::query_scalar::<_, bool>(
        "SELECT u1.city = u2.city AND u1.city != '' FROM users u1, users u2 WHERE u1.id = $1 AND u2.id = $2",
    )
    .bind(auth.user_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(false);

    // Same country
    let same_country: bool = sqlx::query_scalar::<_, bool>(
        "SELECT u1.country = u2.country AND u1.country != '' FROM users u1, users u2 WHERE u1.id = $1 AND u2.id = $2",
    )
    .bind(auth.user_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or(false);

    // Mutual groups
    let mutual_groups: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM group_members g1 JOIN group_members g2 ON g1.group_id = g2.group_id WHERE g1.user_id = $1 AND g2.user_id = $2",
    )
    .bind(auth.user_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    // Mutual liked pages
    let mutual_pages: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM page_likes p1 JOIN page_likes p2 ON p1.page_id = p2.page_id WHERE p1.user_id = $1 AND p2.user_id = $2",
    )
    .bind(auth.user_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({
        "data": {
            "mutual_friends": mutual_friends,
            "mutual_groups": mutual_groups,
            "mutual_pages": mutual_pages,
            "same_city": same_city,
            "same_country": same_country
        }
    })))
}

/// POST /v1/reports — Unified report endpoint for user/page/group/comment/post
/// Matches PHP: report_user.php, report_page.php, report_group.php, report_comment.php
#[derive(Debug, Deserialize)]
pub struct CreateReportRequest {
    pub target_type: String,
    pub target_id: i64,
    pub reason: String,
    pub description: Option<String>,
}

pub async fn create_report(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateReportRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let valid_types = ["user", "post", "page", "group", "comment", "blog", "movie", "product"];
    if !valid_types.contains(&req.target_type.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "target_type must be one of: {}",
            valid_types.join(", ")
        )));
    }
    if req.reason.trim().is_empty() {
        return Err(ApiError::BadRequest("reason is required".into()));
    }

    // Prevent duplicate pending reports
    let already: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM reports WHERE reporter_id = $1 AND target_type = $2 AND target_id = $3 AND status = 'pending')",
    )
    .bind(auth.user_id)
    .bind(req.target_type.trim())
    .bind(req.target_id)
    .fetch_one(&state.db)
    .await?;

    if already {
        return Err(ApiError::BadRequest("You already have a pending report for this item".into()));
    }

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO reports (reporter_id, target_type, target_id, reason, description, status) VALUES ($1, $2, $3, $4, $5, 'pending') RETURNING id",
    )
    .bind(auth.user_id)
    .bind(req.target_type.trim())
    .bind(req.target_id)
    .bind(req.reason.trim())
    .bind(req.description.as_deref().unwrap_or(""))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id, "submitted": true } })))
}

/// POST /v1/points/admob — Record AdMob ad view and award points
/// Matches PHP: admob.php — user watches an ad to earn points
pub async fn record_admob_points(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let config = sqlx::query_as::<_, (bool, i64, i64, i64)>(
        r#"SELECT
            COALESCE((SELECT value::boolean FROM site_config WHERE category='features' AND key='points_system'), false),
            COALESCE((SELECT value::bigint FROM site_config WHERE category='points' AND key='admob_point'), 0),
            COALESCE((SELECT value::bigint FROM site_config WHERE category='points' AND key='free_day_limit'), 100),
            COALESCE((SELECT value::bigint FROM site_config WHERE category='points' AND key='pro_day_limit'), 500)"#,
    )
    .fetch_optional(&state.db)
    .await?
    .unwrap_or((false, 0, 100, 500));

    let (enabled, admob_pts, free_limit, _pro_limit) = config;

    if !enabled || admob_pts == 0 {
        return Ok(Json(json!({ "data": { "awarded": false, "message": "Points system is disabled" } })));
    }

    let today = time::OffsetDateTime::now_utc().date();
    let daily_key = format!("admob_points:{}:{}", auth.user_id, today);

    let mut redis = state.redis.clone();

    let current_today: i64 = redis.get(&daily_key).await.unwrap_or(0);

    if current_today + admob_pts > free_limit {
        return Ok(Json(json!({ "data": { "awarded": false, "message": "Daily points limit reached" } })));
    }

    sqlx::query("UPDATE users SET points = points + $1 WHERE id = $2")
        .bind(admob_pts)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    let _: Result<i64, _> = redis.incr(&daily_key, admob_pts).await;
    let _: Result<(), _> = redis.expire(&daily_key, 86400).await;

    Ok(Json(json!({ "data": { "awarded": true, "points_awarded": admob_pts } })))
}

/// PUT /v1/users/me/fields — bulk update custom profile field values
pub async fn update_my_field_values(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateFieldValuesRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut tx = state.db.begin().await?;

    for field in &req.fields {
        sqlx::query(
            r#"
            INSERT INTO user_field_values (user_id, field_id, value)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, field_id) DO UPDATE SET value = EXCLUDED.value
            "#,
        )
        .bind(auth.user_id)
        .bind(field.field_id)
        .bind(&field.value)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Json(json!({ "data": { "updated": req.fields.len() } })))
}

// ── Batch Users ──────────────────────────────────────────────────────────────

/// POST /v1/users/batch — Get multiple users by IDs (PHP: get-many-users-data.php)
#[derive(Debug, Deserialize)]
pub struct BatchUsersRequest {
    pub user_ids: Vec<i64>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct PublicUserRow {
    pub id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub cover: String,
    pub is_verified: bool,
    pub is_pro: i16,
    pub is_online: bool,
    pub last_seen: OffsetDateTime,
}

pub async fn batch_users(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<BatchUsersRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if req.user_ids.is_empty() || req.user_ids.len() > 100 {
        return Err(ApiError::BadRequest("user_ids: 1–100 IDs required".into()));
    }

    let users = sqlx::query_as::<_, PublicUserRow>(
        "SELECT id, username, first_name, last_name, avatar, cover,
                is_verified, is_pro, is_online, last_seen
         FROM users WHERE id = ANY($1) AND deleted_at IS NULL",
    )
    .bind(&req.user_ids)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": users })))
}

/// GET /v1/users/by-phone?phone=+1234567890 — Look up user by phone (PHP: get-user-data-phone.php)
pub async fn get_user_by_phone(
    State(state): State<AppState>,
    _auth: AuthUser,
    axum::extract::Query(params): axum::extract::Query<ByPhoneParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if params.phone.trim().is_empty() {
        return Err(ApiError::BadRequest("phone is required".into()));
    }

    let user = sqlx::query_as::<_, PublicUserRow>(
        "SELECT id, username, first_name, last_name, avatar, cover,
                is_verified, is_pro, is_online, last_seen
         FROM users WHERE phone_number = $1 AND deleted_at IS NULL",
    )
    .bind(params.phone.trim())
    .fetch_optional(&state.db)
    .await?;

    match user {
        Some(u) => Ok(Json(json!({ "data": u }))),
        None => Err(ApiError::NotFound("User not found".into())),
    }
}

#[derive(Debug, Deserialize)]
pub struct ByPhoneParams {
    pub phone: String,
}

/// PUT /v1/users/me/lastseen — Update last seen / presence (PHP: update_lastseen.php)
pub async fn update_lastseen(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("UPDATE users SET last_seen = NOW(), is_online = TRUE WHERE id = $1")
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "updated": true } })))
}

/// GET /v1/users/me/referrals — Get users I referred (PHP: get_referrers.php)
pub async fn my_referrals(
    State(state): State<AppState>,
    auth: AuthUser,
    axum::extract::Query(params): axum::extract::Query<PaginationParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = params.limit();
    let cursor_id = params.cursor_id().unwrap_or(i64::MAX);

    let rows = sqlx::query_as::<_, PublicUserRow>(
        r#"SELECT u.id, u.username, u.first_name, u.last_name, u.avatar, u.cover,
                  u.is_verified, u.is_pro, u.is_online, u.last_seen
           FROM invitation_links il
           JOIN users u ON u.id = il.used_by
           WHERE il.user_id = $1 AND il.used_by IS NOT NULL AND u.id < $2
           ORDER BY u.id DESC LIMIT $3"#,
    )
    .bind(auth.user_id)
    .bind(cursor_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = rows.len() as i64 > limit;
    let data: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = data.last().map(|r| r.id.to_string());

    Ok(Json(json!({ "data": data, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

/// GET /v1/users/me/inviters — Get users who invited me (PHP: get_invites.php)
pub async fn my_inviters(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = sqlx::query_as::<_, PublicUserRow>(
        r#"SELECT u.id, u.username, u.first_name, u.last_name, u.avatar, u.cover,
                  u.is_verified, u.is_pro, u.is_online, u.last_seen
           FROM invitation_links il
           JOIN users u ON u.id = il.user_id
           WHERE il.used_by = $1
           ORDER BY u.id DESC LIMIT 50"#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": rows })))
}

/// POST /v1/users/me/onboarding/skip — Skip onboarding step (PHP: skip_step.php)
#[derive(Debug, Deserialize)]
pub struct SkipStepRequest {
    pub step: String,
}

pub async fn skip_onboarding_step(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SkipStepRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let valid = ["start_up_info", "startup_image", "startup_follow"];
    if !valid.contains(&req.step.as_str()) {
        return Err(ApiError::BadRequest(format!("step must be one of: {}", valid.join(", "))));
    }

    // Store skip status in privacy_settings JSONB to avoid dynamic SQL
    sqlx::query(
        "UPDATE users SET privacy_settings = privacy_settings || jsonb_build_object($1, true) WHERE id = $2",
    )
    .bind(format!("skipped_{}", req.step))
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "skipped": true } })))
}

/// POST /v1/search/recent — Register a recent search entry (PHP: register_recent_search.php)
#[derive(Debug, Deserialize)]
pub struct RegisterRecentSearchRequest {
    pub search_type: String,
    pub target_id: i64,
}

pub async fn register_recent_search(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<RegisterRecentSearchRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let valid_types = ["user", "page", "group", "hashtag", "product"];
    if !valid_types.contains(&req.search_type.as_str()) {
        return Err(ApiError::BadRequest("invalid search_type".into()));
    }

    sqlx::query(
        "INSERT INTO recent_searches (user_id, search_type, target_id, searched_at)
         VALUES ($1, $2, $3, NOW())
         ON CONFLICT DO NOTHING",
    )
    .bind(auth.user_id)
    .bind(req.search_type.trim())
    .bind(req.target_id)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "registered": true } })))
}

/// POST /v1/general — Batch data fetch for mobile app startup (PHP: get-general-data.php)
/// Returns requested data types in a single request: notifications, friend_requests,
/// messages_count, trending_hashtags, announcements, pro_users, promoted content
#[derive(Debug, Deserialize)]
pub struct GeneralDataRequest {
    pub fetch: Vec<String>,
    pub android_device_id: Option<String>,
    pub ios_device_id: Option<String>,
}

pub async fn get_general_data(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<GeneralDataRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if req.fetch.is_empty() {
        return Err(ApiError::BadRequest("fetch array is required".into()));
    }

    // Update device ID if provided
    if let Some(ref android_id) = req.android_device_id
        && !android_id.trim().is_empty()
    {
        let _ = sqlx::query("UPDATE users SET android_device_id = $1 WHERE id = $2")
            .bind(android_id.trim())
            .bind(auth.user_id)
            .execute(&state.db)
            .await;
    }
    if let Some(ref ios_id) = req.ios_device_id
        && !ios_id.trim().is_empty()
    {
        let _ = sqlx::query("UPDATE users SET ios_device_id = $1 WHERE id = $2")
            .bind(ios_id.trim())
            .bind(auth.user_id)
            .execute(&state.db)
            .await;
    }

    let mut result = serde_json::Map::new();

    for fetch_type in &req.fetch {
        match fetch_type.as_str() {
            "notifications" => {
                let count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM notifications WHERE recipient_id = $1 AND is_read = FALSE",
                )
                .bind(auth.user_id)
                .fetch_one(&state.db)
                .await
                .unwrap_or(0);
                result.insert("new_notifications_count".into(), json!(count));
            }
            "messages" => {
                let count: i64 = sqlx::query_scalar(
                    r#"SELECT COALESCE(SUM(
                        (SELECT COUNT(*) FROM messages m
                         WHERE m.conversation_id = cm.conversation_id
                           AND m.created_at > cm.last_read_at
                           AND m.sender_id != $1
                           AND m.deleted_at IS NULL)
                    ), 0)
                    FROM conversation_members cm
                    WHERE cm.user_id = $1 AND cm.is_active = TRUE"#,
                )
                .bind(auth.user_id)
                .fetch_one(&state.db)
                .await
                .unwrap_or(0);
                result.insert("count_new_messages".into(), json!(count));
            }
            "follow_requests" | "friend_requests" => {
                let count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM follows WHERE following_id = $1 AND status = 'pending'",
                )
                .bind(auth.user_id)
                .fetch_one(&state.db)
                .await
                .unwrap_or(0);
                result.insert("new_friend_requests_count".into(), json!(count));
            }
            "trending_hashtags" => {
                let tags = sqlx::query_as::<_, (i64, String, i32)>(
                    "SELECT id, tag, use_count FROM hashtags WHERE trending = TRUE ORDER BY use_count DESC LIMIT 10",
                )
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();
                let tag_data: Vec<_> = tags.into_iter()
                    .map(|(id, tag, count)| json!({"id": id, "tag": tag, "use_count": count}))
                    .collect();
                result.insert("trending_hashtag".into(), json!(tag_data));
            }
            "announcements" => {
                let items = sqlx::query_as::<_, (i64, String)>(
                    "SELECT id, content FROM announcements WHERE active = TRUE ORDER BY id DESC LIMIT 5",
                )
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();
                let ann_data: Vec<_> = items.into_iter()
                    .map(|(id, content)| json!({"id": id, "content": content}))
                    .collect();
                result.insert("announcement".into(), json!(ann_data));
            }
            "pro_users" => {
                let users = sqlx::query_as::<_, (i64, String, String, String, String, bool, i16)>(
                    "SELECT id, username, first_name, last_name, avatar, is_verified, is_pro FROM users WHERE is_pro > 0 AND deleted_at IS NULL ORDER BY RANDOM() LIMIT 10",
                )
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();
                let user_data: Vec<_> = users.into_iter()
                    .map(|(id, username, first_name, last_name, avatar, is_verified, is_pro)| {
                        json!({"id": id, "username": username, "first_name": first_name, "last_name": last_name, "avatar": avatar, "is_verified": is_verified, "is_pro": is_pro})
                    }).collect();
                result.insert("pro_users".into(), json!(user_data));
            }
            "promoted_pages" => {
                let pages = sqlx::query_as::<_, (i64, String, String, String, i32)>(
                    "SELECT page_id, page_name, page_title, avatar, like_count FROM pages WHERE is_boosted = TRUE AND active = TRUE LIMIT 5",
                )
                .fetch_all(&state.db)
                .await
                .unwrap_or_default();
                let page_data: Vec<_> = pages.into_iter()
                    .map(|(id, name, title, avatar, likes)| {
                        json!({"id": id, "page_name": name, "page_title": title, "avatar": avatar, "like_count": likes})
                    }).collect();
                result.insert("promoted_pages".into(), json!(page_data));
            }
            _ => {}
        }
    }

    Ok(Json(json!({ "data": result })))
}

/// POST /v1/contact — Contact form (PHP: contact_us.php)
#[derive(Debug, Deserialize)]
pub struct ContactRequest {
    pub name: String,
    pub email: String,
    pub message: String,
    pub subject: Option<String>,
}

pub async fn contact_us(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<ContactRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if req.name.trim().is_empty() || req.email.trim().is_empty() || req.message.trim().is_empty() {
        return Err(ApiError::BadRequest("name, email, message are required".into()));
    }

    // Store in sent_emails as a contact message
    sqlx::query(
        "INSERT INTO sent_emails (subject, body, recipient_count, sent_by)
         VALUES ($1, $2, 0, NULL)",
    )
    .bind(format!("Contact: {}", req.subject.as_deref().unwrap_or(&req.name)))
    .bind(format!("From: {} <{}>\n\n{}", req.name.trim(), req.email.trim(), req.message.trim()))
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "sent": true } })))
}

/// POST /v1/users/me/verification-request — Submit identity verification docs (PHP: verificate-user.php)
pub async fn request_verification(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let message = req["message"].as_str().unwrap_or("").trim().to_string();
    let document_url = req["document_url"].as_str().unwrap_or("").trim().to_string();
    let full_name = req["full_name"].as_str().unwrap_or("").trim().to_string();

    if document_url.is_empty() {
        return Err(ApiError::BadRequest("Document photo is required".into()));
    }

    sqlx::query(
        r#"INSERT INTO verification_requests (user_id, full_name, message, document_url, status, created_at)
        VALUES ($1, $2, $3, $4, 'pending', NOW())
        ON CONFLICT (user_id) DO UPDATE SET full_name = $2, message = $3, document_url = $4, status = 'pending'"#,
    )
    .bind(auth.user_id)
    .bind(&full_name)
    .bind(&message)
    .bind(&document_url)
    .execute(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "submitted": true, "status": "pending" } })))
}
