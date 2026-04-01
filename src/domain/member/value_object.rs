//! 成员聚合值对象定义
//!
//! 包含成员聚合使用的所有值对象：
//! - MembershipId: 成员 ID
//! - UserEmail: 用户邮箱（带验证）
//! - ToolId: 工具 ID
//! - ToolRiskLevel: 工具风险等级
//! - ToolPolicy: 工具策略

use thiserror::Error;

/// 值对象相关错误类型
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ValueObjectError {
    #[error("无效的邮箱地址：{0}")]
    InvalidEmail(String),

    #[error("无效的工具 ID: {0}")]
    InvalidToolId(String),

    #[error("无效的风险等级：{0}")]
    InvalidRiskLevel(String),
}

// ============================================================================
// MembershipId (已从 tenant/value_object.rs 导入，此处仅重新导出)
// ============================================================================

// ============================================================================
// UserEmail - 邮箱值对象
// ============================================================================

/// 用户邮箱 - 带邮箱格式验证
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserEmail(String);

impl UserEmail {
    /// 简单的邮箱格式验证
    fn validate(email: &str) -> Result<(), ValueObjectError> {
        if email.is_empty() {
            return Err(ValueObjectError::InvalidEmail("邮箱不能为空".to_string()));
        }

        // 基本格式验证：包含 @ 和 .
        if !email.contains('@') {
            return Err(ValueObjectError::InvalidEmail("邮箱必须包含 @".to_string()));
        }

        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return Err(ValueObjectError::InvalidEmail("邮箱格式不正确".to_string()));
        }

        let (local, domain) = (parts[0], parts[1]);

        if local.is_empty() {
            return Err(ValueObjectError::InvalidEmail(
                "邮箱用户名不能为空".to_string(),
            ));
        }

        if domain.is_empty() {
            return Err(ValueObjectError::InvalidEmail(
                "邮箱域名不能为空".to_string(),
            ));
        }

        if !domain.contains('.') {
            return Err(ValueObjectError::InvalidEmail(
                "邮箱域名必须包含顶级域名".to_string(),
            ));
        }

        // 验证域名部分不以 . 开头或结尾
        if domain.starts_with('.') || domain.ends_with('.') {
            return Err(ValueObjectError::InvalidEmail(
                "域名不能以 . 开头或结尾".to_string(),
            ));
        }

        Ok(())
    }

    /// 创建新的 UserEmail，会进行验证
    pub fn new(email: String) -> Result<Self, ValueObjectError> {
        let email = email.to_lowercase().trim().to_string();
        Self::validate(&email)?;
        Ok(Self(email))
    }

    /// 从字符串创建，不验证（用于从数据库加载）
    pub fn from_str_unchecked(email: &str) -> Self {
        Self(email.to_string())
    }

    /// 获取底层字符串引用
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for UserEmail {
    type Error = ValueObjectError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

impl TryFrom<&str> for UserEmail {
    type Error = ValueObjectError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s.to_string())
    }
}

impl std::fmt::Display for UserEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// ToolId - 工具 ID 值对象
// ============================================================================

/// 工具 ID - 封装工具标识符
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolId(String);

impl ToolId {
    /// 创建新的 ToolId
    pub fn new(id: String) -> Result<Self, ValueObjectError> {
        if id.is_empty() {
            return Err(ValueObjectError::InvalidToolId(
                "工具 ID 不能为空".to_string(),
            ));
        }
        Ok(Self(id))
    }

    /// 从字符串创建，不验证
    pub fn from_str(id: &str) -> Self {
        Self(id.to_string())
    }

    /// 获取底层字符串引用
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ToolId {
    fn from(s: String) -> Self {
        Self::from_str(&s)
    }
}

impl From<&str> for ToolId {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl std::fmt::Display for ToolId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// ToolRiskLevel - 工具风险等级
// ============================================================================

/// 工具风险等级 - 用于控制工具执行权限
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ToolRiskLevel {
    /// 低风险 - 只读操作，如文件读取、搜索
    Low = 0,
    /// 中风险 - 本地写操作，如文件编辑、创建
    Medium = 1,
    /// 高风险 - 系统操作，如 shell 执行、git 操作
    High = 2,
    /// 严重风险 - 危险操作，如 rm -rf、数据库删除
    Critical = 3,
}

impl ToolRiskLevel {
    /// 从字符串创建 RiskLevel
    pub fn from_str(level: &str) -> Result<Self, ValueObjectError> {
        match level.to_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            _ => Err(ValueObjectError::InvalidRiskLevel(format!(
                "无效的风险等级：{}",
                level
            ))),
        }
    }

    /// 获取风险等级的字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    /// 检查是否小于等于指定风险等级
    pub fn is_at_most(&self, other: Self) -> bool {
        self <= &other
    }
}

impl Default for ToolRiskLevel {
    fn default() -> Self {
        Self::Low
    }
}

// ============================================================================
// ToolPolicy - 工具策略
// ============================================================================

/// 工具策略 - 定义成员对工具的执行权限
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPolicy {
    /// 工具 ID
    tool_id: ToolId,
    /// 风险等级
    risk_level: ToolRiskLevel,
    /// 是否允许执行
    allowed: bool,
    /// 备注说明
    note: Option<String>,
}

impl ToolPolicy {
    /// 创建新的工具策略
    pub fn new(tool_id: ToolId, risk_level: ToolRiskLevel, allowed: bool) -> Self {
        Self {
            tool_id,
            risk_level,
            allowed,
            note: None,
        }
    }

