//! Organization 值对象定义
//!
//! 值对象是不可变的，通过其属性值来定义相等性。
//! 本模块包含组织聚合根使用的所有值对象。

use crate::domain::common::generate_id;
use thiserror::Error;

/// 组织相关错误类型
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum OrganizationError {
    #[error("无效的组织名称：{0}")]
    InvalidName(String),

    #[error("无效的组织 slug: {0}")]
    InvalidSlug(String),

    #[error("组织不存在：{0}")]
    NotFound(String),

    #[error("组织已存在：{0}")]
    AlreadyExists(String),

    #[error("数据库错误：{0}")]
    DatabaseError(String),
}

/// 组织 ID - 封装 UUID 字符串
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrganizationId(String);

impl OrganizationId {
    /// 生成新的 OrganizationId
    pub fn generate() -> Self {
        Self(generate_id())
    }

    /// 从字符串创建 OrganizationId
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// 从字符串切片创建 OrganizationId
    pub fn from_str(id: &str) -> Self {
        Self(id.to_string())
    }

    /// 获取底层字符串引用
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

/// 组织名称 - 验证长度在 1-255 之间
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrganizationName(String);

impl OrganizationName {
    const MIN_LEN: usize = 1;
    const MAX_LEN: usize = 255;

    /// 创建新的 OrganizationName，会进行验证
    pub fn new(name: String) -> Result<Self, OrganizationError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(OrganizationError::InvalidName("名称不能为空".to_string()));
        }
        if trimmed.len() < Self::MIN_LEN {
            return Err(OrganizationError::InvalidName(format!(
                "名称长度不能少于 {} 个字符",
                Self::MIN_LEN
            )));
        }
        if trimmed.len() > Self::MAX_LEN {
            return Err(OrganizationError::InvalidName(format!(
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

impl TryFrom<String> for OrganizationName {
    type Error = OrganizationError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl std::fmt::Display for OrganizationName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 组织 Slug - URL 友好的标识符，仅允许小写字母、数字和连字符
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OrganizationSlug(String);

impl OrganizationSlug {
    const MIN_LEN: usize = 1;
    const MAX_LEN: usize = 100;

    /// 创建新的 OrganizationSlug，会进行验证
    pub fn new(slug: String) -> Result<Self, OrganizationError> {
        let slug = slug.to_lowercase();

        if slug.is_empty() {
            return Err(OrganizationError::InvalidSlug("slug 不能为空".to_string()));
        }
        if slug.len() < Self::MIN_LEN {
            return Err(OrganizationError::InvalidSlug(format!(
                "slug 长度不能少于 {} 个字符",
                Self::MIN_LEN
            )));
        }
        if slug.len() > Self::MAX_LEN {
            return Err(OrganizationError::InvalidSlug(format!(
                "slug 长度不能超过 {} 个字符",
                Self::MAX_LEN
            )));
        }

        // 验证只包含小写字母、数字和连字符
        if !slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(OrganizationError::InvalidSlug(
                "slug 只能包含小写字母、数字和连字符 (-)".to_string(),
            ));
        }

        // 验证不能以连字符开头或结尾
        if slug.starts_with('-') || slug.ends_with('-') {
            return Err(OrganizationError::InvalidSlug(
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

impl TryFrom<String> for OrganizationSlug {
    type Error = OrganizationError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl std::fmt::Display for OrganizationSlug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_organization_id_generation() {
        let id1 = OrganizationId::generate();
        let id2 = OrganizationId::generate();
        assert_ne!(id1, id2);
        assert_eq!(id1.as_str().len(), 36);
    }

    #[test]
    fn test_organization_id_from_str() {
        let id = OrganizationId::from_str("test-org-id");
        assert_eq!(id.as_str(), "test-org-id");
    }

    #[test]
    fn test_organization_name_valid() {
        let name = OrganizationName::new("Test Organization".to_string()).unwrap();
        assert_eq!(name.as_str(), "Test Organization");
    }

    #[test]
    fn test_organization_name_empty() {
        let result = OrganizationName::new("".to_string());
        assert!(matches!(result, Err(OrganizationError::InvalidName(_))));
    }

    #[test]
    fn test_organization_name_whitespace_only() {
        let result = OrganizationName::new("   ".to_string());
        assert!(matches!(result, Err(OrganizationError::InvalidName(_))));
    }

    #[test]
    fn test_organization_name_too_long() {
        let long_name = "a".repeat(300);
        let result = OrganizationName::new(long_name);
        assert!(matches!(result, Err(OrganizationError::InvalidName(_))));
    }

    #[test]
    fn test_organization_name_trimmed() {
        let name = OrganizationName::new("  Test Org  ".to_string()).unwrap();
        assert_eq!(name.as_str(), "Test Org");
    }

    #[test]
    fn test_organization_slug_valid() {
        let slug = OrganizationSlug::new("test-slug-123".to_string()).unwrap();
        assert_eq!(slug.as_str(), "test-slug-123");
    }

    #[test]
    fn test_organization_slug_lowercase_conversion() {
        let slug = OrganizationSlug::new("TEST-SLUG".to_string()).unwrap();
        assert_eq!(slug.as_str(), "test-slug");
    }

    #[test]
    fn test_organization_slug_empty() {
        let result = OrganizationSlug::new("".to_string());
        assert!(matches!(result, Err(OrganizationError::InvalidSlug(_))));
    }

    #[test]
    fn test_organization_slug_invalid_chars() {
        let result = OrganizationSlug::new("test_slug".to_string());
        assert!(matches!(result, Err(OrganizationError::InvalidSlug(_))));

        let result = OrganizationSlug::new("test.slug".to_string());
        assert!(matches!(result, Err(OrganizationError::InvalidSlug(_))));
    }

    #[test]
    fn test_organization_slug_cannot_start_with_hyphen() {
        let result = OrganizationSlug::new("-test-slug".to_string());
        assert!(matches!(result, Err(OrganizationError::InvalidSlug(_))));
    }

    #[test]
    fn test_organization_slug_cannot_end_with_hyphen() {
        let result = OrganizationSlug::new("test-slug-".to_string());
        assert!(matches!(result, Err(OrganizationError::InvalidSlug(_))));
    }

    #[test]
    fn test_organization_slug_too_long() {
        let long_slug = "a".repeat(150);
        let result = OrganizationSlug::new(long_slug);
        assert!(matches!(result, Err(OrganizationError::InvalidSlug(_))));
    }
}
