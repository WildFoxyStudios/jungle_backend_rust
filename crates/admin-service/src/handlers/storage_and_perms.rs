//! Storage provider CRUD + permissions catalog admin endpoints.
//!
//! - `/v1/admin/storage/config` → list / create storage providers (S3/R2/MinIO/Wasabi/...)
//! - `/v1/admin/storage/config/{id}` → update / delete
//! - `/v1/admin/storage/config/{id}/test` → validate credentials with a PutObject/DeleteObject probe
//! - `/v1/admin/permissions/catalog` → list catalog of granular admin permissions

use aws_sdk_s3::primitives::ByteStream;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    crypto,
    errors::ApiError,
    permissions::Permission,
};
use uuid;

// ═══════════════════════════════════════════════════════════════════
// Storage config
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StorageProviderRow {
    pub id: i64,
    pub name: String,
    pub provider_type: String,
    pub bucket: String,
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub access_key: String,
    pub public_url: Option<String>,
    pub is_active: bool,
    pub priority: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateStorageRequest {
    pub name: String,
    pub provider_type: String,
    pub bucket: String,
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub access_key: String,
    pub secret_key: String,
    pub public_url: Option<String>,
    pub priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStorageRequest {
    pub bucket: Option<String>,
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub public_url: Option<String>,
    pub is_active: Option<bool>,
    pub priority: Option<i32>,
}

pub async fn list_storage(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageStorage, &state).await?;

    let rows: Vec<StorageProviderRow> = sqlx::query_as(
        r#"SELECT id, name, provider_type, bucket, endpoint, region,
                  access_key, public_url, is_active, priority
             FROM storage_providers
         ORDER BY priority ASC"#,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": rows })))
}

pub async fn create_storage(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateStorageRequest>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageStorage, &state).await?;

    if req.secret_key.trim().is_empty() {
        return Err(ApiError::BadRequest("secret_key required".into()));
    }

    let enc_key = enc_key_from_env();
    let encrypted = crypto::encrypt(&enc_key, &req.secret_key)
        .map_err(|e| ApiError::Internal(format!("encryption failed: {}", e)))?;

    let id: i64 = sqlx::query_scalar(
        r#"INSERT INTO storage_providers
            (name, provider_type, bucket, endpoint, region, access_key,
             secret_key_encrypted, public_url, priority)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id"#,
    )
    .bind(&req.name)
    .bind(&req.provider_type)
    .bind(&req.bucket)
    .bind(&req.endpoint)
    .bind(&req.region)
    .bind(&req.access_key)
    .bind(&encrypted)
    .bind(&req.public_url)
    .bind(req.priority.unwrap_or(100))
    .fetch_one(&state.db)
    .await?;

    Ok(Json(json!({ "data": { "id": id } })))
}

