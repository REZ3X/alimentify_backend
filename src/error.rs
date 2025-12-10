use axum::{ http::StatusCode, response::IntoResponse, Json };
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Not found: {0}")] NotFound(String),

    #[error("Bad request: {0}")] BadRequest(String),

    #[error("Internal server error")] InternalError(#[from] anyhow::Error),

    #[allow(dead_code)] #[error("Validation error: {0}")] ValidationError(String),

    #[error("External API unavailable: {0}")] ExternalApiError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::ValidationError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            AppError::InternalError(_) =>
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
            AppError::ExternalApiError(msg) =>
                (StatusCode::SERVICE_UNAVAILABLE, msg),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
