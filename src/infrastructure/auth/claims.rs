use chrono::Utc;
use serde::{Deserialize, Serialize};

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeeClaims {
    /// 标准字段
    pub sub: String,      // 用户 ID
    pub exp: usize,       // 过期时间 (Unix timestamp)
    pub iat: usize,       // 签发时间
    pub iss: String,      // 签发者

    /// 自定义字段
    pub user_id: String,
    pub tenant_id: Option<String>,
    pub organization_id: Option<String>,
    pub team_id: Option<String>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
}

impl BeeClaims {
    pub fn new(
        user_id: &str,
        tenant_id: Option<&str>,
        organization_id: Option<&str>,
        team_id: Option<&str>,
        roles: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        let exp = now + chrono::Duration::days(1);

        Self {
            sub: user_id.to_string(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            iss: "bee-agents".to_string(),
            user_id: user_id.to_string(),
            tenant_id: tenant_id.map(String::from),
            organization_id: organization_id.map(String::from),
            team_id: team_id.map(String::from),
            roles,
            permissions: vec![],
        }
    }

    /// 检查是否包含指定角色
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// 检查是否包含指定权限
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
            || self.has_role("PlatformAdmin")
    }
}
