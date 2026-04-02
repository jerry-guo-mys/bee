//! Gateway JWT 认证集成
//!
//! 提供基于 JWT 的用户认证和租户上下文提取

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

/// JWT Claims 结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// 用户 ID
    pub sub: String,
    /// 租户 ID
    pub tenant_id: Option<String>,
    /// 组织 ID
    pub organization_id: Option<String>,
    /// 团队 ID
    pub team_id: Option<String>,
    /// 成员角色
    pub role: Option<String>,
    /// 过期时间
    pub exp: usize,
    /// 签发时间
    pub iat: usize,
}

/// JWT 认证器
pub struct JwtAuthenticator {
    secret: String,
    validation: Validation,
}

impl JwtAuthenticator {
    /// 创建新的 JWT 认证器
    pub fn new(secret: String) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.leeway = 60; // 60 秒宽限期

        Self { secret, validation }
    }

    /// 验证并解析 JWT Token
    pub fn verify(&self, token: &str) -> Result<JwtClaims, JwtAuthError> {
        let decoding_key = DecodingKey::from_secret(self.secret.as_bytes());

        let token_data = decode::<JwtClaims>(token, &decoding_key, &self.validation)?;

        Ok(token_data.claims)
    }

    /// 从请求头中提取 Token
    pub fn extract_from_header(header_value: &str) -> Option<&str> {
        header_value
            .strip_prefix("Bearer ")
            .or_else(|| header_value.strip_prefix("bearer "))
    }
}

/// JWT 认证错误
#[derive(Debug, thiserror::Error)]
pub enum JwtAuthError {
    #[error("Invalid token: {0}")]
    InvalidToken(#[from] jsonwebtoken::errors::Error),

    #[error("Token expired")]
    TokenExpired,

    #[error("Missing tenant context")]
    MissingTenantContext,

    #[error("Invalid claims")]
    InvalidClaims,
}

/// 认证上下文
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// 用户 ID
    pub user_id: String,
    /// 租户 ID
    pub tenant_id: Option<String>,
    /// 组织 ID
    pub organization_id: Option<String>,
    /// 团队 ID
    pub team_id: Option<String>,
    /// 成员角色
    pub role: Option<String>,
}

impl AuthContext {
    /// 从 JWT Claims 创建认证上下文
    pub fn from_claims(claims: &JwtClaims) -> Result<Self, JwtAuthError> {
        Ok(Self {
            user_id: claims.sub.clone(),
            tenant_id: claims.tenant_id.clone(),
            organization_id: claims.organization_id.clone(),
            team_id: claims.team_id.clone(),
            role: claims.role.clone(),
        })
    }

    /// 创建空认证上下文（用于未认证用户）
    pub fn anonymous() -> Self {
        Self {
            user_id: "anonymous".to_string(),
            tenant_id: None,
            organization_id: None,
            team_id: None,
            role: None,
        }
    }

    /// 检查是否是认证用户
    pub fn is_authenticated(&self) -> bool {
        self.user_id != "anonymous"
    }

    /// 检查是否有租户上下文
    pub fn has_tenant_context(&self) -> bool {
        self.tenant_id.is_some()
    }
}

/// 从认证上下文提取客户端元数据
pub fn extract_client_metadata(auth_ctx: &AuthContext) -> Option<serde_json::Value> {
    let mut map = serde_json::Map::new();

    if let Some(ref tenant_id) = auth_ctx.tenant_id {
        map.insert(
            "tenant_id".to_string(),
            serde_json::Value::String(tenant_id.clone()),
        );
    }

    if let Some(ref org_id) = auth_ctx.organization_id {
        map.insert(
            "organization_id".to_string(),
            serde_json::Value::String(org_id.clone()),
        );
    }

    if let Some(ref team_id) = auth_ctx.team_id {
        map.insert(
            "team_id".to_string(),
            serde_json::Value::String(team_id.clone()),
        );
    }

    if let Some(ref role) = auth_ctx.role {
        map.insert("role".to_string(), serde_json::Value::String(role.clone()));
    }

    if map.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn create_test_token(secret: &str, tenant_id: Option<&str>) -> String {
        use jsonwebtoken::{encode, EncodingKey, Header};

        let now = Utc::now();
        let claims = JwtClaims {
            sub: "test-user".to_string(),
            tenant_id: tenant_id.map(String::from),
            organization_id: Some("test-org".to_string()),
            team_id: None,
            role: Some("member".to_string()),
            exp: (now + Duration::hours(1)).timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    #[test]
    fn test_jwt_authenticator_verify_success() {
        let secret = "test-secret".to_string();
        let authenticator = JwtAuthenticator::new(secret.clone());
        let token = create_test_token(&secret, Some("test-tenant"));

        let claims = authenticator.verify(&token);
        assert!(claims.is_ok());
        let claims = claims.unwrap();
        assert_eq!(claims.sub, "test-user");
        assert_eq!(claims.tenant_id, Some("test-tenant".to_string()));
    }

    #[test]
    fn test_jwt_authenticator_verify_wrong_secret() {
        let secret = "test-secret".to_string();
        let authenticator = JwtAuthenticator::new(secret);
        let token = create_test_token("wrong-secret", Some("test-tenant"));

        let result = authenticator.verify(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_from_header_bearer() {
        let header = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...";
        let token = JwtAuthenticator::extract_from_header(header);
        assert_eq!(token, Some("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."));
    }

    #[test]
    fn test_extract_from_header_lowercase() {
        let header = "bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...";
        let token = JwtAuthenticator::extract_from_header(header);
        assert_eq!(token, Some("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."));
    }

    #[test]
    fn test_extract_from_header_no_prefix() {
        let header = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...";
        let token = JwtAuthenticator::extract_from_header(header);
        assert_eq!(token, None);
    }

    #[test]
    fn test_auth_context_from_claims() {
        let claims = JwtClaims {
            sub: "user-123".to_string(),
            tenant_id: Some("tenant-456".to_string()),
            organization_id: Some("org-789".to_string()),
            team_id: Some("team-000".to_string()),
            role: Some("admin".to_string()),
            exp: 0,
            iat: 0,
        };

        let ctx = AuthContext::from_claims(&claims).unwrap();
        assert_eq!(ctx.user_id, "user-123");
        assert_eq!(ctx.tenant_id, Some("tenant-456".to_string()));
        assert!(ctx.is_authenticated());
        assert!(ctx.has_tenant_context());
    }

    #[test]
    fn test_auth_context_anonymous() {
        let ctx = AuthContext::anonymous();
        assert_eq!(ctx.user_id, "anonymous");
        assert!(!ctx.is_authenticated());
        assert!(!ctx.has_tenant_context());
    }

    #[test]
    fn test_extract_client_metadata() {
        let ctx = AuthContext {
            user_id: "user-123".to_string(),
            tenant_id: Some("tenant-456".to_string()),
            organization_id: Some("org-789".to_string()),
            team_id: None,
            role: Some("member".to_string()),
        };

        let metadata = extract_client_metadata(&ctx);
        assert!(metadata.is_some());
        let metadata = metadata.unwrap();
        let map = metadata.as_object().unwrap();
        assert_eq!(map.get("tenant_id").unwrap(), "tenant-456");
        assert_eq!(map.get("organization_id").unwrap(), "org-789");
        assert_eq!(map.get("role").unwrap(), "member");
    }
}
