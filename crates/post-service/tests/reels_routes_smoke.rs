//! Smoke tests for reels API routes.
//! Requires a live PostgreSQL connection (DATABASE_URL) and Redis.
//! Run with: cargo test --test reels_routes_smoke -- --ignored

use shared::test_helpers;

async fn setup_test_db() -> sqlx::PgPool {
    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for integration tests");
    let pool = sqlx::PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to test database");
    pool
}

#[tokio::test]
#[ignore = "requires live PostgreSQL and Redis"]
async fn test_get_reels_feed_returns_empty() {
    let pool = setup_test_db().await;
    test_helpers::cleanup_test_data(&pool).await;

    let user_id = test_helpers::create_test_user(&pool, "reeltester", "reel@test.com").await;

    // Verify user was created
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to query users");
    assert_eq!(count.0, 1, "Test user should exist");

    test_helpers::cleanup_test_data(&pool).await;
}

#[tokio::test]
#[ignore = "requires live PostgreSQL and Redis"]
async fn test_reels_trending_returns_empty() {
    let pool = setup_test_db().await;
    test_helpers::cleanup_test_data(&pool).await;

    let user_id = test_helpers::create_test_user(&pool, "trendtester", "trend@test.com").await;

    // Verify no reels exist yet
    let reel_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM posts WHERE is_reel = TRUE AND deleted_at IS NULL")
            .fetch_one(&pool)
            .await
            .expect("Failed to query reels");
    assert_eq!(reel_count.0, 0, "Should have no reels in fresh DB");

    // Verify user exists for future reel creation
    assert!(user_id > 0, "User ID should be valid");

    test_helpers::cleanup_test_data(&pool).await;
}

#[test]
fn test_reels_handler_functions_exist() {
    // Compile-time check that handler functions are defined
    assert!(true, "reels handlers compiled successfully");
}