    /// 创建带备注的工具策略
    pub fn with_note(mut self, note: String) -> Self {
        self.note = Some(note);
        self
    }

    /// 检查是否可以执行指定风险等级的工具
    pub fn can_execute(&self, required_level: ToolRiskLevel) -> bool {
        if !self.allowed {
            return false;
        }
        // 允许执行的条件：策略风险等级 >= 工具所需风险等级
        self.risk_level >= required_level
    }

    /// 获取工具 ID
    pub fn tool_id(&self) -> &ToolId {
        &self.tool_id
    }

    /// 获取风险等级
    pub fn risk_level(&self) -> ToolRiskLevel {
        self.risk_level
    }

    /// 是否允许
    pub fn is_allowed(&self) -> bool {
        self.allowed
    }

    /// 获取备注
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_email_valid() {
        let email = UserEmail::new("test@example.com".to_string()).unwrap();
        assert_eq!(email.as_str(), "test@example.com");
    }

    #[test]
    fn test_user_email_lowercase() {
        let email = UserEmail::new("TEST@EXAMPLE.COM".to_string()).unwrap();
        assert_eq!(email.as_str(), "test@example.com");
    }

    #[test]
    fn test_user_email_empty() {
        let result = UserEmail::new("".to_string());
        assert!(matches!(result, Err(ValueObjectError::InvalidEmail(_))));
    }

    #[test]
    fn test_user_email_no_at() {
        let result = UserEmail::new("testexample.com".to_string());
        assert!(matches!(result, Err(ValueObjectError::InvalidEmail(_))));
    }

    #[test]
    fn test_user_email_no_domain() {
        let result = UserEmail::new("test@".to_string());
        assert!(matches!(result, Err(ValueObjectError::InvalidEmail(_))));
    }

    #[test]
    fn test_user_email_no_tld() {
        let result = UserEmail::new("test@example".to_string());
        assert!(matches!(result, Err(ValueObjectError::InvalidEmail(_))));
    }

    #[test]
    fn test_user_email_display() {
        let email = UserEmail::new("test@example.com".to_string()).unwrap();
        assert_eq!(format!("{}", email), "test@example.com");
    }

    #[test]
    fn test_tool_id_creation() {
        let tool_id = ToolId::new("shell".to_string()).unwrap();
        assert_eq!(tool_id.as_str(), "shell");
    }

    #[test]
    fn test_tool_id_empty() {
        let result = ToolId::new("".to_string());
        assert!(matches!(result, Err(ValueObjectError::InvalidToolId(_))));
    }

    #[test]
    fn test_tool_id_from_str() {
        let tool_id = ToolId::from_str("file_read");
        assert_eq!(tool_id.as_str(), "file_read");
    }

    #[test]
    fn test_tool_risk_level_from_str() {
        assert_eq!(ToolRiskLevel::from_str("low").unwrap(), ToolRiskLevel::Low);
        assert_eq!(
            ToolRiskLevel::from_str("medium").unwrap(),
            ToolRiskLevel::Medium
        );
        assert_eq!(
            ToolRiskLevel::from_str("high").unwrap(),
            ToolRiskLevel::High
        );
        assert_eq!(
            ToolRiskLevel::from_str("critical").unwrap(),
            ToolRiskLevel::Critical
        );
        assert!(ToolRiskLevel::from_str("invalid").is_err());
    }

    #[test]
    fn test_tool_risk_level_as_str() {
        assert_eq!(ToolRiskLevel::Low.as_str(), "low");
        assert_eq!(ToolRiskLevel::Medium.as_str(), "medium");
        assert_eq!(ToolRiskLevel::High.as_str(), "high");
        assert_eq!(ToolRiskLevel::Critical.as_str(), "critical");
    }

    #[test]
    fn test_tool_risk_level_ordering() {
        assert!(ToolRiskLevel::Low < ToolRiskLevel::Medium);
        assert!(ToolRiskLevel::Medium < ToolRiskLevel::High);
        assert!(ToolRiskLevel::High < ToolRiskLevel::Critical);
    }

    #[test]
    fn test_tool_risk_level_is_at_most() {
        assert!(ToolRiskLevel::Low.is_at_most(ToolRiskLevel::Low));
        assert!(ToolRiskLevel::Low.is_at_most(ToolRiskLevel::Medium));
        assert!(!ToolRiskLevel::High.is_at_most(ToolRiskLevel::Medium));
    }

    #[test]
    fn test_tool_policy_can_execute() {
        let policy = ToolPolicy::new(ToolId::from_str("shell"), ToolRiskLevel::Medium, true);

        // 可以执行低风险工具
        assert!(policy.can_execute(ToolRiskLevel::Low));
        // 可以执行同等风险工具
        assert!(policy.can_execute(ToolRiskLevel::Medium));
        // 不能执行高风险工具
        assert!(!policy.can_execute(ToolRiskLevel::High));
    }

    #[test]
    fn test_tool_policy_not_allowed() {
        let policy = ToolPolicy::new(
            ToolId::from_str("dangerous_tool"),
            ToolRiskLevel::Critical,
            false,
        );

        // 即使风险等级足够，但不允许执行
        assert!(!policy.can_execute(ToolRiskLevel::Low));
    }

    #[test]
    fn test_tool_policy_with_note() {
        let policy = ToolPolicy::new(ToolId::from_str("shell"), ToolRiskLevel::High, true)
            .with_note("仅管理员可用".to_string());

        assert_eq!(policy.note(), Some("仅管理员可用"));
    }
}
