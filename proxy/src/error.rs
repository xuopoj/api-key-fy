use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("rate limit exceeded")]
    RateLimitExceeded,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Unauthorized => {
                tracing::warn!("401 unauthorized");
                (StatusCode::UNAUTHORIZED, self.to_string())
            }
            AppError::Forbidden => {
                tracing::warn!("403 forbidden");
                (StatusCode::FORBIDDEN, self.to_string())
            }
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::RateLimitExceeded => {
                tracing::warn!("429 rate limit exceeded");
                (StatusCode::TOO_MANY_REQUESTS, self.to_string())
            }
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Internal(e) => {
                tracing::error!("500 internal error: {e:#}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error".to_string())
            }
            AppError::Sqlx(e) => {
                tracing::error!("500 db error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error".to_string())
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
