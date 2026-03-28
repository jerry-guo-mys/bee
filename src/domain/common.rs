//! 领域通用类型定义
//!
//! 包含整个 SaaS 多租户系统共享的基础类型、枚举和工具函数。

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// 生成 UUID v4
pub fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

/// 获取当前 UTC 时间
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

/// 成员角色 - 定义用户在租户/组织/团队中的权限级别
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MembershipRole {
    /// 平台管理员 - 拥有所有权限
    PlatformAdmin,
    /// 组织管理员 - 拥有组织内所有权限
    OrgAdmin,
    /// 团队管理员 - 拥有团队内所有权限
    TeamAdmin,
    /// 普通成员 - 基础执行权限
    Member,
    /// 观察者 - 只读权限
    Viewer,
}

impl MembershipRole {
    /// 检查是否有指定权限
    pub fn has_permission(&self, permission: &Permission) -> bool {
        match (self, permission) {
            // PlatformAdmin 拥有所有权限
            (MembershipRole::PlatformAdmin, _) => true,

            // OrgAdmin 拥有组织级读写权限
            (MembershipRole::OrgAdmin, Permission::OrgRead) => true,
            (MembershipRole::OrgAdmin, Permission::OrgWrite) => true,
            (MembershipRole::OrgAdmin, Permission::TenantRead) => true,
            (MembershipRole::OrgAdmin, Permission::TeamRead) => true,

            // TeamAdmin 拥有团队级读写权限
            (MembershipRole::TeamAdmin, Permission::TeamRead) => true,
            (MembershipRole::TeamAdmin, Permission::TeamWrite) => true,
            (MembershipRole::TeamAdmin, Permission::OrgRead) => true,
            (MembershipRole::TeamAdmin, Permission::AgentRead) => true,
            (MembershipRole::TeamAdmin, Permission::AgentExecute) => true,

            // Member 拥有基础执行权限
            (MembershipRole::Member, Permission::AgentRead) => true,
            (MembershipRole::Member, Permission::AgentExecute) => true,
            (MembershipRole::Member, Permission::TeamRead) => true,
            (MembershipRole::Member, Permission::OrgRead) => true,
            (MembershipRole::Member, Permission::TenantRead) => true,

            // Viewer 只有只读权限
            (MembershipRole::Viewer, Permission::TenantRead) => true,
            (MembershipRole::Viewer, Permission::OrgRead) => true,
            (MembershipRole::Viewer, Permission::TeamRead) => true,
            (MembershipRole::Viewer, Permission::AgentRead) => true,

            _ => false,
        }
    }
}

/// 权限定义 - 细粒度权限控制
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    // 租户级权限
    TenantRead,
    TenantWrite,
    TenantDelete,

    // 组织级权限
    OrgRead,
    OrgWrite,
    OrgDelete,

    // 团队级权限
    TeamRead,
    TeamWrite,
    TeamDelete,

    // Agent 级权限
    AgentRead,
    AgentExecute,
    AgentModify,
    AgentDelete,

    // 工具级权限 - 可指定具体工具
    ToolExecute(String),
}

/// 成员状态
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MembershipStatus {
    #[default]
    /// 待处理 - 已邀请但未接受
    Pending,
    /// 活跃 - 正常成员
    Active,
    /// 暂停 - 暂时禁用
    Suspended,
    /// 已移除 - 已删除或退出
    Removed,
}

/// 租户状态
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TenantStatus {
    #[default]
    /// 活跃 - 正常使用中
    Active,
    /// 暂停 - 被管理员暂停
    Suspended,
    /// 已归档 - 已删除但保留数据
    Archived,
}

/// Agent 实例状态
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AgentInstanceStatus {
    #[default]
    /// 活跃 - 可正常使用
    Active,
    /// 禁用 - 被管理员禁用
    Disabled,
    /// 已归档 - 已删除但保留历史
    Archived,
}

/// 任务状态
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TaskStatus {
    #[default]
    /// 待处理
    Pending,
    /// 执行中
    InProgress,
    /// 已完成
    Done,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 会话状态
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SessionStatus {
    #[default]
    /// 空闲
    Idle,
    /// 处理中
    Processing,
    /// 等待中 (等待用户输入或外部事件)
    Waiting,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36); // UUID 标准长度
    }

    #[test]
    fn test_now() {
        let now1 = now();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let now2 = now();
        assert!(now2 >= now1);
    }

    #[test]
    fn test_membership_role_permissions() {
        // PlatformAdmin 拥有所有权限
        assert!(MembershipRole::PlatformAdmin.has_permission(&Permission::TenantRead));
        assert!(MembershipRole::PlatformAdmin.has_permission(&Permission::TenantWrite));
        assert!(MembershipRole::PlatformAdmin.has_permission(&Permission::TenantDelete));
        assert!(MembershipRole::PlatformAdmin.has_permission(&Permission::ToolExecute("shell".to_string())));

        // OrgAdmin 有组织级权限
        assert!(MembershipRole::OrgAdmin.has_permission(&Permission::OrgRead));
        assert!(MembershipRole::OrgAdmin.has_permission(&Permission::OrgWrite));
        assert!(!MembershipRole::OrgAdmin.has_permission(&Permission::OrgDelete));

        // Viewer 只有只读权限
        assert!(MembershipRole::Viewer.has_permission(&Permission::TenantRead));
        assert!(MembershipRole::Viewer.has_permission(&Permission::OrgRead));
        assert!(!MembershipRole::Viewer.has_permission(&Permission::TenantWrite));
        assert!(!MembershipRole::Viewer.has_permission(&Permission::AgentExecute));
    }

    #[test]
    fn test_membership_status_default() {
        assert_eq!(MembershipStatus::default(), MembershipStatus::Pending);
    }

    #[test]
    fn test_tenant_status_default() {
        assert_eq!(TenantStatus::default(), TenantStatus::Active);
    }

    #[test]
    fn test_agent_instance_status_default() {
        assert_eq!(AgentInstanceStatus::default(), AgentInstanceStatus::Active);
    }

    #[test]
    fn test_task_status_default() {
        assert_eq!(TaskStatus::default(), TaskStatus::Pending);
    }

    #[test]
    fn test_session_status_default() {
        assert_eq!(SessionStatus::default(), SessionStatus::Idle);
    }
}
