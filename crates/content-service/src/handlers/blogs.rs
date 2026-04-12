use axum::{
    extract::{Multipart, Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateBlogRequest {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    pub content: String,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
    pub category_id: Option<i64>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBlogRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
    pub category_id: Option<i64>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AddCommentRequest {
    #[validate(length(min = 1, max = 5000))]
    pub content: String,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct BlogRow {
    pub id: i64,
    pub uuid: uuid::Uuid,
    pub user_id: i64,
    pub title: String,
    pub content: Option<String>,
    pub description: Option<String>,
    pub thumbnail: String,
    pub category_id: Option<i64>,
    pub tags: Vec<String>,
    pub view_count: i32,
    pub share_count: i32,
    pub is_approved: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct BlogSummary {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub thumbnail: String,
    pub user_id: i64,
    pub username: String,
    pub avatar: String,
    pub view_count: i32,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CommentRow {
    pub id: i64,
    pub blog_id: i64,
    pub user_id: i64,
    pub parent_id: Option<i64>,
    pub content: String,
    pub like_count: i32,
    pub created_at: OffsetDateTime,
    pub username: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CategoryRow {
    pub id: i64,
    pub name_key: String,
    pub slug: Option<String>,
}

pub async fn list_blogs(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let blogs = sqlx::query_as::<_, BlogSummary>(
        r#"
        SELECT b.id, b.title, b.description, b.thumbnail, b.user_id,
            u.username, u.avatar, b.view_count, b.created_at
        FROM blogs b JOIN users u ON u.id = b.user_id
        WHERE b.is_approved = TRUE
          AND ($1::bigint IS NULL OR b.id < $1)
        ORDER BY b.id DESC LIMIT $2
        "#,
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = blogs.len() as i64 > limit;
    let blogs: Vec<_> = blogs.into_iter().take(limit as usize).collect();
    let next_cursor = blogs.last().map(|b| b.id.to_string());

    Ok(Json(json!({ "data": blogs, "meta": { "cursor": next_cursor, "has_more": has_more } })))
}

pub async fn create_blog(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateBlogRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(ApiError::from)?;

    let tags = req.tags.unwrap_or_default();

    let blog = sqlx::query_as::<_, BlogRow>(
        r#"
        INSERT INTO blogs (user_id, title, content, description, thumbnail, category_id, tags)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(auth.user_id)
    .bind(&req.title)
    .bind(&req.content)
    .bind(&req.description)
    .bind(req.thumbnail.as_deref().unwrap_or("default-blog.jpg"))
    .bind(req.category_id)
    .bind(&tags)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": blog })))
}

pub async fn get_blog(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    // Increment view count
    sqlx::query("UPDATE blogs SET view_count = view_count + 1 WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    let blog = sqlx::query_as::<_, BlogRow>("SELECT * FROM blogs WHERE id = $1 AND is_approved = TRUE")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Blog not found".into()))?;

    Ok(Json(json!({ "data": blog })))
}

pub async fn update_blog(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateBlogRequest>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM blogs WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Blog not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    let blog = sqlx::query_as::<_, BlogRow>(
        r#"
        UPDATE blogs SET
            title = COALESCE($2, title),
            content = COALESCE($3, content),
            description = COALESCE($4, description),
            thumbnail = COALESCE($5, thumbnail),
            category_id = COALESCE($6, category_id),
            tags = COALESCE($7, tags),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.title)
    .bind(&req.content)
    .bind(&req.description)
    .bind(&req.thumbnail)
    .bind(req.category_id)
    .bind(&req.tags)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": blog })))
}

pub async fn delete_blog(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM blogs WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Blog not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    sqlx::query("DELETE FROM blogs WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn search_blogs(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = q.pagination.limit();

    let blogs = sqlx::query_as::<_, BlogSummary>(
        r#"
        SELECT b.id, b.title, b.description, b.thumbnail, b.user_id,
            u.username, u.avatar, b.view_count, b.created_at
        FROM blogs b JOIN users u ON u.id = b.user_id
        WHERE b.is_approved = TRUE AND b.search_vector @@ plainto_tsquery('english', $1)
        ORDER BY ts_rank(b.search_vector, plainto_tsquery('english', $1)) DESC
        LIMIT $2
        "#,
    )
    .bind(&q.q)
    .bind(limit)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": blogs })))
}

pub async fn my_blogs(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let blogs = sqlx::query_as::<_, BlogRow>(
        "SELECT * FROM blogs WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": blogs })))
}

pub async fn list_categories(
    State(state): State<AppState>,
) -> Result<Json<Value>, ApiError> {
    let cats = sqlx::query_as::<_, CategoryRow>(
        "SELECT id, name_key, slug FROM categories WHERE type = 'blog' AND active = TRUE ORDER BY sort_order",
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": cats })))
}

pub async fn list_comments(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let comments = sqlx::query_as::<_, CommentRow>(
        r#"
        SELECT bc.id, bc.blog_id, bc.user_id, bc.parent_id, bc.content, bc.like_count, bc.created_at,
            u.username, u.first_name, u.last_name, u.avatar
        FROM blog_comments bc JOIN users u ON u.id = bc.user_id
        WHERE bc.blog_id = $1
          AND ($2::bigint IS NULL OR bc.id < $2)
        ORDER BY bc.id DESC LIMIT $3
        "#,
    )
    .bind(id)
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = comments.len() as i64 > limit;
    let comments: Vec<_> = comments.into_iter().take(limit as usize).collect();

    Ok(Json(json!({ "data": comments, "meta": { "has_more": has_more } })))
}

pub async fn add_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<AddCommentRequest>,
) -> Result<Json<Value>, ApiError> {
    req.validate().map_err(ApiError::from)?;

    let comment = sqlx::query_as::<_, CommentRow>(
        r#"
        WITH ins AS (
            INSERT INTO blog_comments (blog_id, user_id, parent_id, content)
            VALUES ($1, $2, $3, $4)
            RETURNING *
        )
        SELECT ins.id, ins.blog_id, ins.user_id, ins.parent_id, ins.content, ins.like_count, ins.created_at,
            u.username, u.first_name, u.last_name, u.avatar
        FROM ins JOIN users u ON u.id = ins.user_id
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(req.parent_id)
    .bind(&req.content)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": comment })))
}

pub async fn delete_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let owner = sqlx::query_scalar::<_, i64>("SELECT user_id FROM blog_comments WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Comment not found".into()))?;

    if owner != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    sqlx::query("DELETE FROM blog_comments WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "deleted": true } })))
}

/// POST /v1/blogs/upload-image — upload an image for the blog WYSIWYG editor
pub async fn upload_blog_image(
    State(_state): State<AppState>,
    _auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Multipart error: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name != "image" {
            continue;
        }

        let file_name = field
            .file_name()
            .unwrap_or("upload.jpg")
            .to_string();

        let data = field
            .bytes()
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {e}")))?;

        if data.len() > 10 * 1024 * 1024 {
            return Err(ApiError::BadRequest("Image exceeds 10MB limit".into()));
        }

        let ext = file_name
            .rsplit('.')
            .next()
            .unwrap_or("jpg");
        let unique_name = format!("{}.{ext}", uuid::Uuid::new_v4());
        let upload_dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".into());
        let dir_path = format!("{upload_dir}/blogs");
        tokio::fs::create_dir_all(&dir_path)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to create dir: {e}")))?;
        let file_path = format!("{dir_path}/{unique_name}");
        tokio::fs::write(&file_path, &data)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to save file: {e}")))?;

        let base_url = std::env::var("MEDIA_BASE_URL").unwrap_or_else(|_| "/uploads".into());
        let url = format!("{base_url}/blogs/{unique_name}");

        return Ok(Json(json!({ "data": { "url": url } })));
    }

    Err(ApiError::BadRequest("No image field found in multipart".into()))
}
