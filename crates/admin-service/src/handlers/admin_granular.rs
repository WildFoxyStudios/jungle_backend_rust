//! Granular admin actions on a single user + extended dashboard stats +
//! rich `users` listing filters. Implements plan Â§3.22 AP-A2, AP-A3, AP-A4.
//!
//! Endpoints exposed:
//! * `DELETE /v1/admin/users/{id}/posts`           â€” nuke only the user's posts
//! * `DELETE /v1/admin/users/{id}/articles`        â€” nuke only the user's blog articles
//! * `DELETE /v1/admin/users/{id}/stories`         â€” nuke only the user's stories
//! * `DELETE /v1/admin/users/{id}/messages`        â€” nuke only the user's messages
//! * `DELETE /v1/admin/users/{id}/notifications`   â€” nuke only the user's notifications
//! * `PUT    /v1/admin/users/{id}/permissions`     â€” write the granular JSONB map
//! * `GET    /v1/admin/users/{id}/permissions`     â€” read the granular JSONB map
//! * `GET    /v1/admin/stats/extended`             â€” 8 stat cards for the dashboard
//! * `GET    /v1/admin/users` (extended filters)   â€” ip, phone, gender, country,
//!   date_from, date_to

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    permissions::Permission,
};
use sqlx::FromRow;
use time::OffsetDateTime;

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Granular delete actions on a user's content.
// Each handler is deliberately surgical: it only touches its named
// resource so an admin can "delete posts but keep the account alive".
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

/// Shared response shape for every granular delete.
#[derive(Serialize)]
struct DeletedCount {
    deleted: u64,
}

fn deleted(n: u64) -> Json<Value> {
    Json(json!({ "data": DeletedCount { deleted: n } }))
}

pub async fn delete_user_posts(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageUsers, &state).await?;
    // Soft-delete to preserve foreign keys (comments, reactions, shares still
    // want to reference the row). Every read path filters on `deleted_at IS NULL`.
    let n = sqlx::query(
        "UPDATE posts SET deleted_at = NOW() \
         WHERE user_id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .execute(&state.db)
    .await?
    .rows_affected();
    Ok(deleted(n))
}

pub async fn delete_user_articles(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageUsers, &state).await?;
    let n = sqlx::query(
        "UPDATE blog_articles SET deleted_at = NOW() \
         WHERE user_id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .execute(&state.db)
    .await?
    .rows_affected();
    Ok(deleted(n))
}

pub async fn delete_user_stories(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageUsers, &state).await?;
    // Hard delete: stories are ephemeral by design and have no downstream refs.
    let n = sqlx::query("DELETE FROM stories WHERE user_id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await?
        .rows_affected();
    Ok(deleted(n))
}

pub async fn delete_user_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageUsers, &state).await?;
    // Soft delete so the conversation UI can still show "message deleted".
    let n = sqlx::query(
        "UPDATE messages SET deleted_at = NOW() \
         WHERE sender_id = $1 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .execute(&state.db)
    .await?
    .rows_affected();
    Ok(deleted(n))
}

pub async fn delete_user_notifications(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageUsers, &state).await?;
    let n = sqlx::query("DELETE FROM notifications WHERE recipient_id = $1 OR sender_id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await?
        .rows_affected();
    Ok(deleted(n))
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Granular permissions JSONB (plan Â§3.22 AP-A1)
// Uses the `users.permissions` JSONB column added in
// 20260422000013_user_permissions.sql.
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

pub async fn get_granular_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ViewUsers, &state).await?;

    let perms: Option<Value> =
        sqlx::query_scalar("SELECT permissions FROM users WHERE id = $1 AND deleted_at IS NULL")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?;

    let perms = perms.ok_or_else(|| ApiError::NotFound("User not found".into()))?;

    Ok(Json(json!({ "data": { "permissions": perms } })))
}

