use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    models::{AuthUserResponse, PublicUser, User},
};
use validator::Validate;

pub async fn get_me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("User not found".into()))?;

    let resp = AuthUserResponse::from(&user);
    Ok(Json(serde_json::json!({ "data": resp })))
}

pub async fn get_user(
    State(state): State<AppState>,
    auth: shared::auth::OptionalAuth,
    Path(username): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE LOWER(username) = $1 AND deleted_at IS NULL AND is_active = TRUE",
    )
    .bind(username.to_lowercase())
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::NotFound("User not found".into()))?;

    // Check if blocked
    if let Some(ref viewer) = auth.0 {
        let blocked: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM blocks WHERE blocker_id = $1 AND blocked_id = $2)",
        )
        .bind(user.id)
        .bind(viewer.user_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(false);

        if blocked {
            return Err(ApiError::NotFound("User not found".into()));
        }
    }

    let mut public = PublicUser::from(&user);

    // Check online status
    let time_threshold = time::OffsetDateTime::now_utc() - time::Duration::seconds(60);
    public.is_online = user.last_seen.map(|ls| ls > time_threshold).unwrap_or(false);

    // Follower/following counts
    let follower_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM follows WHERE following_id = $1 AND status = 'active'",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let following_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM follows WHERE follower_id = $1 AND status = 'active'",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    // Is the viewer following this user?
    let is_following = if let Some(ref viewer) = auth.0 {
        let val: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM follows WHERE follower_id = $1 AND following_id = $2 AND status = 'active')",
        )
        .bind(viewer.user_id)
        .bind(user.id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(false);
        val
    } else {
        false
    };

    let is_following_me = if let Some(ref viewer) = auth.0 {
        let val: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM follows WHERE follower_id = $1 AND following_id = $2 AND status = 'active')",
        )
        .bind(user.id)
        .bind(viewer.user_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(false);
        val
    } else {
        false
    };

    Ok(Json(serde_json::json!({
        "data": {
            "user": public,
            "follower_count": follower_count,
            "following_count": following_count,
            "is_following": is_following,
            "is_following_me": is_following_me,
        }
    })))
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileRequest {
    #[validate(length(min = 3, max = 30))]
    pub username: Option<String>,
    #[validate(length(max = 50))]
    pub first_name: Option<String>,
    #[validate(length(max = 50))]
    pub last_name: Option<String>,
    #[validate(length(max = 500))]
    pub about: Option<String>,
    pub gender: Option<String>,
    pub birthday: Option<String>,
    #[validate(length(max = 100))]
    pub city: Option<String>,
    #[validate(length(max = 100))]
    pub location: Option<String>,
    #[validate(length(max = 255))]
    pub website: Option<String>,
    #[validate(length(max = 200))]
    pub school: Option<String>,
    #[validate(length(max = 200))]
    pub working: Option<String>,
    pub language: Option<String>,
}

pub async fn update_me(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    req.validate()?;

    // Username uniqueness check
    if let Some(ref new_username) = req.username {
        let taken: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM users WHERE LOWER(username) = $1 AND id != $2)",
        )
        .bind(new_username.to_lowercase())
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(false);

        if taken {
            return Err(ApiError::Conflict("Username is already taken".into()));
        }
    }

    // Accept both "location" and "city" from the frontend
    let city_value = req.location.as_ref().or(req.city.as_ref());

    let user = sqlx::query_as::<_, User>(
        r#"UPDATE users SET
            username = COALESCE($2, username),
            first_name = COALESCE($3, first_name),
            last_name = COALESCE($4, last_name),
            about = COALESCE($5, about),
            gender = COALESCE($6, gender),
            birthday = COALESCE($7, birthday),
            city = COALESCE($8, city),
            website = COALESCE($9, website),
            school = COALESCE($10, school),
            working = COALESCE($11, working),
            language = COALESCE($12, language),
            updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING *"#,
    )
    .bind(auth.user_id)
    .bind(&req.username)
    .bind(&req.first_name)
    .bind(&req.last_name)
    .bind(&req.about)
    .bind(&req.gender)
    .bind(&req.birthday)
    .bind(city_value)
    .bind(&req.website)
    .bind(&req.school)
    .bind(&req.working)
    .bind(&req.language)
    .fetch_one(&state.db)
    .await?;

    let resp = AuthUserResponse::from(&user);
    Ok(Json(serde_json::json!({ "data": resp })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateAvatarRequest {
    pub avatar_url: String,
}

pub async fn update_avatar(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateAvatarRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("UPDATE users SET avatar = $1, updated_at = NOW() WHERE id = $2")
        .bind(&req.avatar_url)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "data": { "avatar": req.avatar_url }
    })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateCoverRequest {
    pub cover_url: String,
}

pub async fn update_cover(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateCoverRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    sqlx::query("UPDATE users SET cover = $1, updated_at = NOW() WHERE id = $2")
        .bind(&req.cover_url)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "data": { "cover": req.cover_url }
    })))
}

