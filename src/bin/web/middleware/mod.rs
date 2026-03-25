//! Web 中间件模块
//!
//! 实现认证、限流、日志等 HTTP 中间件

pub mod auth;
pub mod rate_limit;
pub mod logging;
pub mod cors;
pub mod error;

pub use auth::{AuthMiddleware, AuthState, require_auth};
pub use rate_limit::{RateLimitMiddleware, RateLimiter};
pub use logging::LoggingMiddleware;
pub use cors::setup_cors;
pub use error::{ErrorMiddleware, AppError, AppResult};