/// Full replacement of the permissions map. The frontend ships the
/// complete desired state so partial diffs are not our concern.
#[derive(Deserialize)]
pub struct UpdateGranularPermsRequest {
    pub permissions: Value,
}

pub async fn put_granular_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
    Json(req): Json<UpdateGranularPermsRequest>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageUsers, &state).await?;

    // Reject non-object payloads â€” the column shape is a flat stringâ†’bool map.
    if !req.permissions.is_object() {
        return Err(ApiError::BadRequest(
            "permissions must be a JSON object".into(),
        ));
    }

    let updated =
        sqlx::query("UPDATE users SET permissions = $1 WHERE id = $2 AND deleted_at IS NULL")
            .bind(&req.permissions)
            .bind(user_id)
            .execute(&state.db)
            .await?
            .rows_affected();

    if updated == 0 {
        return Err(ApiError::NotFound("User not found".into()));
    }

    Ok(Json(json!({ "data": { "permissions": req.permissions } })))
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Extended dashboard stats (plan Â§3.22 AP-A2)
// Returns 8 cards in a single round-trip + optional time range.
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[derive(Deserialize)]
pub struct StatsRangeQuery {
    /// `today`, `yesterday`, `week`, `month`, `last_month`, `year`, `all`.
    pub range: Option<String>,
    /// ISO 8601 lower bound, overrides `range` when present.
    pub from: Option<OffsetDateTime>,
    /// ISO 8601 upper bound, overrides `range` when present.
    pub to: Option<OffsetDateTime>,
}

#[derive(Serialize)]
pub struct ExtendedStats {
    pub total_users: i64,
    pub total_posts: i64,
    pub total_pages: i64,
    pub total_groups: i64,
    pub online_users: i64,
    pub total_comments: i64,
    pub total_games: i64,
    pub total_messages: i64,
    /// Echoes back the effective window (so the client can render a subtitle).
    pub range: String,
    pub from: Option<OffsetDateTime>,
    pub to: Option<OffsetDateTime>,
}

/// Converts a named range into a lower bound. `None` means "since epoch".
fn range_to_from(range: &str) -> Option<&'static str> {
    match range {
        "today" => Some("NOW() - INTERVAL '1 day'"),
        "yesterday" => Some("NOW() - INTERVAL '2 day'"),
        "week" => Some("NOW() - INTERVAL '7 day'"),
        "month" => Some("NOW() - INTERVAL '30 day'"),
        "last_month" => Some("NOW() - INTERVAL '60 day'"),
        "year" => Some("NOW() - INTERVAL '365 day'"),
        _ => None, // "all" / unknown â†’ no lower bound
    }
}

