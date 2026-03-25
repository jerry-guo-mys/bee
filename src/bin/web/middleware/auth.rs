//! 认证中间件

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};

/// 认证中间件
#[derive(Debug, Clone)]
pub struct AuthMiddleware {
    pub enabled: bool,
    pub jwt_secret: Option<String>,
}

impl AuthMiddleware {
    pub fn new(enabled: bool, jwt_secret: Option<String>) -> Self {
        Self { enabled, jwt_secret }
    }

    /// 认证中间件处理函数
    pub async fn authenticate(
        State(state): State<crate::routes::WebAppState>,
        mut request: Request,
        next: Next,
    ) -> Result<Response, (StatusCode, &'static str)> {
        // 如果未启用认证，直接通过
        if !state.config.auth.enabled {
            return Ok(next.run(request).await);
        }

        // 从请求头获取 token
        let auth_header = request
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok());

        match auth_header {
            Some(header) if header.starts_with("Bearer ") => {
                let token = &header[7..];

                // TODO: 验证 JWT token
                if token.is_empty() {
                    return Err((StatusCode::UNAUTHORIZED, "Invalid token"));
                }

                // 将用户信息注入请求
                request.extensions_mut().insert(AuthState::Authenticated {
                    user_id: "user-123".to_string(),
                    roles: vec!["user".to_string()],
                });

                Ok(next.run(request).await)
            }
            _ => Err((StatusCode::UNAUTHORIZED, "Missing authorization header")),
        }
    }
}

/// 认证状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthState {
    Authenticated {
        user_id: String,
        roles: Vec<String>,
    },
    Anonymous,
}

impl AuthState {
    pub fn is_authenticated(&self) -> bool {
        matches!(self, AuthState::Authenticated { .. })
    }

    pub fn user_id(&self) -> Option<&str> {
        match self {
            AuthState::Authenticated { user_id, .. } => Some(user_id),
            AuthState::Anonymous => None,
        }
    }

    pub fn has_role(&self, role: &str) -> bool {
        match self {
            AuthState::Authenticated { roles, .. } => roles.contains(&role.to_string()),
            AuthState::Anonymous => false,
        }
    }
}

/// 需要认证的路由处理器包装器
pub async fn require_auth(
    State(state): State<crate::routes::WebAppState>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    AuthMiddleware::authenticate(State(state), request, next).await
}

/// API Key 认证
#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    pub api_key: String,
}

impl ApiKeyAuth {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub async fn authenticate(
        &self,
        mut request: Request,
        next: Next,
    ) -> Result<Response, (StatusCode, &'static str)> {
        let api_key_header = request
            .headers()
            .get("X-API-Key")
            .and_then(|h| h.to_str().ok());

        match api_key_header {
            Some(key) if key == self.api_key => {
                request.extensions_mut().insert(AuthState::Authenticated {
                    user_id: "api-user".to_string(),
                    roles: vec!["api".to_string()],
                });
                Ok(next.run(request).await)
            }
            _ => Err((StatusCode::UNAUTHORIZED, "Invalid API key")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_state() {
        let authenticated = AuthState::Authenticated {
            user_id: "user-123".to_string(),
            roles: vec!["admin".to_string()],
        };

        assert!(authenticated.is_authenticated());
        assert_eq!(authenticated.user_id(), Some("user-123"));
        assert!(authenticated.has_role("admin"));
        assert!(!authenticated.has_role("guest"));

        let anonymous = AuthState::Anonymous;
        assert!(!anonymous.is_authenticated());
        assert!(anonymous.user_id().is_none());
    }
}