pub async fn update_storage(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateStorageRequest>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageStorage, &state).await?;

    let enc_key = enc_key_from_env();

    if let Some(bucket) = &req.bucket {
        sqlx::query("UPDATE storage_providers SET bucket = $1, updated_at = NOW() WHERE id = $2")
            .bind(bucket).bind(id).execute(&state.db).await?;
    }
    if let Some(ep) = &req.endpoint {
        sqlx::query("UPDATE storage_providers SET endpoint = $1, updated_at = NOW() WHERE id = $2")
            .bind(ep).bind(id).execute(&state.db).await?;
    }
    if let Some(r) = &req.region {
        sqlx::query("UPDATE storage_providers SET region = $1, updated_at = NOW() WHERE id = $2")
            .bind(r).bind(id).execute(&state.db).await?;
    }
    if let Some(ak) = &req.access_key {
        sqlx::query("UPDATE storage_providers SET access_key = $1, updated_at = NOW() WHERE id = $2")
            .bind(ak).bind(id).execute(&state.db).await?;
    }
    if let Some(sk) = &req.secret_key
        && !sk.is_empty()
    {
        let encrypted = crypto::encrypt(&enc_key, sk)
            .map_err(|e| ApiError::Internal(format!("encryption failed: {}", e)))?;
        sqlx::query("UPDATE storage_providers SET secret_key_encrypted = $1, updated_at = NOW() WHERE id = $2")
            .bind(encrypted).bind(id).execute(&state.db).await?;
    }
    if let Some(pu) = &req.public_url {
        sqlx::query("UPDATE storage_providers SET public_url = $1, updated_at = NOW() WHERE id = $2")
            .bind(pu).bind(id).execute(&state.db).await?;
    }
    if let Some(active) = req.is_active {
        sqlx::query("UPDATE storage_providers SET is_active = $1, updated_at = NOW() WHERE id = $2")
            .bind(active).bind(id).execute(&state.db).await?;
    }
    if let Some(p) = req.priority {
        sqlx::query("UPDATE storage_providers SET priority = $1, updated_at = NOW() WHERE id = $2")
            .bind(p).bind(id).execute(&state.db).await?;
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

pub async fn delete_storage(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageStorage, &state).await?;
    sqlx::query("DELETE FROM storage_providers WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn test_storage(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageStorage, &state).await?;

    type StorageRow = (
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        String,
    );
    let row: Option<StorageRow> = sqlx::query_as(
        r#"SELECT provider_type, bucket, endpoint, region, access_key, secret_key_encrypted
             FROM storage_providers WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    let (_ptype, bucket, endpoint, region, access_key, secret_enc) =
        row.ok_or(ApiError::NotFound("storage provider not found".into()))?;

    let enc_key = enc_key_from_env();
    let secret_key = crypto::decrypt(&enc_key, &secret_enc)
        .map_err(|e| ApiError::Internal(format!("decrypt failed: {}", e)))?;

    // Build a dedicated S3 client using the stored credentials
    let creds = aws_sdk_s3::config::Credentials::new(
        &access_key,
        &secret_key,
        None,
        None,
        "storage_test",
    );
    let mut builder = aws_sdk_s3::Config::builder()
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new(
            region.unwrap_or_else(|| "auto".into()),
        ))
        .credentials_provider(creds)
        .force_path_style(true);
    if let Some(ep) = endpoint.as_deref()
        && !ep.is_empty()
    {
        builder = builder.endpoint_url(ep);
    }
    let client = aws_sdk_s3::Client::from_conf(builder.build());

    let test_key = format!(".jungle-storage-probe-{}.txt", uuid::Uuid::new_v4());
    let body = ByteStream::from_static(b"ok");

    let put_result = client
        .put_object()
        .bucket(&bucket)
        .key(&test_key)
        .body(body)
        .content_type("text/plain")
        .send()
        .await;

    match put_result {
        Ok(_) => {
            // Clean up immediately
            let _ = client.delete_object().bucket(&bucket).key(&test_key).send().await;
            Ok(Json(json!({
                "data": { "ok": true, "message": "PutObject + DeleteObject succeeded" }
            })))
        }
        Err(e) => Ok(Json(json!({
            "data": { "ok": false, "error": e.to_string() }
        }))),
    }
}

fn enc_key_from_env() -> Vec<u8> {
    let master = std::env::var("INTERNAL_SERVICE_KEY")
        .or_else(|_| std::env::var("JWT_SECRET"))
        .unwrap_or_else(|_| "change-me-insecure-dev-key".into());
    shared::crypto::derive_key(master.as_bytes()).to_vec()
}

// ═══════════════════════════════════════════════════════════════════
// Permissions catalog
// ═══════════════════════════════════════════════════════════════════

pub async fn permissions_catalog(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageStorage, &state).await?;

    type CatRow = (String, String, String);
    let rows: Vec<CatRow> = sqlx::query_as(
        r#"SELECT key, description, category
             FROM admin_permissions_catalog
         ORDER BY category, key"#,
    )
    .fetch_all(&state.db)
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(|(key, description, category)| json!({ "key": key, "description": description, "category": category }))
        .collect();

    Ok(Json(json!({ "data": data })))
}