pub async fn extended_stats(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<StatsRangeQuery>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ViewUsers, &state).await?;

    // Determine the time window. Explicit `from/to` always win.
    let range = q.range.as_deref().unwrap_or("all").to_string();
    let (from_sql, to_sql) = if q.from.is_some() || q.to.is_some() {
        ("$1::timestamptz", "$2::timestamptz")
    } else {
        let lower = range_to_from(&range).unwrap_or("'epoch'::timestamptz");
        // We'll just format the lower bound directly â€” no user input, safe.
        // For an upper bound we always use NOW() when only a range is given.
        return extended_stats_simple(&state, &range, lower).await;
    };

    // Explicit from/to path: bind values.
    let from = q.from.unwrap_or(OffsetDateTime::UNIX_EPOCH);
    let to = q.to.unwrap_or_else(OffsetDateTime::now_utc);

    let total_users: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM users WHERE created_at >= {from_sql} AND created_at <= {to_sql} AND deleted_at IS NULL",
    )).bind(from).bind(to).fetch_one(&state.db).await?;
    let total_posts: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM posts WHERE created_at >= {from_sql} AND created_at <= {to_sql} AND deleted_at IS NULL",
    )).bind(from).bind(to).fetch_one(&state.db).await?;
    let total_pages: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM pages WHERE created_at >= {from_sql} AND created_at <= {to_sql}",
    ))
    .bind(from)
    .bind(to)
    .fetch_one(&state.db)
    .await?;
    let total_groups: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM groups WHERE created_at >= {from_sql} AND created_at <= {to_sql}",
    ))
    .bind(from)
    .bind(to)
    .fetch_one(&state.db)
    .await?;
    // Online: last_seen within the last 5 minutes. The time-range filter
    // doesn't really make sense here; we surface the live metric instead.
    let online_users: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE last_seen > NOW() - INTERVAL '5 minutes' AND deleted_at IS NULL",
    ).fetch_one(&state.db).await.unwrap_or(0);
    let total_comments: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM post_comments WHERE created_at >= {from_sql} AND created_at <= {to_sql} AND deleted_at IS NULL",
    )).bind(from).bind(to).fetch_one(&state.db).await.unwrap_or(0);
    let total_games: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM games WHERE created_at >= {from_sql} AND created_at <= {to_sql}",
    ))
    .bind(from)
    .bind(to)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);
    let total_messages: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM messages WHERE created_at >= {from_sql} AND created_at <= {to_sql} AND deleted_at IS NULL",
    )).bind(from).bind(to).fetch_one(&state.db).await.unwrap_or(0);

    Ok(Json(json!({
        "data": ExtendedStats {
            total_users, total_posts, total_pages, total_groups,
            online_users, total_comments, total_games, total_messages,
            range, from: Some(from), to: Some(to),
        }
    })))
}

/// Same as the explicit-window path but uses an inline SQL lower bound,
/// which is safe because it comes from our hard-coded enum only.
async fn extended_stats_simple(
    state: &AppState,
    range: &str,
    lower: &str,
) -> Result<Json<Value>, ApiError> {
    let sql = |table: &str, filter_deleted: bool| {
        let deleted = if filter_deleted {
            " AND deleted_at IS NULL"
        } else {
            ""
        };
        format!("SELECT COUNT(*) FROM {table} WHERE created_at >= {lower}{deleted}")
    };

    let total_users: i64 = sqlx::query_scalar(&sql("users", true))
        .fetch_one(&state.db)
        .await?;
    let total_posts: i64 = sqlx::query_scalar(&sql("posts", true))
        .fetch_one(&state.db)
        .await?;
    let total_pages: i64 = sqlx::query_scalar(&sql("pages", false))
        .fetch_one(&state.db)
        .await?;
    let total_groups: i64 = sqlx::query_scalar(&sql("groups", false))
        .fetch_one(&state.db)
        .await?;
    let online_users: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE last_seen > NOW() - INTERVAL '5 minutes' AND deleted_at IS NULL",
    ).fetch_one(&state.db).await.unwrap_or(0);
    let total_comments: i64 = sqlx::query_scalar(&sql("post_comments", true))
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let total_games: i64 = sqlx::query_scalar(&sql("games", false))
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let total_messages: i64 = sqlx::query_scalar(&sql("messages", true))
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    Ok(Json(json!({
        "data": ExtendedStats {
            total_users, total_posts, total_pages, total_groups,
            online_users, total_comments, total_games, total_messages,
            range: range.to_string(),
            from: None,
            to: None,
        }
    })))
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Rich users listing (plan Â§3.22 AP-A3)
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[derive(Deserialize, Default)]
pub struct UsersFilterQuery {
    pub q: Option<String>,
    pub ip: Option<String>,
    pub phone: Option<String>,
    pub gender: Option<String>,
    pub country: Option<String>,
    pub date_from: Option<OffsetDateTime>,
    pub date_to: Option<OffsetDateTime>,
    pub status: Option<String>, // active|banned|verified|pro|admin|pending
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(FromRow, Serialize)]
pub struct AdminUserRow {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
    pub phone_number: Option<String>,
    pub gender: Option<String>,
    pub country_id: Option<i32>,
    /// Registration IP captured by the auth handler. Admin-only.
    pub signup_ip: Option<String>,
    pub signup_source: Option<String>,
    pub is_active: bool,
    pub is_banned: bool,
    pub is_verified: bool,
    pub is_pro: i16,
    pub is_admin: bool,
    pub email_verified: bool,
    pub last_seen: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

pub async fn list_users_filtered(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<UsersFilterQuery>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ViewUsers, &state).await?;

