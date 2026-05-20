use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use thiserror::Error;

use crate::openai::{ApiError, ErrorResponse};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Missing required parameter: {0}")]
    MissingParam(String),

    #[error("Upstream error: {0}")]
    UpstreamError(String),

    #[error("Authentication failed")]
    AuthFailed,

    #[error("Timeout")]
    Timeout,

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            Self::MissingParam(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::AuthFailed => StatusCode::UNAUTHORIZED,
            Self::Timeout => StatusCode::GATEWAY_TIMEOUT,
            Self::UpstreamError(_) => StatusCode::BAD_GATEWAY,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn error_code(&self) -> &str {
        match self {
            Self::InvalidRequest(_) => "invalid_request_error",
            Self::MissingParam(_) => "invalid_request_error",
            Self::AuthFailed => "authentication_error",
            Self::Timeout => "timeout",
            Self::UpstreamError(_) => "upstream_error",
            Self::Internal(_) => "internal_error",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error = ApiError {
            message: self.to_string(),
            r#type: self.error_code().to_string(),
            param: None,
            code: Some(self.error_code().to_string()),
        };
        let body = ErrorResponse { error };
        (status, Json(body)).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;