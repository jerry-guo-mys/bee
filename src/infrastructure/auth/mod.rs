//! 认证基础设施

pub mod claims;
pub mod jwt;

#[cfg(feature = "web")]
pub mod middleware;

pub use claims::BeeClaims;
pub use jwt::{JwtError, JwtService};

#[cfg(feature = "web")]
pub use middleware::{jwt_middleware, optional_jwt_middleware, require_role, AuthState};
