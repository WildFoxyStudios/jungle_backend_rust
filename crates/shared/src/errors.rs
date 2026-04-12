use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Validation error")]
    Validation(Vec<FieldError>),

    #[error("Too many requests")]
    RateLimited,

    #[error("Internal server error: {0}")]
    Internal(String),
}

#[derive(Debug, serde::Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message, details) = match &self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg.clone(), None),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", "Authentication required".into(), None),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", if msg.is_empty() { "Access denied".into() } else { msg.clone() }, None),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone(), None),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg.clone(), None),
            ApiError::Validation(errors) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "VALIDATION_ERROR",
                "Invalid input".into(),
                Some(serde_json::to_value(errors).unwrap_or_default()),
            ),
            ApiError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED", "Too many requests".into(), None),
            ApiError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "Internal server error".into(), None)
            }
        };

        let body = if let Some(details) = details {
            json!({ "error": { "code": code, "message": message, "details": details } })
        } else {
            json!({ "error": { "code": code, "message": message } })
        };

        (status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        tracing::error!("Database error: {:?}", e);
        match e {
            sqlx::Error::RowNotFound => ApiError::NotFound("Resource not found".into()),
            sqlx::Error::Database(db_err) => {
                if db_err.code().as_deref() == Some("23505") {
                    ApiError::Conflict("Resource already exists".into())
                } else {
                    ApiError::Internal("Database error".into())
                }
            }
            _ => ApiError::Internal("Database error".into()),
        }
    }
}

impl From<jsonwebtoken::errors::Error> for ApiError {
    fn from(_: jsonwebtoken::errors::Error) -> Self {
        ApiError::Unauthorized
    }
}

impl From<argon2::password_hash::Error> for ApiError {
    fn from(_: argon2::password_hash::Error) -> Self {
        ApiError::Internal("Password hashing error".into())
    }
}

impl From<redis::RedisError> for ApiError {
    fn from(e: redis::RedisError) -> Self {
        tracing::error!("Redis error: {:?}", e);
        ApiError::Internal("Cache error".into())
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(e: validator::ValidationErrors) -> Self {
        let errors: Vec<FieldError> = e
            .field_errors()
            .into_iter()
            .flat_map(|(field, errs)| {
                errs.iter().map(move |err| FieldError {
                    field: field.to_string(),
                    message: err.message.as_ref().map(|m| m.to_string()).unwrap_or_else(|| format!("Invalid {}", field)),
                })
            })
            .collect();
        ApiError::Validation(errors)
    }
}
