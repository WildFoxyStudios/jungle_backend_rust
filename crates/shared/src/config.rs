use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub nats_url: String,
    pub jwt_secret: String,
    /// Previous JWT secret kept during rotation grace period. Tokens signed
    /// with the old secret (carrying `kid: "previous"`) are still accepted
    /// until they naturally expire.
    pub jwt_secret_previous: Option<String>,
    pub jwt_refresh_secret: String,
    pub server_host: String,
    pub server_port: u16,
    pub frontend_url: String,
    pub allowed_origins: Vec<String>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        Self {
            database_url: std::env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
            nats_url: std::env::var("NATS_URL")
                .unwrap_or_else(|_| "nats://127.0.0.1:4222".into()),
            jwt_secret: std::env::var("JWT_SECRET")
                .expect("JWT_SECRET must be set"),
            jwt_secret_previous: std::env::var("JWT_SECRET_PREVIOUS")
                .ok()
                .filter(|s| !s.is_empty()),
            jwt_refresh_secret: std::env::var("JWT_REFRESH_SECRET")
                .unwrap_or_else(|_| std::env::var("JWT_SECRET").unwrap()),
            server_host: std::env::var("SERVER_HOST")
                .unwrap_or_else(|_| "0.0.0.0".into()),
            server_port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".into())
                .parse()
                .expect("SERVER_PORT must be a valid u16"),
            frontend_url: std::env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:3001".into()),
            allowed_origins: std::env::var("ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3001".into())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        }
    }

    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }
}

pub type SharedConfig = Arc<AppConfig>;
