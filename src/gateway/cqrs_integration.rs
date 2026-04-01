//! Gateway CQRS 集成
//!
//! 将命令总线和查询总线集成到 Gateway，支持多租户管理操作

use std::sync::Arc;

use tokio::sync::mpsc;

use super::message::{ClientInfo, GatewayMessage, MessageType, MemberDto};
use crate::application::commands::{
    CreateTenantCommand, CreateOrganizationCommand, CreateTeamCommand,
    InviteMemberCommand, AcceptInviteCommand, SuspendMemberCommand,
    handler::{InMemoryCommandBus, CommandBus},
};
use crate::application::queries::{
    GetTenantQuery, ListMembersQuery, GetOrganizationQuery,
    handler::{InMemoryQueryBus, QueryBus},
};
use crate::domain::common::MembershipRole;
use crate::domain::tenant::{TenantId, OrganizationId, TeamId, UserId, MembershipId};

/// Gateway CQRS 集成服务
pub struct GatewayCqrsService {
    command_bus: Arc<InMemoryCommandBus>,
    query_bus: Arc<InMemoryQueryBus>,
    message_tx: Option<mpsc::UnboundedSender<GatewayMessage>>,
    string_tx: Option<mpsc::UnboundedSender<String>>,
    client_info: ClientInfo,
}

impl GatewayCqrsService {
    /// 创建新的集成服务（使用 GatewayMessage sender）
    pub fn new(
        command_bus: Arc<InMemoryCommandBus>,
        query_bus: Arc<InMemoryQueryBus>,
        message_tx: mpsc::UnboundedSender<GatewayMessage>,
        client_info: ClientInfo,
    ) -> Self {
        Self {
            command_bus,
            query_bus,
            message_tx: Some(message_tx),
            string_tx: None,
            client_info,
        }
    }

    /// 创建新的集成服务（使用 String sender）
    pub fn new_for_string_tx(
        command_bus: Arc<InMemoryCommandBus>,
        query_bus: Arc<InMemoryQueryBus>,
        string_tx: mpsc::UnboundedSender<String>,
        client_info: ClientInfo,
    ) -> Self {
        Self {
            command_bus,
            query_bus,
            message_tx: None,
            string_tx: Some(string_tx),
            client_info,
        }
    }

    /// 发送消息（支持两种 sender 类型）
    fn send_message(&self, message: MessageType) -> Result<(), String> {
        let msg = GatewayMessage::new(Some(self.client_info.client_id.clone()), message);

        if let Some(ref tx) = self.message_tx {
            tx.send(msg)
                .map_err(|e| format!("Failed to send message: {}", e))
        } else if let Some(ref tx) = self.string_tx {
            tx.send(serde_json::to_string(&msg).unwrap_or_default())
                .map_err(|e| format!("Failed to send message: {}", e))
        } else {
            Err("No sender available".to_string())
        }
    }

    /// 处理创建租户命令
    pub async fn handle_create_tenant(&self, name: String, slug: String) -> Result<(), String> {
        let user_id = UserId::new(self.client_info.client_id.clone());

        let command = CreateTenantCommand {
            name,
            slug,
            creator_id: user_id,
        };

        match self.command_bus.dispatch(command).await {
            Ok(tenant) => {
                self.send_message(MessageType::OperationResult {
                    success: true,
                    message: format!("Tenant created: {}", tenant.name()),
                    data: Some(serde_json::json!({
                        "tenant_id": tenant.id().to_string(),
                        "name": tenant.name().as_str(),
                        "slug": tenant.slug().as_str(),
                        "status": tenant.status().to_string(),
                    })),
                })
            }
            Err(e) => self.send_message(MessageType::OperationResult {
                success: false,
                message: format!("Failed to create tenant: {}", e),
                data: None,
            }),
        }
    }

    /// 处理获取租户查询
    pub async fn handle_get_tenant(&self, tenant_id: String) -> Result<(), String> {
        let query = GetTenantQuery {
            tenant_id: TenantId::new(tenant_id),
        };

        match self.query_bus.ask(query).await {
            Ok(tenant_opt) => {
                if let Some(tenant) = tenant_opt {
                    self.send_message(MessageType::Tenant {
                        tenant_id: tenant.id().to_string(),
                        name: tenant.name().as_str().to_string(),
                        slug: tenant.slug().as_str().to_string(),
                        status: tenant.status().to_string(),
                    })
                } else {
                    self.send_message(MessageType::OperationResult {
                        success: false,
                        message: "Tenant not found".to_string(),
                        data: None,
                    })
                }
            }
            Err(e) => self.send_message(MessageType::OperationResult {
                success: false,
                message: format!("Query error: {}", e),
                data: None,
            }),
        }
    }

