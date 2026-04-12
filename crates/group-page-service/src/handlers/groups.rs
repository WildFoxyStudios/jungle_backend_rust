use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    events::DomainEvent,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateGroupRequest {
    #[validate(length(min = 3, max = 32))]
    pub group_name: String,
    #[validate(length(min = 1, max = 100))]
    pub group_title: String,
    pub about: Option<String>,
    pub category_id: Option<i64>,
    pub privacy: Option<String>,
    pub join_privacy: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupRequest {
    pub group_title: Option<String>,
    pub about: Option<String>,
    pub category_id: Option<i64>,
    pub privacy: Option<String>,
    pub join_privacy: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangeRoleRequest {
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

#[derive(Debug, Serialize, FromRow)]
pub struct GroupRow {
    pub id: i64,
    pub uuid: uuid::Uuid,
    pub user_id: i64,
    pub group_name: String,
    pub group_title: String,
    pub avatar: String,
    pub cover: String,
    pub about: String,
    pub category_id: Option<i64>,
    pub privacy: String,
    pub join_privacy: String,
    pub active: bool,
    pub member_count: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct GroupSummary {
    pub id: i64,
    pub group_name: String,
    pub group_title: String,
    pub avatar: String,
    pub privacy: String,
    pub member_count: i32,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MemberRow {
    pub user_id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub role: String,
    pub status: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct JoinRequestRow {
    pub id: i64,
    pub user_id: i64,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CategoryRow {
    pub id: i64,
    pub name_key: String,
    pub slug: Option<String>,
    pub parent_id: Option<i64>,
}

pub async fn create_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateGroupRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(ApiError::from)?;

    let mut tx = state.db.begin().await?;

    let group = sqlx::query_as::<_, GroupRow>(
        r#"
        INSERT INTO groups (user_id, group_name, group_title, about, category_id, privacy, join_privacy)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(auth.user_id)
    .bind(&req.group_name)
    .bind(&req.group_title)
    .bind(req.about.as_deref().unwrap_or(""))
    .bind(req.category_id)
    .bind(req.privacy.as_deref().unwrap_or("public"))
    .bind(req.join_privacy.as_deref().unwrap_or("open"))
    .fetch_one(&mut *tx)
    .await?;

    // Add creator as owner
    sqlx::query("INSERT INTO group_members (group_id, user_id, role, status) VALUES ($1, $2, 'owner', 'active')")
        .bind(group.id)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE groups SET member_count = 1 WHERE id = $1")
        .bind(group.id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(json!({ "data": group })))
}

pub async fn get_group(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let group = sqlx::query_as::<_, GroupRow>("SELECT * FROM groups WHERE group_name = $1 AND active = TRUE")
        .bind(&slug)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Group not found".into()))?;

    Ok(Json(json!({ "data": group })))
}

pub async fn update_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateGroupRequest>,
) -> Result<Json<Value>, ApiError> {
    verify_group_admin(&state, id, auth.user_id).await?;

    let group = sqlx::query_as::<_, GroupRow>(
        r#"
        UPDATE groups SET
            group_title = COALESCE($2, group_title),
            about = COALESCE($3, about),
            category_id = COALESCE($4, category_id),
            privacy = COALESCE($5, privacy),
            join_privacy = COALESCE($6, join_privacy),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.group_title)
    .bind(&req.about)
    .bind(req.category_id)
    .bind(&req.privacy)
    .bind(&req.join_privacy)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": group })))
}

pub async fn delete_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM groups WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Group not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    sqlx::query("UPDATE groups SET active = FALSE, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn join_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let group = sqlx::query_as::<_, GroupRow>("SELECT * FROM groups WHERE id = $1 AND active = TRUE")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Group not found".into()))?;

    let status = if group.join_privacy == "open" { "active" } else { "pending" };

    sqlx::query(
        "INSERT INTO group_members (group_id, user_id, role, status) VALUES ($1, $2, 'member', $3) ON CONFLICT (group_id, user_id) DO UPDATE SET status = $3",
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(status)
    .execute(&state.db)
    .await?;

    if status == "active" {
        sqlx::query("UPDATE groups SET member_count = (SELECT COUNT(*) FROM group_members WHERE group_id = $1 AND status = 'active') WHERE id = $1")
            .bind(id)
            .execute(&state.db)
            .await?;

        let _ = state.event_bus.publish(&DomainEvent::GroupJoined {
            group_id: id,
            user_id: auth.user_id,
        }).await;
    }

    Ok(Json(json!({ "data": { "status": status } })))
}

pub async fn leave_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    // Owner can't leave
    let role = sqlx::query_scalar::<_, String>(
        "SELECT role FROM group_members WHERE group_id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?;

    if role.as_deref() == Some("owner") {
        return Err(ApiError::BadRequest("Owner cannot leave. Transfer ownership first.".into()));
    }

    sqlx::query("DELETE FROM group_members WHERE group_id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    sqlx::query("UPDATE groups SET member_count = (SELECT COUNT(*) FROM group_members WHERE group_id = $1 AND status = 'active') WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    let _ = state.event_bus.publish(&DomainEvent::GroupLeft {
        group_id: id,
        user_id: auth.user_id,
    }).await;

    Ok(Json(json!({ "data": { "left": true } })))
}

pub async fn list_members(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let members = sqlx::query_as::<_, MemberRow>(
        r#"
        SELECT gm.user_id, u.username, u.first_name, u.last_name, u.avatar, gm.role, gm.status
        FROM group_members gm JOIN users u ON u.id = gm.user_id
        WHERE gm.group_id = $1 AND gm.status = 'active'
          AND ($2::bigint IS NULL OR gm.id < $2)
        ORDER BY gm.id DESC LIMIT $3
        "#,
    )
    .bind(id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = members.len() as i64 > limit;
    let members: Vec<_> = members.into_iter().take(limit as usize).collect();

    Ok(Json(json!({ "data": members, "meta": { "has_more": has_more } })))
}

pub async fn kick_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, uid)): Path<(i64, i64)>,
) -> Result<Json<Value>, ApiError> {
    verify_group_admin(&state, id, auth.user_id).await?;

    sqlx::query("DELETE FROM group_members WHERE group_id = $1 AND user_id = $2 AND role = 'member'")
        .bind(id)
        .bind(uid)
        .execute(&state.db)
        .await?;

    sqlx::query("UPDATE groups SET member_count = (SELECT COUNT(*) FROM group_members WHERE group_id = $1 AND status = 'active') WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "kicked": true } })))
}

pub async fn change_role(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, uid)): Path<(i64, i64)>,
    Json(req): Json<ChangeRoleRequest>,
) -> Result<Json<Value>, ApiError> {
    verify_group_admin(&state, id, auth.user_id).await?;

    let valid_roles = ["admin", "moderator", "member"];
    if !valid_roles.contains(&req.role.as_str()) {
        return Err(ApiError::BadRequest("Invalid role".into()));
    }

    sqlx::query("UPDATE group_members SET role = $1 WHERE group_id = $2 AND user_id = $3")
        .bind(&req.role)
        .bind(id)
        .bind(uid)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "role": req.role } })))
}

