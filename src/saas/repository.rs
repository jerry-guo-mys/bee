//! SaaS 仓储边界
//!
//! 当前只定义 trait，后续由 sqlite、文件迁移层或远程服务实现。

use async_trait::async_trait;

use crate::saas::models::{
    AgentInstance, AgentTemplate, CollaborationGroup, Conversation, ConversationMessage,
    Membership, Organization, TaskRecord, Team, Tenant, Workspace,
};

#[async_trait]
pub trait OrgRepository: Send + Sync {
    async fn create_tenant(&self, tenant: Tenant) -> Result<Tenant, String>;
    async fn get_tenant(&self, tenant_id: &str) -> Result<Option<Tenant>, String>;
    async fn create_organization(&self, organization: Organization)
        -> Result<Organization, String>;
    async fn list_organizations(&self, tenant_id: &str) -> Result<Vec<Organization>, String>;
    async fn create_team(&self, team: Team) -> Result<Team, String>;
    async fn list_teams(&self, organization_id: &str) -> Result<Vec<Team>, String>;
    async fn create_membership(&self, membership: Membership) -> Result<Membership, String>;
    async fn list_memberships(&self, organization_id: &str) -> Result<Vec<Membership>, String>;
}

#[async_trait]
pub trait AgentRepository: Send + Sync {
    async fn create_template(&self, template: AgentTemplate) -> Result<AgentTemplate, String>;
    async fn list_templates(&self, tenant_id: &str) -> Result<Vec<AgentTemplate>, String>;
    async fn create_instance(&self, instance: AgentInstance) -> Result<AgentInstance, String>;
    async fn list_instances(
        &self,
        organization_id: &str,
        team_id: Option<&str>,
    ) -> Result<Vec<AgentInstance>, String>;
    async fn create_workspace(&self, workspace: Workspace) -> Result<Workspace, String>;
    async fn list_workspaces(&self, organization_id: &str) -> Result<Vec<Workspace>, String>;
}

#[async_trait]
pub trait ConversationRepository: Send + Sync {
    async fn create_group(&self, group: CollaborationGroup) -> Result<CollaborationGroup, String>;
    async fn list_groups(&self, organization_id: &str) -> Result<Vec<CollaborationGroup>, String>;
    async fn create_conversation(&self, conversation: Conversation)
        -> Result<Conversation, String>;
    async fn get_conversation(&self, conversation_id: &str)
        -> Result<Option<Conversation>, String>;
    async fn list_conversations(
        &self,
        organization_id: &str,
        user_id: Option<&str>,
    ) -> Result<Vec<Conversation>, String>;
    async fn append_message(
        &self,
        message: ConversationMessage,
    ) -> Result<ConversationMessage, String>;
    async fn list_messages(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ConversationMessage>, String>;
}

#[async_trait]
pub trait TaskRepository: Send + Sync {
    async fn create_task(&self, task: TaskRecord) -> Result<TaskRecord, String>;
    async fn get_task(&self, task_id: &str) -> Result<Option<TaskRecord>, String>;
    async fn list_tasks(
        &self,
        organization_id: &str,
        team_id: Option<&str>,
    ) -> Result<Vec<TaskRecord>, String>;
    async fn update_task(&self, task: TaskRecord) -> Result<TaskRecord, String>;
}
