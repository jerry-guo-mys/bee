//! SaaS 领域层
//!
//! 用于承载多租户产品化后的主数据模型与仓储边界。
//! 当前阶段先定义领域对象和 repository trait，后续再接 sqlite / API / 服务实现。

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

pub use bootstrap::{bootstrap_workspace_saas, SaasBootstrapResult};
pub use bootstrap_service::{
    build_bootstrap_plan, persist_bootstrap_plan, templates_for_industry, AgentTemplateSeed,
    IndustryTemplate, OrganizationBootstrapPlan, OrganizationBootstrapRequest,
    OrganizationBootstrapResult, TeamTemplate,
};
pub use migration::{LegacyWorkspaceImporter, MigrationReport};
pub use models::{
    AgentInstance, AgentTemplate, CollaborationGroup, Conversation, ConversationMessage,
    Membership, Organization, TaskRecord, Team, Tenant, UserAccount, Workspace,
};
pub use repository::{AgentRepository, ConversationRepository, OrgRepository, TaskRepository};
pub use sqlite::{init_saas_sqlite, SaasSqliteStore};
pub use sqlite_seed_repository::SaasSeedRepository;
pub use sqlite_template_repository::SaasTemplateRepository;
pub use template_catalog::{load_platform_agent_templates, TemplateAssistantEntry};
pub use template_instantiation_service::{
    instantiate_team_templates, list_team_templates, TeamTemplateInstantiationRequest,
    TeamTemplateInstantiationResult,
};
