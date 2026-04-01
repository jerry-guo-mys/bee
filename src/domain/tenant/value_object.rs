//! 租户值对象定义
//!
//! 值对象是不可变的，通过其属性值来定义相等性。
//! 本模块包含租户聚合根使用的所有值对象。

use crate::domain::common::generate_id;
use thiserror::Error;

/// 租户相关错误类型
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TenantError {
    #[error("无效的租户名称：{0}")]
    InvalidName(String),

    #[error("无效的租户 slug: {0}")]
    InvalidSlug(String),

    #[error("租户不存在：{0}")]
    NotFound(String),

    #[error("租户已存在：{0}")]
    AlreadyExists(String),

    #[error("租户状态无效：{0}")]
    InvalidStatus(String),

    #[error("数据库错误：{0}")]
    DatabaseError(String),
}

/// 租户 ID - 封装 UUID 字符串
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TenantId(String);

impl TenantId {
    /// 生成新的 TenantId
    pub fn generate() -> Self {
        Self(generate_id())
    }

    /// 从字符串创建 TenantId
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// 从字符串切片创建 TenantId
    pub fn from_str(id: &str) -> Self {
        Self(id.to_string())
    }

    /// 获取底层字符串引用
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for TenantId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for TenantId {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl std::fmt::Display for TenantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 租户名称 - 验证长度在 1-255 之间
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TenantName(String);

impl TenantName {
    const MIN_LEN: usize = 1;
    const MAX_LEN: usize = 255;

    /// 创建新的 TenantName，会进行验证
    pub fn new(name: String) -> Result<Self, TenantError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(TenantError::InvalidName("名称不能为空".to_string()));
        }
        if trimmed.len() < Self::MIN_LEN {
            return Err(TenantError::InvalidName(format!(
                "名称长度不能少于 {} 个字符",
                Self::MIN_LEN
            )));
        }
        if trimmed.len() > Self::MAX_LEN {
            return Err(TenantError::InvalidName(format!(
                "名称长度不能超过 {} 个字符",
                Self::MAX_LEN
            )));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// 获取底层字符串引用
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for TenantName {
    type Error = TenantError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl std::fmt::Display for TenantName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 租户 Slug - URL 友好的标识符，仅允许小写字母和数字
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TenantSlug(String);

impl TenantSlug {
    const MIN_LEN: usize = 1;
    const MAX_LEN: usize = 100;

    /// 创建新的 TenantSlug，会进行验证
    pub fn new(slug: String) -> Result<Self, TenantError> {
        let slug = slug.to_lowercase();

        if slug.is_empty() {
            return Err(TenantError::InvalidSlug("slug 不能为空".to_string()));
        }
        if slug.len() < Self::MIN_LEN {
            return Err(TenantError::InvalidSlug(format!(
                "slug 长度不能少于 {} 个字符",
                Self::MIN_LEN
            )));
        }
        if slug.len() > Self::MAX_LEN {
            return Err(TenantError::InvalidSlug(format!(
                "slug 长度不能超过 {} 个字符",
                Self::MAX_LEN
            )));
        }

        // 验证只包含小写字母、数字和连字符
        if !slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(TenantError::InvalidSlug(
                "slug 只能包含小写字母、数字和连字符 (-)".to_string(),
            ));
        }

        // 验证不能以连字符开头或结尾
        if slug.starts_with('-') || slug.ends_with('-') {
            return Err(TenantError::InvalidSlug(
                "slug 不能以连字符开头或结尾".to_string(),
            ));
        }

        Ok(Self(slug))
    }

    /// 从字符串创建，不验证（用于从数据库加载）
    pub fn from_str_unchecked(slug: &str) -> Self {
        Self(slug.to_string())
    }

    /// 获取底层字符串引用
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for TenantSlug {
    type Error = TenantError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl std::fmt::Display for TenantSlug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 组织 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrganizationId(String);

impl OrganizationId {
    pub fn generate() -> Self {
        Self(generate_id())
    }

    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn from_str(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for OrganizationId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for OrganizationId {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl std::fmt::Display for OrganizationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 团队 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TeamId(String);

impl TeamId {
    pub fn generate() -> Self {
        Self(generate_id())
    }

    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn from_str(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for TeamId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for TeamId {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl std::fmt::Display for TeamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 用户 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(String);

impl UserId {
    pub fn generate() -> Self {
        Self(generate_id())
    }

    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn from_str(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for UserId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for UserId {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Agent ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(String);

impl AgentId {
    pub fn generate() -> Self {
        Self(generate_id())
    }

    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn from_str(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for AgentId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for AgentId {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 成员 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MembershipId(String);

impl MembershipId {
    pub fn generate() -> Self {
        Self(generate_id())
    }

    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn from_str(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for MembershipId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for MembershipId {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl std::fmt::Display for MembershipId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_id_generation() {
        let id1 = TenantId::generate();
        let id2 = TenantId::generate();
        assert_ne!(id1, id2);
        assert_eq!(id1.as_str().len(), 36);
    }

    #[test]
    fn test_tenant_id_from_str() {
        let id = TenantId::from_str("test-uuid-12345");
        assert_eq!(id.as_str(), "test-uuid-12345");
    }

    #[test]
    fn test_tenant_name_valid() {
        let name = TenantName::new("Test Tenant".to_string()).unwrap();
        assert_eq!(name.as_str(), "Test Tenant");
    }

    #[test]
    fn test_tenant_name_empty() {
        let result = TenantName::new("".to_string());
        assert!(matches!(result, Err(TenantError::InvalidName(_))));
    }

    #[test]
    fn test_tenant_name_whitespace_only() {
        let result = TenantName::new("   ".to_string());
        assert!(matches!(result, Err(TenantError::InvalidName(_))));
    }

    #[test]
    fn test_tenant_name_too_long() {
        let long_name = "a".repeat(300);
        let result = TenantName::new(long_name);
        assert!(matches!(result, Err(TenantError::InvalidName(_))));
    }

    #[test]
    fn test_tenant_name_trimmed() {
        let name = TenantName::new("  Test Tenant  ".to_string()).unwrap();
        assert_eq!(name.as_str(), "Test Tenant");
    }

    #[test]
    fn test_tenant_slug_valid() {
        let slug = TenantSlug::new("test-slug-123".to_string()).unwrap();
        assert_eq!(slug.as_str(), "test-slug-123");
    }

    #[test]
    fn test_tenant_slug_lowercase_conversion() {
        let slug = TenantSlug::new("TEST-SLUG".to_string()).unwrap();
        assert_eq!(slug.as_str(), "test-slug");
    }

    #[test]
    fn test_tenant_slug_empty() {
        let result = TenantSlug::new("".to_string());
        assert!(matches!(result, Err(TenantError::InvalidSlug(_))));
    }

    #[test]
    fn test_tenant_slug_invalid_chars() {
        let result = TenantSlug::new("test_slug".to_string());
        assert!(matches!(result, Err(TenantError::InvalidSlug(_))));

        let result = TenantSlug::new("test.slug".to_string());
        assert!(matches!(result, Err(TenantError::InvalidSlug(_))));

        let result = TenantSlug::new("test slug".to_string());
        assert!(matches!(result, Err(TenantError::InvalidSlug(_))));
    }

    #[test]
    fn test_tenant_slug_cannot_start_with_hyphen() {
        let result = TenantSlug::new("-test-slug".to_string());
        assert!(matches!(result, Err(TenantError::InvalidSlug(_))));
    }

    #[test]
    fn test_tenant_slug_cannot_end_with_hyphen() {
        let result = TenantSlug::new("test-slug-".to_string());
        assert!(matches!(result, Err(TenantError::InvalidSlug(_))));
    }

    #[test]
    fn test_tenant_slug_too_long() {
        let long_slug = "a".repeat(150);
        let result = TenantSlug::new(long_slug);
        assert!(matches!(result, Err(TenantError::InvalidSlug(_))));
    }

    #[test]
    fn test_organization_id() {
        let id1 = OrganizationId::generate();
        let id2 = OrganizationId::generate();
        assert_ne!(id1, id2);

        let id3 = OrganizationId::from_str("org-123");
        assert_eq!(id3.as_str(), "org-123");
    }

    #[test]
    fn test_team_id() {
        let id = TeamId::generate();
        assert_eq!(id.as_str().len(), 36);

        let id2 = TeamId::from("team-456");
        assert_eq!(id2.as_str(), "team-456");
    }

    #[test]
    fn test_user_id() {
        let id = UserId::generate();
        assert_eq!(id.as_str().len(), 36);

        let id2 = UserId::from_str("user-789");
        assert_eq!(id2.as_str(), "user-789");
    }

    #[test]
    fn test_agent_id() {
        let id = AgentId::generate();
        assert_eq!(id.as_str().len(), 36);

        let id2 = AgentId::new("agent-abc".to_string());
        assert_eq!(id2.as_str(), "agent-abc");
    }

    #[test]
    fn test_membership_id() {
        let id = MembershipId::generate();
        assert_eq!(id.as_str().len(), 36);
    }
}