pub async fn join_requests(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    verify_group_admin(&state, id, auth.user_id).await?;

    let requests = sqlx::query_as::<_, JoinRequestRow>(
        r#"
        SELECT gm.id, gm.user_id, u.username, u.first_name, u.last_name, u.avatar, gm.created_at
        FROM group_members gm JOIN users u ON u.id = gm.user_id
        WHERE gm.group_id = $1 AND gm.status = 'pending'
        ORDER BY gm.created_at
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": requests })))
}

pub async fn accept_join(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, rid)): Path<(i64, i64)>,
) -> Result<Json<Value>, ApiError> {
    verify_group_admin(&state, id, auth.user_id).await?;

    sqlx::query("UPDATE group_members SET status = 'active' WHERE id = $1 AND group_id = $2 AND status = 'pending'")
        .bind(rid)
        .bind(id)
        .execute(&state.db)
        .await?;

    sqlx::query("UPDATE groups SET member_count = (SELECT COUNT(*) FROM group_members WHERE group_id = $1 AND status = 'active') WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "accepted": true } })))
}

pub async fn reject_join(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((id, rid)): Path<(i64, i64)>,
) -> Result<Json<Value>, ApiError> {
    verify_group_admin(&state, id, auth.user_id).await?;

    sqlx::query("DELETE FROM group_members WHERE id = $1 AND group_id = $2 AND status = 'pending'")
        .bind(rid)
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "rejected": true } })))
}

pub async fn list_categories(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let cats = sqlx::query_as::<_, CategoryRow>(
        "SELECT id, name_key, slug, parent_id FROM categories WHERE type = 'group' AND active = TRUE ORDER BY sort_order",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": cats })))
}

pub async fn search_groups(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Value>, ApiError> {
    let ilike = format!("%{}%", q.q);
    let limit = q.pagination.limit();

    let groups = sqlx::query_as::<_, GroupSummary>(
        "SELECT id, group_name, group_title, avatar, privacy, member_count FROM groups WHERE active = TRUE AND (group_name ILIKE $1 OR group_title ILIKE $1) ORDER BY member_count DESC LIMIT $2",
    )
    .bind(&ilike)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": groups })))
}

pub async fn suggested_groups(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let groups = sqlx::query_as::<_, GroupSummary>(
        r#"
        SELECT id, group_name, group_title, avatar, privacy, member_count
        FROM groups
        WHERE active = TRUE AND id NOT IN (SELECT group_id FROM group_members WHERE user_id = $1)
        ORDER BY member_count DESC LIMIT 20
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": groups })))
}

pub async fn my_groups(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let groups = sqlx::query_as::<_, GroupSummary>(
        "SELECT id, group_name, group_title, avatar, privacy, member_count FROM groups WHERE user_id = $1 AND active = TRUE ORDER BY created_at DESC",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": groups })))
}

pub async fn joined_groups(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let groups = sqlx::query_as::<_, GroupSummary>(
        r#"
        SELECT g.id, g.group_name, g.group_title, g.avatar, g.privacy, g.member_count
        FROM groups g JOIN group_members gm ON gm.group_id = g.id
        WHERE gm.user_id = $1 AND gm.status = 'active' AND g.active = TRUE
        ORDER BY gm.created_at DESC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": groups })))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn verify_group_admin(state: &AppState, group_id: i64, user_id: i64) -> Result<(), ApiError> {
    let role = sqlx::query_scalar::<_, String>(
        "SELECT role FROM group_members WHERE group_id = $1 AND user_id = $2 AND status = 'active'",
    )
    .bind(group_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(ApiError::Forbidden("".into()))?;

    if role != "owner" && role != "admin" {
        return Err(ApiError::Forbidden("".into()));
    }
    Ok(())
}