    /// 处理创建组织命令
    pub async fn handle_create_organization(
        &self,
        tenant_id: String,
        name: String,
        slug: String,
    ) -> Result<(), String> {
        let creator_id = UserId::new(self.client_info.client_id.clone());

        let command = CreateOrganizationCommand {
            tenant_id,
            name,
            slug,
            creator_id,
        };

        match self.command_bus.dispatch(command).await {
            Ok(org) => self.send_message(MessageType::OperationResult {
                success: true,
                message: format!("Organization created: {}", org.name()),
                data: Some(serde_json::json!({
                    "organization_id": org.id().to_string(),
                    "name": org.name().as_str(),
                    "slug": org.slug().as_str(),
                    "tenant_id": org.tenant_id().to_string(),
                })),
            }),
            Err(e) => self.send_message(MessageType::OperationResult {
                success: false,
                message: format!("Failed to create organization: {}", e),
                data: None,
            }),
        }
    }

    /// 处理获取组织查询
    pub async fn handle_get_organization(&self, organization_id: String) -> Result<(), String> {
        // TODO: 从认证上下文获取 tenant_id
        let tenant_id = self.client_info.client_id.clone();

        let query = GetOrganizationQuery {
            tenant_id,
            organization_id,
        };

        match self.query_bus.ask(query).await {
            Ok(Some(org)) => self.send_message(MessageType::Organization {
                organization_id: org.id().to_string(),
                name: org.name().as_str().to_string(),
                slug: org.slug().as_str().to_string(),
                tenant_id: org.tenant_id().to_string(),
            }),
            Ok(None) => self.send_message(MessageType::OperationResult {
                success: false,
                message: "Organization not found".to_string(),
                data: None,
            }),
            Err(e) => self.send_message(MessageType::OperationResult {
                success: false,
                message: format!("Query error: {}", e),
                data: None,
            }),
        }
    }

    /// 处理创建团队命令
    pub async fn handle_create_team(
        &self,
        tenant_id: String,
        organization_id: String,
        name: String,
        code: Option<String>,
    ) -> Result<(), String> {
        let creator_id = UserId::new(self.client_info.client_id.clone());

        let command = CreateTeamCommand {
            tenant_id,
            organization_id,
            name,
            code,
            parent_team_id: None,
            creator_id,
        };

        match self.command_bus.dispatch(command).await {
            Ok(team) => self.send_message(MessageType::OperationResult {
                success: true,
                message: format!("Team created: {}", team.name()),
                data: Some(serde_json::json!({
                    "team_id": team.id().to_string(),
                    "name": team.name().as_str(),
                    "code": team.code().map(|c| c.as_str()),
                })),
            }),
            Err(e) => self.send_message(MessageType::OperationResult {
                success: false,
                message: format!("Failed to create team: {}", e),
                data: None,
            }),
        }
    }

    /// 处理邀请成员命令
    pub async fn handle_invite_member(
        &self,
        tenant_id: String,
        organization_id: String,
        team_id: Option<String>,
        email: String,
        role: String,
    ) -> Result<(), String> {
        let role = self.parse_membership_role(&role).unwrap_or(MembershipRole::Member);
        let team_id = team_id.map(|id| TeamId::new(id));
        let user_id = UserId::new(self.client_info.client_id.clone());

        let command = InviteMemberCommand {
            tenant_id: TenantId::new(tenant_id),
            organization_id: OrganizationId::new(organization_id),
            team_id,
            user_id: user_id.clone(),
            email,
            role,
            inviter_id: user_id,
        };

        match self.command_bus.dispatch(command).await {
            Ok(membership) => self.send_message(MessageType::OperationResult {
                success: true,
                message: format!("Member invited: {}", membership.id()),
                data: Some(serde_json::json!({
                    "membership_id": membership.id().to_string(),
                    "email": membership.email().as_str(),
                    "role": Self::membership_role_to_string(&membership.role()),
                    "status": membership.status().to_string(),
                })),
            }),
            Err(e) => self.send_message(MessageType::OperationResult {
                success: false,
                message: format!("Failed to invite member: {}", e),
                data: None,
            }),
        }
    }

