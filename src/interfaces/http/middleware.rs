//! HTTP 中间件

use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::infrastructure::auth::{BeeClaims, JwtService};

/// 应用状态（用于 JWT 验证）
#[derive(Clone)]
pub struct HttpAuthState {
    pub jwt_service: Arc<JwtService>,
}

/// JWT 认证中间件
///
/// 从 Authorization header 提取 Bearer token，验证后注入用户上下文
pub async fn auth_middleware(
    State(state): State<HttpAuthState>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 从 Authorization header 提取 token
    let auth_header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    let claims = match auth_header {
        Some(header) => {
            // 期望格式：Bearer <token>
            if let Some(token) = header.strip_prefix("Bearer ") {
                match state.jwt_service.validate_token(token) {
                    Ok(claims) => Some(claims),
                    Err(e) => {
                        tracing::warn!(target: "auth", "JWT validation failed: {}", e);
                        return Err(StatusCode::UNAUTHORIZED);
                    }
                }
            } else {
                // Header 格式不正确
                tracing::warn!(target: "auth", "Invalid Authorization header format");
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
        None => {
            // 没有 token，允许匿名访问（某些端点可能不需要认证）
            // 如果需要强制认证，可以在这里返回 Err(StatusCode::UNAUTHORIZED)
            None
        }
    };

    // 将 claims 注入到 request extensions
    if let Some(claims) = claims {
        request.extensions_mut().insert(claims);
    }

    Ok(next.run(request).await)
}

/// 租户上下文提取中间件
///
/// 从 JWT claims 或 header 中提取租户上下文并注入到 request extensions
pub async fn tenant_context_middleware(
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 尝试从 JWT claims 获取租户上下文
    let tenant_context = request
        .extensions()
        .get::<BeeClaims>()
        .and_then(|claims| claims.tenant_id.clone());

    // 如果没有从 claims 获取，尝试从 header 获取
    let tenant_context = tenant_context.or_else(|| {
        request
            .headers()
            .get("X-Tenant-ID")
            .and_then(|value| value.to_str().ok())
            .map(String::from)
    });

    // 注入租户上下文到 request extensions
    if let Some(tenant_id) = tenant_context {
        request.extensions_mut().insert(TenantContext { tenant_id });
    }

    Ok(next.run(request).await)
}

/// 租户上下文
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: String,
}

/// 从 request 中提取租户上下文
pub fn extract_tenant_context<B>(request: &Request<B>) -> Option<TenantContext> {
    request.extensions().get::<TenantContext>().cloned()
}

/// 从 request 中提取用户声明
pub fn extract_claims<B>(request: &Request<B>) -> Option<BeeClaims> {
    request.extensions().get::<BeeClaims>().cloned()
}

/// 权限检查辅助函数
pub fn check_permission(claims: &BeeClaims, required_permission: &str) -> bool {
    claims.has_permission(required_permission)
}

/// 角色检查辅助函数
pub fn check_role(claims: &BeeClaims, required_role: &str) -> bool {
    claims.has_role(required_role)
}
