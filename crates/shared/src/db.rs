use sqlx::postgres::{PgPool, PgPoolOptions};

pub async fn create_pool(database_url: &str) -> PgPool {
    let max_conn: u32 = std::env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);

    PgPoolOptions::new()
        .max_connections(max_conn)
        .min_connections(0)
        .acquire_timeout(std::time::Duration::from_secs(10))
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .connect(database_url)
        .await
        .expect("Failed to connect to PostgreSQL")
}

pub async fn run_migrations(pool: &PgPool) {
    let skip = std::env::var("SKIP_DB_MIGRATIONS")
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            v == "1" || v == "true" || v == "yes"
        })
        .unwrap_or(false);
    if skip {
        tracing::info!("Skipping database migrations (SKIP_DB_MIGRATIONS=true)");
        return;
    }

    sqlx::migrate!("../../migrations")
        .run(pool)
        .await
        .expect("Failed to run database migrations");
}
