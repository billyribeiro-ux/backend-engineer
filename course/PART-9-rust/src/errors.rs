use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("{message}")]
    NotFound { message: String },

    #[error("{message}")]
    Unauthorized { message: String },

    #[error("{message}")]
    Forbidden { message: String },

    #[error("{message}")]
    BadRequest { code: String, message: String, field: Option<String> },

    #[error("{message}")]
    Conflict { code: String, message: String },

    #[error("An unexpected error occurred")]
    Internal(#[from] anyhow::Error),

    #[error("Database error")]
    Database(#[from] sqlx::Error),

    #[error("JWT error")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

impl AppError {
    pub fn not_found(resource: &str) -> Self {
        AppError::NotFound {
            message: format!("{} not found.", resource),
        }
    }

    pub fn unauthorized(message: &str) -> Self {
        AppError::Unauthorized { message: message.to_string() }
    }

    pub fn forbidden(message: &str) -> Self {
        AppError::Forbidden { message: message.to_string() }
    }

    pub fn bad_request(code: &str, message: &str) -> Self {
        AppError::BadRequest {
            code: code.to_string(),
            message: message.to_string(),
            field: None,
        }
    }

    pub fn bad_request_field(code: &str, message: &str, field: &str) -> Self {
        AppError::BadRequest {
            code: code.to_string(),
            message: message.to_string(),
            field: Some(field.to_string()),
        }
    }

    pub fn conflict(code: &str, message: &str) -> Self {
        AppError::Conflict {
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message, field) = match &self {
            AppError::NotFound { message } => {
                (StatusCode::NOT_FOUND, "NOT_FOUND", message.as_str(), None)
            }
            AppError::Unauthorized { message } => {
                (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", message.as_str(), None)
            }
            AppError::Forbidden { message } => {
                (StatusCode::FORBIDDEN, "FORBIDDEN", message.as_str(), None)
            }
            AppError::BadRequest { code, message, field } => {
                (StatusCode::BAD_REQUEST, code.as_str(), message.as_str(), field.as_deref())
            }
            AppError::Conflict { code, message } => {
                (StatusCode::CONFLICT, code.as_str(), message.as_str(), None)
            }
            AppError::Database(e) => {
                // Log the real error internally, return a safe message
                tracing::error!("Database error: {:?}", e);
                match e {
                    sqlx::Error::RowNotFound => {
                        (StatusCode::NOT_FOUND, "NOT_FOUND", "Resource not found.", None)
                    }
                    _ => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "INTERNAL_ERROR",
                        "An unexpected error occurred.",
                        None,
                    ),
                }
            }
            AppError::Internal(e) => {
                tracing::error!("Internal error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An unexpected error occurred.",
                    None,
                )
            }
            AppError::Jwt(_) => {
                (StatusCode::UNAUTHORIZED, "INVALID_TOKEN", "Invalid or expired token.", None)
            }
        };

        let body = json!({
            "error": {
                "code": code,
                "message": message,
                "field": field,
            }
        });

        (status, Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
