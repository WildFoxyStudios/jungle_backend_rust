use axum::{
    extract::{Query, State},
    Json,
};
use serde::Serialize;
use serde_json::{json, Value};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;

fn require_admin(auth: &AuthUser) -> Result<(), ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }
    Ok(())
}

#[derive(Debug, Serialize, FromRow)]
pub struct BackupLogRow {
    pub id: i64,
    pub filename: String,
    pub size_bytes: i64,
    pub status: String,
    pub created_at: OffsetDateTime,
}

/// GET /v1/admin/backups — list previous backup logs
pub async fn list_backups(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;
    let limit = params.limit();
    let offset = 0i64;

    let rows = sqlx::query_as::<_, BackupLogRow>(
        r#"SELECT id, filename, size_bytes, status, created_at
           FROM backup_logs
           ORDER BY id DESC LIMIT $1 OFFSET $2"#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Ok(Json(json!({ "data": rows })))
}

/// POST /v1/admin/backups/trigger — initiate a new database backup
pub async fn trigger_backup(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    require_admin(&auth)?;

    let now = OffsetDateTime::now_utc();
    let filename = format!(
        "backup_{:04}{:02}{:02}_{:02}{:02}{:02}.sql.gz",
        now.year(), now.month() as u8, now.day(),
        now.hour(), now.minute(), now.second()
    );

    // Insert pending backup record
    let row = sqlx::query_as::<_, (i64,)>(
        r#"INSERT INTO backup_logs (filename, size_bytes, status)
           VALUES ($1, 0, 'pending')
           RETURNING id"#,
    )
    .bind(&filename)
    .fetch_one(&state.db)
    .await?;

    let backup_id = row.0;

    tracing::info!(
        backup_id,
        filename = %filename,
        triggered_by = auth.user_id,
        "Database backup triggered"
    );

    // Spawn background task to run pg_dump
    let db = state.db.clone();
    let fname = filename.clone();
    tokio::spawn(async move {
        run_pg_dump(db, backup_id, &fname).await;
    });

    Ok(Json(json!({
        "data": {
            "id": backup_id,
            "filename": filename,
            "status": "pending"
        }
    })))
}

async fn run_pg_dump(db: PgPool, backup_id: i64, filename: &str) {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_default();
    let backup_dir = std::env::var("BACKUP_DIR").unwrap_or_else(|_| "/tmp/backups".into());

    // Ensure backup directory exists
    if let Err(e) = tokio::fs::create_dir_all(&backup_dir).await {
        tracing::error!(error = %e, "Failed to create backup directory");
        update_backup_status(&db, backup_id, "failed", 0).await;
        return;
    }

    let output_path = format!("{}/{}", backup_dir, filename);

    // Run pg_dump and pipe through gzip
    let result = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "pg_dump '{}' | gzip > '{}'",
            database_url, output_path
        ))
        .output()
        .await;

    match result {
        Ok(output) if output.status.success() => {
            let size = tokio::fs::metadata(&output_path)
                .await
                .map(|m| m.len() as i64)
                .unwrap_or(0);

            tracing::info!(
                backup_id,
                filename,
                size_bytes = size,
                "Database backup completed"
            );

            update_backup_status(&db, backup_id, "completed", size).await;
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::error!(
                backup_id,
                filename,
                stderr = %stderr,
                "pg_dump failed"
            );
            update_backup_status(&db, backup_id, "failed", 0).await;
        }
        Err(e) => {
            tracing::error!(
                backup_id,
                error = %e,
                "Failed to spawn pg_dump process"
            );
            update_backup_status(&db, backup_id, "failed", 0).await;
        }
    }
}

async fn update_backup_status(db: &PgPool, backup_id: i64, status: &str, size_bytes: i64) {
    sqlx::query("UPDATE backup_logs SET status = $1, size_bytes = $2 WHERE id = $3")
        .bind(status)
        .bind(size_bytes)
        .bind(backup_id)
        .execute(db)
        .await
        .ok();
}