    let limit = q.per_page.or(q.limit).unwrap_or(50).clamp(1, 500);
    let page = q.page.unwrap_or(1).max(1);
    let offset = q.offset.unwrap_or((page - 1) * limit).max(0);

    // Build the WHERE dynamically with bound params only â€” never string
    // interpolation of user input.
    let mut conds: Vec<String> = vec!["u.deleted_at IS NULL".into()];
    let mut args: Vec<Box<dyn SqlArg>> = Vec::new();

    if let Some(search) = q.q.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        args.push(Box::new(format!("%{search}%")));
        let idx = args.len();
        conds.push(format!(
            "(u.username ILIKE ${idx} OR u.email ILIKE ${idx} \
              OR u.first_name ILIKE ${idx} OR u.last_name ILIKE ${idx})"
        ));
    }
    if let Some(ip) = q.ip.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        args.push(Box::new(ip.to_string()));
        let idx = args.len();
        conds.push(format!("u.signup_ip = ${idx}"));
    }
    if let Some(phone) = q.phone.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        args.push(Box::new(format!("%{phone}%")));
        let idx = args.len();
        conds.push(format!("u.phone_number ILIKE ${idx}"));
    }
    if let Some(gender) = q
        .gender
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        args.push(Box::new(gender.to_string()));
        let idx = args.len();
        conds.push(format!("u.gender = ${idx}"));
    }
    if let Some(country) = q
        .country
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        && let Ok(cid) = country.parse::<i32>()
    {
        args.push(Box::new(cid));
        let idx = args.len();
        conds.push(format!("u.country_id = ${idx}"));
    }
    if let Some(from) = q.date_from {
        args.push(Box::new(from));
        let idx = args.len();
        conds.push(format!("u.created_at >= ${idx}"));
    }
    if let Some(to) = q.date_to {
        args.push(Box::new(to));
        let idx = args.len();
        conds.push(format!("u.created_at <= ${idx}"));
    }
    match q.status.as_deref() {
        Some("active") => conds.push("u.is_active = TRUE".into()),
        Some("banned") => conds.push("u.is_active = FALSE".into()),
        Some("verified") => conds.push("u.is_verified = TRUE".into()),
        Some("pro") => conds.push("u.is_pro > 0".into()),
        Some("admin") => conds.push("u.is_admin = TRUE".into()),
        Some("pending") => conds.push("u.email_verified = FALSE".into()),
        _ => {}
    }

    let where_sql = conds.join(" AND ");

    let limit_idx = args.len() + 1;
    let offset_idx = args.len() + 2;

    let sql = format!(
        "SELECT u.id, u.username, u.email, u.first_name, u.last_name, u.avatar, \
                u.phone_number, u.gender, u.country_id, u.signup_ip, \
                u.signup_source, u.is_active, NOT u.is_active AS is_banned, u.is_verified, \
                u.is_pro, u.is_admin, u.email_verified, u.last_seen, u.created_at \
           FROM users u \
          WHERE {where_sql} \
          ORDER BY u.id DESC \
          LIMIT ${limit_idx} OFFSET ${offset_idx}"
    );

    let mut query = sqlx::query_as::<_, AdminUserRow>(&sql);
    for arg in &args {
        query = arg.bind(query);
    }
    query = query.bind(limit).bind(offset);

    let rows = query.fetch_all(&state.db).await?;

    // Total count (without pagination) for the UI footer.
    let count_sql = format!("SELECT COUNT(*) FROM users u WHERE {where_sql}");
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for arg in &args {
        count_query = arg.bind_scalar(count_query);
    }
    let total: i64 = count_query.fetch_one(&state.db).await.unwrap_or(0);

    Ok(Json(json!({
        "data": rows,
        "meta": {
            "total": total,
            "page": page,
            "per_page": limit,
            "limit": limit,
            "offset": offset,
        }
    })))
}

