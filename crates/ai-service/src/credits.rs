//! Credit tracking: per-user words + image balance, automatic reset per plan cycle.

use shared::errors::ApiError;
use sqlx::PgPool;
use time::OffsetDateTime;

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AiCredits {
    pub user_id: i64,
    pub words_remaining: i32,
    pub images_remaining: i32,
    pub words_limit: i32,
    pub images_limit: i32,
    pub plan: String,
    pub reset_at: OffsetDateTime,
}

#[derive(Debug, Copy, Clone)]
pub enum CreditKind {
    Words(i32),
    Images(i32),
}

/// Fetch or initialize a user's credits. Creates a free-plan row if missing.
/// Automatically resets the counters if `reset_at` has passed.
pub async fn get_or_init(pool: &PgPool, user_id: i64) -> Result<AiCredits, ApiError> {
    let existing = sqlx::query_as::<_, AiCredits>(
        r#"SELECT user_id, words_remaining, images_remaining, words_limit, images_limit,
                  plan, reset_at
             FROM user_ai_credits
            WHERE user_id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = existing {
        if row.reset_at <= OffsetDateTime::now_utc() {
            reset_cycle(pool, user_id, &row.plan).await
        } else {
            Ok(row)
        }
    } else {
        init_free(pool, user_id).await
    }
}

/// Look up free-plan defaults and insert a new row.
async fn init_free(pool: &PgPool, user_id: i64) -> Result<AiCredits, ApiError> {
    let (words, images, days) = plan_defaults(pool, "free").await?;

    let now = OffsetDateTime::now_utc();
    let reset = now + time::Duration::days(days as i64);

    sqlx::query(
        r#"INSERT INTO user_ai_credits
            (user_id, words_remaining, images_remaining, words_limit, images_limit, plan, reset_at)
           VALUES ($1, $2, $3, $2, $3, 'free', $4)"#,
    )
    .bind(user_id)
    .bind(words)
    .bind(images)
    .bind(reset)
    .execute(pool)
    .await?;

    Ok(AiCredits {
        user_id,
        words_remaining: words,
        images_remaining: images,
        words_limit: words,
        images_limit: images,
        plan: "free".into(),
        reset_at: reset,
    })
}

async fn reset_cycle(pool: &PgPool, user_id: i64, plan: &str) -> Result<AiCredits, ApiError> {
    let (words, images, days) = plan_defaults(pool, plan).await?;
    let reset = OffsetDateTime::now_utc() + time::Duration::days(days as i64);

    let row = sqlx::query_as::<_, AiCredits>(
        r#"UPDATE user_ai_credits
              SET words_remaining = $2, images_remaining = $3,
                  words_limit = $2, images_limit = $3,
                  reset_at = $4, updated_at = NOW()
            WHERE user_id = $1
        RETURNING user_id, words_remaining, images_remaining, words_limit, images_limit,
                  plan, reset_at"#,
    )
    .bind(user_id)
    .bind(words)
    .bind(images)
    .bind(reset)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

async fn plan_defaults(pool: &PgPool, plan: &str) -> Result<(i32, i32, i32), ApiError> {
    let row: Option<(i32, i32, i32)> = sqlx::query_as(
        r#"SELECT words_per_cycle, images_per_cycle, cycle_days
             FROM ai_plan_credits WHERE plan = $1"#,
    )
    .bind(plan)
    .fetch_optional(pool)
    .await?;

    Ok(row.unwrap_or((2000, 5, 30)))
}

/// Attempt to deduct credits atomically. Returns Err(Forbidden) if insufficient.
pub async fn deduct(pool: &PgPool, user_id: i64, kind: CreditKind) -> Result<(), ApiError> {
    // Ensure row exists
    let _ = get_or_init(pool, user_id).await?;

    let affected = match kind {
        CreditKind::Words(n) => sqlx::query(
            r#"UPDATE user_ai_credits
                      SET words_remaining = words_remaining - $2,
                          updated_at = NOW()
                    WHERE user_id = $1 AND words_remaining >= $2"#,
        )
        .bind(user_id)
        .bind(n)
        .execute(pool)
        .await?
        .rows_affected(),
        CreditKind::Images(n) => sqlx::query(
            r#"UPDATE user_ai_credits
                      SET images_remaining = images_remaining - $2,
                          updated_at = NOW()
                    WHERE user_id = $1 AND images_remaining >= $2"#,
        )
        .bind(user_id)
        .bind(n)
        .execute(pool)
        .await?
        .rows_affected(),
    };

    if affected == 0 {
        return Err(ApiError::Forbidden(
            "Insufficient AI credits. Upgrade your plan or wait for the next cycle.".into(),
        ));
    }
    Ok(())
}

pub struct UsageLog<'a> {
    pub user_id: i64,
    pub provider: &'a str,
    pub kind: &'a str,
    pub tokens_used: i32,
    pub images_generated: i32,
    pub cost_cents: i32,
    pub success: bool,
    pub error_message: Option<&'a str>,
}

pub async fn log_usage(pool: &PgPool, entry: UsageLog<'_>) {
    let _ = sqlx::query(
        r#"INSERT INTO ai_usage_log
            (user_id, provider, kind, tokens_used, images_generated,
             cost_cents, success, error_message)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
    )
    .bind(entry.user_id)
    .bind(entry.provider)
    .bind(entry.kind)
    .bind(entry.tokens_used)
    .bind(entry.images_generated)
    .bind(entry.cost_cents)
    .bind(entry.success)
    .bind(entry.error_message)
    .execute(pool)
    .await;
}
