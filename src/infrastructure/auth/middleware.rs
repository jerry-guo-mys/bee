#![cfg(feature = "web")]

use super::claims::BeeClaims;
use super::jwt::{JwtError, JwtService};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

/// 认证状态
#[derive(Debug, Clone)]
pub enum AuthStatus {
    Authenticated(BeeClaims),
    Unauthenticated,
}

/// 扩展请求中的认证信息
#[derive(Debug, Clone)]
pub struct AuthState {
    pub claims: Option<BeeClaims>,
}

/// JWT 认证中间件
pub async fn jwt_middleware(
    State(jwt_service): State<Arc<JwtService>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h: &axum::http::HeaderValue| h.to_str().ok());

    if let Some(header) = auth_header {
        let token = header.strip_prefix("Bearer ").unwrap_or(header);

        match jwt_service.validate_token(token) {
            Ok(claims) => {
                request.extensions_mut().insert(AuthState {
                    claims: Some(claims),
                });
            }
            Err(JwtError::TokenExpired) => {
                return Err(StatusCode::UNAUTHORIZED);
            }
            Err(_) => {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }

    Ok(next.run(request).await)
}

/// 可选认证中间件（token 存在才验证）
pub async fn optional_jwt_middleware(
    State(jwt_service): State<Arc<JwtService>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h: &axum::http::HeaderValue| h.to_str().ok());

    if let Some(header) = auth_header {
        let token = header.strip_prefix("Bearer ").unwrap_or(header);
        if let Ok(claims) = jwt_service.validate_token(token) {
            request.extensions_mut().insert(AuthState {
                claims: Some(claims),
            });
        }
    }

    next.run(request).await
}

/// 权限检查辅助函数
pub fn require_role(claims: &BeeClaims, required_role: &str) -> Result<(), StatusCode> {
    if claims.has_role(required_role) {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}
