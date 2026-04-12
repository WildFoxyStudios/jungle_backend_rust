use sqlx::PgPool;

pub async fn create_test_user(db: &PgPool, username: &str, email: &str) -> i64 {
    let password_hash = "$argon2id$v=19$m=19456,t=2,p=1$test$testhash";
    sqlx::query_scalar::<_, i64>(
        r#"INSERT INTO users (username, email, password, first_name, last_name, is_active)
           VALUES ($1, $2, $3, 'Test', 'User', TRUE)
           RETURNING id"#,
    )
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .fetch_one(db)
    .await
    .expect("Failed to create test user")
}

pub fn generate_test_jwt(user_id: i64) -> String {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "test-jwt-secret".into());
    let claims = serde_json::json!({
        "sub": user_id,
        "exp": chrono_now_secs() + 3600,
        "iat": chrono_now_secs(),
    });
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("JWT encode failed")
}

fn chrono_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub async fn cleanup_test_data(db: &PgPool) {
    let tables = [
        "reactions",
        "comments",
        "notifications",
        "messages",
        "conversation_members",
        "conversations",
        "follows",
        "blocks",
        "sessions",
        "posts",
        "users",
    ];
    for table in tables {
        let query = format!("DELETE FROM {} WHERE TRUE", table);
        let _ = sqlx::query(&query).execute(db).await;
    }
}
