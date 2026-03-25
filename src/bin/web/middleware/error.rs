//! 错误处理中间件

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response, Json},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 错误处理中间件
#[derive(Debug, Clone)]
pub struct ErrorMiddleware;

impl ErrorMiddleware {
    /// 错误处理中间件处理函数
    pub async fn handle_errors(
        request: Request,
        next: Next,
    ) -> Result<Response, AppError> {
        match next.run(request).await {
            response => Ok(response),
        }
    }
}

/// 应用错误类型
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.clone()),
            AppError::Timeout => (StatusCode::REQUEST_TIMEOUT, "Request timeout".to_string()),
            AppError::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, "Rate limit exceeded".to_string()),
            AppError::ValidationError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            AppError::JsonError(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            AppError::IoError(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };

        let body = ErrorResponse {
            error: ErrorBody {
                code: status.as_u16(),
                message: error_message,
                type: self.error_type(),
            },
        };

        (status, Json(body)).into_response()
    }
}

/// 错误类型标识
impl AppError {
    fn error_type(&self) -> &'static str {
        match self {
            AppError::BadRequest(_) => "bad_request",
            AppError::Unauthorized(_) => "unauthorized",
            AppError::Forbidden(_) => "forbidden",
            AppError::NotFound(_) => "not_found",
            AppError::Conflict(_) => "conflict",
            AppError::Internal(_) => "internal_error",
            AppError::ServiceUnavailable(_) => "service_unavailable",
            AppError::Timeout => "timeout",
            AppError::RateLimitExceeded => "rate_limit_exceeded",
            AppError::ValidationError(_) => "validation_error",
            AppError::JsonError(_) => "json_error",
            AppError::IoError(_) => "io_error",
        }
    }
}

/// 应用结果类型
pub type AppResult<T> = Result<T, AppError>;

/// 错误响应体
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: u16,
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: &'static str,
}

/// 扩展 axum 的 StatusCode 处理
pub trait StatusCodeExt {
    fn to_app_error(self) -> AppError;
}

impl StatusCodeExt for StatusCode {
    fn to_app_error(self) -> AppError {
        match self.as_u16() {
            400 => AppError::BadRequest("Bad request".to_string()),
            401 => AppError::Unauthorized("Unauthorized".to_string()),
            403 => AppError::Forbidden("Forbidden".to_string()),
            404 => AppError::NotFound("Not found".to_string()),
            408 => AppError::Timeout,
            409 => AppError::Conflict("Conflict".to_string()),
            422 => AppError::ValidationError("Validation failed".to_string()),
            429 => AppError::RateLimitExceeded,
            503 => AppError::ServiceUnavailable("Service unavailable".to_string()),
            _ => AppError::Internal(format!("Unexpected error: {}", self)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_serialization() {
        let error = AppError::BadRequest("Invalid input".to_string());
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_status_code_ext() {
        let status = StatusCode::NOT_FOUND;
        let error = status.to_app_error();

        assert!(matches!(error, AppError::NotFound(_)));
    }
}
