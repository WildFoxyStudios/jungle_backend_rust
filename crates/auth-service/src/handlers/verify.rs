use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::{Value, json};
use shared::{auth::AppState, errors::ApiError};

#[derive(Debug, Deserialize)]
pub struct VerifyEmailRequest {
    pub email: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyPhoneRequest {
    pub phone: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct ResendCodeRequest {
    pub email: Option<String>,
    pub phone: Option<String>,
}

pub async fn verify_email(
    State(state): State<AppState>,
    Json(req): Json<VerifyEmailRequest>,
) -> Result<Json<Value>, ApiError> {
    let updated = sqlx::query(
        r#"
        UPDATE users SET email_verified = TRUE, email_code = NULL
        WHERE email = $1 AND email_code = $2 AND deleted_at IS NULL
        "#,
    )
    .bind(&req.email)
    .bind(&req.code)
    .execute(&state.db)
    .await?
    .rows_affected();

    if updated == 0 {
        return Err(ApiError::BadRequest(
            "Invalid or expired verification code".into(),
        ));
    }

    Ok(Json(json!({ "data": { "verified": true } })))
}

/// POST /v1/auth/verify-email-by-code — Activate account via one-shot
/// code from an email link (matches PHP `/activation/<code>`).
///
/// Unlike `verify_email`, this variant does **not** require the caller to
/// know which email the code was issued for: we look up the row by the
/// unique, time-limited `email_code` column directly. This is what the
/// `/activate/[code]` page in the frontend uses.
#[derive(Debug, Deserialize)]
pub struct VerifyByCodeRequest {
    pub code: String,
}

pub async fn verify_email_by_code(
    State(state): State<AppState>,
    Json(req): Json<VerifyByCodeRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.code.trim().is_empty() {
        return Err(ApiError::BadRequest("Code is required".into()));
    }

    let updated = sqlx::query(
        r#"UPDATE users
              SET email_verified = TRUE, email_code = NULL
            WHERE email_code = $1
              AND email_verified = FALSE
              AND deleted_at IS NULL"#,
    )
    .bind(req.code.trim())
    .execute(&state.db)
    .await?
    .rows_affected();

    if updated == 0 {
        return Err(ApiError::BadRequest(
            "Invalid or expired activation link".into(),
        ));
    }

    Ok(Json(json!({
        "data": {
            "verified": true,
            "message": "Your account has been activated."
        }
    })))
}

pub async fn verify_phone(
    State(state): State<AppState>,
    Json(req): Json<VerifyPhoneRequest>,
) -> Result<Json<Value>, ApiError> {
    // Check code from Redis (SMS verification codes stored with TTL)
    let mut redis = state.redis.clone();
    let stored_code: Option<String> = redis::cmd("GET")
        .arg(format!("phone_verify:{}", req.phone))
        .query_async(&mut redis)
        .await
        .ok();

    let stored = stored_code.ok_or_else(|| ApiError::BadRequest("Code expired".into()))?;
    if stored != req.code {
        return Err(ApiError::BadRequest("Invalid verification code".into()));
    }

    sqlx::query(
        "UPDATE users SET phone_verified = TRUE WHERE phone_number = $1 AND deleted_at IS NULL",
    )
    .bind(&req.phone)
    .execute(&state.db)
    .await?;

    let _: Result<(), _> = redis::cmd("DEL")
        .arg(format!("phone_verify:{}", req.phone))
        .query_async(&mut redis)
        .await;

    Ok(Json(json!({ "data": { "verified": true } })))
}

pub async fn resend_verification(
    State(state): State<AppState>,
    Json(req): Json<ResendCodeRequest>,
) -> Result<Json<Value>, ApiError> {
    if let Some(email) = &req.email {
        let code = format!("{:06}", rand::random::<u32>() % 1_000_000);

        let updated = sqlx::query(
            "UPDATE users SET email_code = $1 WHERE email = $2 AND email_verified = FALSE AND deleted_at IS NULL",
        )
        .bind(&code)
        .bind(email)
        .execute(&state.db)
        .await?
        .rows_affected();

        if updated > 0 {
            let site_name = std::env::var("SITE_NAME").unwrap_or_else(|_| "Jungle".into());
            let (subject, html_body) =
                shared::email_templates::verification_email(&code, &site_name);
            if let Err(e) = shared::email::send_email(email, &subject, &html_body).await {
                tracing::error!(email = %email, error = %e, "Failed to send verification email");
            }
        }
    }

    if let Some(phone) = &req.phone {
        let code = format!("{:06}", rand::random::<u32>() % 1_000_000);

        let mut redis = state.redis.clone();
        let _: Result<(), _> = redis::cmd("SET")
            .arg(format!("phone_verify:{}", phone))
            .arg(&code)
            .arg("EX")
            .arg(600) // 10 minutes
            .query_async(&mut redis)
            .await;

        let site_name = std::env::var("SITE_NAME").unwrap_or_else(|_| "Jungle".into());
        let sms_body = format!("{} - Your verification code is: {}", site_name, code);
        if let Err(e) = shared::sms::send_sms(phone, &sms_body).await {
            tracing::error!(phone = %phone, error = %e, "Failed to send SMS verification");
        }
    }

    Ok(Json(
        json!({ "data": { "message": "Verification code sent" } }),
    ))
}