    /// 处理接受邀请命令
    pub async fn handle_accept_invite(&self, membership_id: String) -> Result<(), String> {
        let command = AcceptInviteCommand {
            membership_id: MembershipId::new(membership_id.clone()),
            user_id: UserId::new(self.client_info.client_id.clone()),
        };

        match self.command_bus.dispatch(command).await {
            Ok(_) => self.send_message(MessageType::OperationResult {
                success: true,
                message: "Invite accepted successfully".to_string(),
                data: Some(serde_json::json!({
                    "membership_id": membership_id,
                    "success": true,
                })),
            }),
            Err(e) => self.send_message(MessageType::OperationResult {
                success: false,
                message: format!("Failed to accept invite: {}", e),
                data: None,
            }),
        }
    }

    /// 处理暂停成员命令
    pub async fn handle_suspend_member(
        &self,
        membership_id: String,
        reason: String,
    ) -> Result<(), String> {
        let command = SuspendMemberCommand {
            membership_id: MembershipId::new(membership_id.clone()),
            reason,
            operator_id: UserId::new(self.client_info.client_id.clone()),
        };

        match self.command_bus.dispatch(command).await {
            Ok(_) => self.send_message(MessageType::OperationResult {
                success: true,
                message: "Member suspended successfully".to_string(),
                data: Some(serde_json::json!({
                    "membership_id": membership_id,
                    "success": true,
                })),
            }),
            Err(e) => self.send_message(MessageType::OperationResult {
                success: false,
                message: format!("Failed to suspend member: {}", e),
                data: None,
            }),
        }
    }

    /// 处理列出成员查询
    pub async fn handle_list_members(
        &self,
        _tenant_id: String,
        organization_id: String,
        team_id: Option<String>,
    ) -> Result<(), String> {
        let team_id = team_id.map(TeamId::new);
        let query = ListMembersQuery {
            tenant_id: TenantId::generate(), // TODO: 从认证上下文获取
            organization_id: OrganizationId::new(organization_id),
            team_id,
            status: None,
            limit: 50,
            offset: 0,
        };

        match self.query_bus.ask(query).await {
            Ok(memberships) => {
                let members: Vec<MemberDto> = memberships
                    .iter()
                    .map(|m| MemberDto {
                        id: m.id().to_string(),
                        user_id: m.user_id().map(|u| u.to_string()).unwrap_or_default(),
                        display_name: None, // TODO: 从用户聚合获取
                        email: Some(m.email().as_str().to_string()),
                        role: Self::membership_role_to_string(&m.role()),
                        status: m.status().to_string(),
                        team_name: m.team_id().map(|t| t.to_string()),
                        joined_at: m.created_at().to_rfc3339(),
                    })
                    .collect();

                self.send_message(MessageType::MembersList { members })
            }
            Err(e) => self.send_message(MessageType::OperationResult {
                success: false,
                message: format!("Failed to list members: {}", e),
                data: None,
            }),
        }
    }

    /// 解析 MembershipRole 字符串
    fn parse_membership_role(&self, s: &str) -> Option<MembershipRole> {
        match s.to_lowercase().as_str() {
            "platformadmin" => Some(MembershipRole::PlatformAdmin),
            "orgadmin" => Some(MembershipRole::OrgAdmin),
            "teamadmin" => Some(MembershipRole::TeamAdmin),
            "member" => Some(MembershipRole::Member),
            "viewer" => Some(MembershipRole::Viewer),
            _ => None,
        }
    }

    /// 将 MembershipRole 转换为字符串
    fn membership_role_to_string(role: &MembershipRole) -> String {
        match role {
            MembershipRole::PlatformAdmin => "PlatformAdmin".to_string(),
            MembershipRole::OrgAdmin => "OrgAdmin".to_string(),
            MembershipRole::TeamAdmin => "TeamAdmin".to_string(),
            MembershipRole::Member => "Member".to_string(),
            MembershipRole::Viewer => "Viewer".to_string(),
        }
    }
}