/// PUT /v1/users/me/social-links — Update public social profile links (PHP: update_socialinks_setting.php)
#[derive(Debug, Deserialize, Serialize)]
pub struct SocialLinksRequest {
    pub facebook:  Option<String>,
    pub twitter:   Option<String>,
    pub linkedin:  Option<String>,
    pub instagram: Option<String>,
    pub youtube:   Option<String>,
    pub github:    Option<String>,
    pub vk:        Option<String>,
    pub tiktok:    Option<String>,
    pub website:   Option<String>,
}

pub async fn update_social_links(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SocialLinksRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let links = serde_json::json!({
        "facebook":  req.facebook.as_deref().unwrap_or(""),
        "twitter":   req.twitter.as_deref().unwrap_or(""),
        "linkedin":  req.linkedin.as_deref().unwrap_or(""),
        "instagram": req.instagram.as_deref().unwrap_or(""),
        "youtube":   req.youtube.as_deref().unwrap_or(""),
        "github":    req.github.as_deref().unwrap_or(""),
        "vk":        req.vk.as_deref().unwrap_or(""),
        "tiktok":    req.tiktok.as_deref().unwrap_or(""),
        "website":   req.website.as_deref().unwrap_or("")
    });

    sqlx::query(
        "UPDATE users SET social_links = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(&links)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    // Also update the website column (for backward compat)
    if let Some(ref site) = req.website {
        sqlx::query("UPDATE users SET website = $1 WHERE id = $2")
            .bind(site.as_str())
            .bind(auth.user_id)
            .execute(&state.db)
            .await?;
    }

    Ok(Json(serde_json::json!({ "data": links })))
}

/// GET /v1/users/{username}/social-links — Get public social links of any user
pub async fn get_social_links(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(username): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let links: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT social_links FROM users WHERE username = $1 AND deleted_at IS NULL",
    )
    .bind(&username)
    .fetch_optional(&state.db)
    .await?
    .flatten();

    Ok(Json(serde_json::json!({ "data": links.unwrap_or(serde_json::json!({})) })))
}

/// GET /v1/users/{username}/popover
///
/// Optimized lightweight response for hover cards. Returns only the fields
/// required by the UI (~70% smaller than `get_user`).
pub async fn get_user_popover(
    State(state): State<AppState>,
    auth: shared::auth::OptionalAuth,
    Path(username): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    type PopRow = (
        i64,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        bool,
        i64,
        i64,
        i64,
    );

    let me_id: Option<i64> = auth.0.as_ref().map(|u| u.user_id);

    let row: Option<PopRow> = sqlx::query_as(
        r#"SELECT u.id, u.username, u.first_name, u.last_name, u.avatar,
                  u.about, COALESCE(u.is_verified, FALSE),
                  (SELECT COUNT(*) FROM follows f WHERE f.following_id = u.id AND f.status = 'active') AS follower_count,
                  (SELECT COUNT(*) FROM follows f WHERE f.follower_id = u.id AND f.status = 'active') AS following_count,
                  (SELECT COUNT(*) FROM posts p WHERE p.user_id = u.id AND p.deleted_at IS NULL) AS post_count
             FROM users u
            WHERE LOWER(u.username) = $1
              AND u.deleted_at IS NULL
              AND u.is_active = TRUE"#,
    )
    .bind(username.to_lowercase())
    .fetch_optional(&state.db)
    .await?;

    let (id, username, first_name, last_name, avatar, about, is_verified, followers, following, posts) =
        row.ok_or(ApiError::NotFound("User not found".into()))?;

    let is_following = if let Some(my_id) = me_id {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM follows WHERE follower_id = $1 AND following_id = $2 AND status = 'active')",
        )
        .bind(my_id)
        .bind(id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(false)
    } else {
        false
    };

    Ok(Json(serde_json::json!({
        "data": {
            "id": id,
            "username": username,
            "first_name": first_name,
            "last_name": last_name,
            "avatar": avatar,
            "about": about,
            "is_verified": is_verified,
            "follower_count": followers,
            "following_count": following,
            "post_count": posts,
            "is_following": is_following,
        }
    })))
}