/// Small trait-object helper so the dynamic filter builder can mix different
/// bound types on the same `Vec`. The frontend only ever sends primitives.
trait SqlArg: Send + Sync {
    fn bind<'q>(
        &'q self,
        q: sqlx::query::QueryAs<'q, sqlx::Postgres, AdminUserRow, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::QueryAs<'q, sqlx::Postgres, AdminUserRow, sqlx::postgres::PgArguments>;

    fn bind_scalar<'q>(
        &'q self,
        q: sqlx::query::QueryScalar<'q, sqlx::Postgres, i64, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::QueryScalar<'q, sqlx::Postgres, i64, sqlx::postgres::PgArguments>;
}

impl SqlArg for String {
    fn bind<'q>(
        &'q self,
        q: sqlx::query::QueryAs<'q, sqlx::Postgres, AdminUserRow, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::QueryAs<'q, sqlx::Postgres, AdminUserRow, sqlx::postgres::PgArguments> {
        q.bind(self.as_str())
    }
    fn bind_scalar<'q>(
        &'q self,
        q: sqlx::query::QueryScalar<'q, sqlx::Postgres, i64, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::QueryScalar<'q, sqlx::Postgres, i64, sqlx::postgres::PgArguments> {
        q.bind(self.as_str())
    }
}
impl SqlArg for i64 {
    fn bind<'q>(
        &'q self,
        q: sqlx::query::QueryAs<'q, sqlx::Postgres, AdminUserRow, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::QueryAs<'q, sqlx::Postgres, AdminUserRow, sqlx::postgres::PgArguments> {
        q.bind(*self)
    }
    fn bind_scalar<'q>(
        &'q self,
        q: sqlx::query::QueryScalar<'q, sqlx::Postgres, i64, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::QueryScalar<'q, sqlx::Postgres, i64, sqlx::postgres::PgArguments> {
        q.bind(*self)
    }
}
impl SqlArg for i32 {
    fn bind<'q>(
        &'q self,
        q: sqlx::query::QueryAs<'q, sqlx::Postgres, AdminUserRow, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::QueryAs<'q, sqlx::Postgres, AdminUserRow, sqlx::postgres::PgArguments> {
        q.bind(*self)
    }
    fn bind_scalar<'q>(
        &'q self,
        q: sqlx::query::QueryScalar<'q, sqlx::Postgres, i64, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::QueryScalar<'q, sqlx::Postgres, i64, sqlx::postgres::PgArguments> {
        q.bind(*self)
    }
}
impl SqlArg for OffsetDateTime {
    fn bind<'q>(
        &'q self,
        q: sqlx::query::QueryAs<'q, sqlx::Postgres, AdminUserRow, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::QueryAs<'q, sqlx::Postgres, AdminUserRow, sqlx::postgres::PgArguments> {
        q.bind(*self)
    }
    fn bind_scalar<'q>(
        &'q self,
        q: sqlx::query::QueryScalar<'q, sqlx::Postgres, i64, sqlx::postgres::PgArguments>,
    ) -> sqlx::query::QueryScalar<'q, sqlx::Postgres, i64, sqlx::postgres::PgArguments> {
        q.bind(*self)
    }
}

// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
// Gender breakdown widget (plan Â§3.22 AP-A3)
// â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

#[derive(Serialize, FromRow)]
pub struct GenderBucket {
    pub gender: Option<String>,
    pub count: i64,
}

pub async fn gender_breakdown(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ViewUsers, &state).await?;

    let buckets = sqlx::query_as::<_, GenderBucket>(
        "SELECT gender, COUNT(*) AS count \
           FROM users \
          WHERE deleted_at IS NULL \
          GROUP BY gender \
          ORDER BY count DESC",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": buckets })))
}

