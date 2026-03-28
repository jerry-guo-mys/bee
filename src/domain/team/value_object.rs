//! Team 值对象定义
//!
//! 值对象是不可变的，通过其属性值来定义相等性。
//! 本模块包含团队聚合根使用的所有值对象。

use crate::domain::common::generate_id;
use thiserror::Error;

/// 团队相关错误类型
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TeamError {
    #[error("无效的团队名称：{0}")]
    InvalidName(String),

    #[error("无效的团队代码：{0}")]
    InvalidCode(String),

    #[error("团队不存在：{0}")]
    NotFound(String),

    #[error("团队已存在：{0}")]
    AlreadyExists(String),

    #[error("数据库错误：{0}")]
    DatabaseError(String),
}

/// 团队 ID - 封装 UUID 字符串
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TeamId(String);

impl TeamId {
    /// 生成新的 TeamId
    pub fn generate() -> Self {
        Self(generate_id())
    }

    /// 从字符串创建 TeamId
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// 从字符串切片创建 TeamId
    pub fn from_str(id: &str) -> Self {
        Self(id.to_string())
    }

    /// 获取底层字符串引用
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

/// 团队名称 - 验证长度在 1-255 之间
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TeamName(String);

impl TeamName {
    const MIN_LEN: usize = 1;
    const MAX_LEN: usize = 255;

    /// 创建新的 TeamName，会进行验证
    pub fn new(name: String) -> Result<Self, TeamError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(TeamError::InvalidName("名称不能为空".to_string()));
        }
        if trimmed.len() < Self::MIN_LEN {
            return Err(TeamError::InvalidName(format!(
                "名称长度不能少于 {} 个字符",
                Self::MIN_LEN
            )));
        }
        if trimmed.len() > Self::MAX_LEN {
            return Err(TeamError::InvalidName(format!(
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

impl TryFrom<String> for TeamName {
    type Error = TeamError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl std::fmt::Display for TeamName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 团队代码 - 可选的唯一标识符，用于内部引用
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TeamCode(String);

impl TeamCode {
    const MIN_LEN: usize = 1;
    const MAX_LEN: usize = 50;

    /// 创建新的 TeamCode，会进行验证
    pub fn new(code: String) -> Result<Self, TeamError> {
        let trimmed = code.trim();
        if trimmed.is_empty() {
            return Err(TeamError::InvalidCode("代码不能为空".to_string()));
        }
        if trimmed.len() < Self::MIN_LEN {
            return Err(TeamError::InvalidCode(format!(
                "代码长度不能少于 {} 个字符",
                Self::MIN_LEN
            )));
        }
        if trimmed.len() > Self::MAX_LEN {
            return Err(TeamError::InvalidCode(format!(
                "代码长度不能超过 {} 个字符",
                Self::MAX_LEN
            )));
        }

        // 验证只包含字母、数字、下划线和连字符
        if !trimmed
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(TeamError::InvalidCode(
                "代码只能包含字母、数字、下划线和连字符".to_string(),
            ));
        }

        Ok(Self(trimmed.to_string()))
    }

    /// 从字符串创建，不验证（用于从数据库加载）
    pub fn from_str_unchecked(code: &str) -> Self {
        Self(code.to_string())
    }

    /// 获取底层字符串引用
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for TeamCode {
    type Error = TeamError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl std::fmt::Display for TeamCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_id_generation() {
        let id1 = TeamId::generate();
        let id2 = TeamId::generate();
        assert_ne!(id1, id2);
        assert_eq!(id1.as_str().len(), 36);
    }

    #[test]
    fn test_team_id_from_str() {
        let id = TeamId::from_str("test-team-id");
        assert_eq!(id.as_str(), "test-team-id");
    }

    #[test]
    fn test_team_name_valid() {
        let name = TeamName::new("Test Team".to_string()).unwrap();
        assert_eq!(name.as_str(), "Test Team");
    }

    #[test]
    fn test_team_name_empty() {
        let result = TeamName::new("".to_string());
        assert!(matches!(result, Err(TeamError::InvalidName(_))));
    }

    #[test]
    fn test_team_name_whitespace_only() {
        let result = TeamName::new("   ".to_string());
        assert!(matches!(result, Err(TeamError::InvalidName(_))));
    }

    #[test]
    fn test_team_name_too_long() {
        let long_name = "a".repeat(300);
        let result = TeamName::new(long_name);
        assert!(matches!(result, Err(TeamError::InvalidName(_))));
    }

    #[test]
    fn test_team_name_trimmed() {
        let name = TeamName::new("  Test Team  ".to_string()).unwrap();
        assert_eq!(name.as_str(), "Test Team");
    }

    #[test]
    fn test_team_code_valid() {
        let code = TeamCode::new("DEV-TEAM-001".to_string()).unwrap();
        assert_eq!(code.as_str(), "DEV-TEAM-001");
    }

    #[test]
    fn test_team_code_with_underscore() {
        let code = TeamCode::new("dev_team_001".to_string()).unwrap();
        assert_eq!(code.as_str(), "dev_team_001");
    }

    #[test]
    fn test_team_code_empty() {
        let result = TeamCode::new("".to_string());
        assert!(matches!(result, Err(TeamError::InvalidCode(_))));
    }

    #[test]
    fn test_team_code_invalid_chars() {
        let result = TeamCode::new("team.code".to_string());
        assert!(matches!(result, Err(TeamError::InvalidCode(_))));

        let result = TeamCode::new("team code".to_string());
        assert!(matches!(result, Err(TeamError::InvalidCode(_))));
    }

    #[test]
    fn test_team_code_too_long() {
        let long_code = "a".repeat(100);
        let result = TeamCode::new(long_code);
        assert!(matches!(result, Err(TeamError::InvalidCode(_))));
    }
}
