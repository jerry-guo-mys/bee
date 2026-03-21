//! SaaS 领域层
//!
//! 用于承载多租户产品化后的主数据模型与仓储边界。
//! 当前阶段先定义领域对象和 repository trait，后续再接 sqlite / API / 服务实现。

pub mod audit_service;
pub mod auth_service;
pub mod bootstrap;
pub mod bootstrap_service;
pub mod migration;
pub mod models;
pub mod repository;
pub mod sqlite;
pub mod sqlite_seed_repository;
pub mod sqlite_template_repository;
pub mod template_catalog;
pub mod template_instantiation_service;
pub mod tool_policy_service;

pub use audit_service::{
    append_audit_log, detail_json as audit_detail_json, list_audit_logs, AuditActor, AuditLogInput,
};
pub use auth_service::{ensure_access, AccessContext, AccessRequirement};
pub use bootstrap::{bootstrap_workspace_saas, SaasBootstrapResult};
pub use bootstrap_service::{
    build_bootstrap_plan, persist_bootstrap_plan, templates_for_industry, AgentTemplateSeed,
    IndustryTemplate, OrganizationBootstrapPlan, OrganizationBootstrapRequest,
    OrganizationBootstrapResult, TeamTemplate,
};
pub use migration::{LegacyWorkspaceImporter, MigrationReport};
pub use models::{
    AgentInstance, AgentTemplate, AuditLogRecord, CollaborationGroup, Conversation,
    ConversationMessage, Membership, Organization, TaskRecord, Team, Tenant, ToolAccessPolicy,
    UserAccount, Workspace,
};
pub use repository::{
    AgentRepository, AuditRepository, ConversationRepository, OrgRepository, TaskRepository,
};
pub use sqlite::{init_saas_sqlite, SaasSqliteStore};
pub use sqlite_seed_repository::SaasSeedRepository;
pub use sqlite_template_repository::SaasTemplateRepository;
pub use template_catalog::{load_platform_agent_templates, TemplateAssistantEntry};
pub use template_instantiation_service::{
    instantiate_team_templates, list_team_templates, TeamTemplateInstantiationRequest,
    TeamTemplateInstantiationResult,
};
pub use tool_policy_service::{
    default_low_risk_tools, list_tool_policies, resolve_effective_tool_allowlist,
    upsert_tool_policy, ToolPolicyInput, ToolPolicyScope,
};
