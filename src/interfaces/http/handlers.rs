//! HTTP 请求处理器
//!
//! 处理 HTTP 请求，调用 CQRS 命令/查询总线

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::commands::handler::InMemoryCommandBus;
use crate::application::queries::handler::InMemoryQueryBus;

use crate::application::{CommandBus, QueryBus};
use crate::application::commands::{
    CreateTenantCommand, CreateOrganizationCommand, CreateTeamCommand,
    InviteMemberCommand, AcceptInviteCommand, SuspendMemberCommand,
};
use crate::application::queries::{
    GetTenantQuery, ListMembersQuery, GetOrganizationQuery,
};
use crate::domain::common::MembershipRole;
use crate::domain::tenant::{TenantId, OrganizationId, TeamId, UserId, MembershipId};

/// 应用状态（共享依赖）
#[derive(Clone)]
pub struct AppState {
    pub command_bus: Arc<InMemoryCommandBus>,
    pub query_bus: Arc<InMemoryQueryBus>,
}

// ========== 请求/响应 DTO ==========

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize)]
pub struct TenantResponse {
    pub tenant_id: String,
    pub name: String,
    pub slug: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize)]
pub struct OrganizationResponse {
    pub organization_id: String,
    pub name: String,
    pub slug: String,
    pub tenant_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub code: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TeamResponse {
    pub team_id: String,
    pub name: String,
    pub code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InviteMemberRequest {
    pub email: String,
    pub role: String,
    pub team_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListMembersQueryParams {
    pub team_id: Option<String>,
    pub status: Option<String>,
    pub page: Option<i32>,
    pub page_size: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct MemberResponse {
    pub id: String,
    pub user_id: String,
    pub display_name: Option<String>,
    pub email: String,
    pub role: String,
    pub status: String,
    pub team_name: Option<String>,
    pub joined_at: String,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

// ========== 处理器实现 ==========

/// POST /tenants - 创建租户
pub async fn create_tenant(
    State(state): State<AppState>,
    Json(req): Json<CreateTenantRequest>,
) -> Result<Json<TenantResponse>, (StatusCode, Json<ApiError>)> {
    // TODO: 从认证上下文获取 user_id
    let user_id = UserId::new("system".to_string());

    let command = CreateTenantCommand {
        name: req.name.clone(),
        slug: req.slug.clone(),
        creator_id: user_id,
    };

    match state.command_bus.dispatch(command).await {
        Ok(tenant) => Ok(Json(TenantResponse {
            tenant_id: tenant.id().to_string(),
            name: tenant.name().as_str().to_string(),
            slug: tenant.slug().as_str().to_string(),
            status: tenant.status().to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "CREATE_TENANT_FAILED".to_string(),
                message: e.to_string(),
            }),
        )),
    }
}

/// GET /tenants/{tenant_id} - 获取租户
pub async fn get_tenant(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
) -> Result<Json<TenantResponse>, (StatusCode, Json<ApiError>)> {
    let query = GetTenantQuery {
        tenant_id: TenantId::new(tenant_id),
    };

    match state.query_bus.ask(query).await {
        Ok(Some(tenant)) => Ok(Json(TenantResponse {
            tenant_id: tenant.id().to_string(),
            name: tenant.name().as_str().to_string(),
            slug: tenant.slug().as_str().to_string(),
            status: tenant.status().to_string(),
        })),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiError {
                code: "TENANT_NOT_FOUND".to_string(),
                message: "Tenant not found".to_string(),
            }),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "GET_TENANT_FAILED".to_string(),
                message: e.to_string(),
            }),
        )),
    }
}

/// POST /tenants/{tenant_id}/organizations - 创建组织
pub async fn create_organization(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Json(req): Json<CreateOrganizationRequest>,
) -> Result<Json<OrganizationResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = UserId::new("system".to_string());

    let command = CreateOrganizationCommand {
        tenant_id,
        name: req.name.clone(),
        slug: req.slug.clone(),
        creator_id: user_id,
    };

    match state.command_bus.dispatch(command).await {
        Ok(org) => Ok(Json(OrganizationResponse {
            organization_id: org.id().to_string(),
            name: org.name().as_str().to_string(),
            slug: org.slug().as_str().to_string(),
            tenant_id: org.tenant_id().to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "CREATE_ORGANIZATION_FAILED".to_string(),
                message: e.to_string(),
            }),
        )),
    }
}

/// GET /organizations/{organization_id} - 获取组织
pub async fn get_organization(
    State(state): State<AppState>,
    Path(organization_id): Path<String>,
) -> Result<Json<OrganizationResponse>, (StatusCode, Json<ApiError>)> {
    // TODO: 从认证上下文获取 tenant_id
    let tenant_id = "system".to_string();

    let query = GetOrganizationQuery {
        tenant_id,
        organization_id,
    };

    match state.query_bus.ask(query).await {
        Ok(Some(org)) => Ok(Json(OrganizationResponse {
            organization_id: org.id().to_string(),
            name: org.name().as_str().to_string(),
            slug: org.slug().as_str().to_string(),
            tenant_id: org.tenant_id().to_string(),
        })),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiError {
                code: "ORGANIZATION_NOT_FOUND".to_string(),
                message: "Organization not found".to_string(),
            }),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "GET_ORGANIZATION_FAILED".to_string(),
                message: e.to_string(),
            }),
        )),
    }
}

/// POST /organizations/{organization_id}/teams - 创建团队
pub async fn create_team(
    State(state): State<AppState>,
    Path(organization_id): Path<String>,
    Json(req): Json<CreateTeamRequest>,
) -> Result<Json<TeamResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = UserId::new("system".to_string());

    let command = CreateTeamCommand {
        tenant_id: "TODO".to_string(), // TODO: 从组织获取
        organization_id,
        name: req.name.clone(),
        code: req.code.clone(),
        parent_team_id: None,
        creator_id: user_id,
    };

    match state.command_bus.dispatch(command).await {
        Ok(team) => Ok(Json(TeamResponse {
            team_id: team.id().to_string(),
            name: team.name().as_str().to_string(),
            code: team.code().map(|c| c.as_str().to_string()),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "CREATE_TEAM_FAILED".to_string(),
                message: e.to_string(),
            }),
        )),
    }
}

/// POST /organizations/{organization_id}/members - 邀请成员
pub async fn invite_member(
    State(state): State<AppState>,
    Path(organization_id): Path<String>,
    Json(req): Json<InviteMemberRequest>,
) -> Result<Json<MemberResponse>, (StatusCode, Json<ApiError>)> {
    let user_id = "system".to_string();
    let tenant_id = "TODO".to_string(); // TODO: 从认证上下文获取

    let role = req.role.parse::<MembershipRole>().unwrap_or(MembershipRole::Member);
    let team_id = req.team_id.as_ref().map(|id| TeamId::new(id.clone()));

    let command = InviteMemberCommand {
        tenant_id: TenantId::new(tenant_id),
        organization_id: OrganizationId::new(organization_id),
        team_id,
        user_id: UserId::new(user_id.clone()),
        email: req.email.clone(),
        role,
        inviter_id: UserId::new(user_id),
    };

    match state.command_bus.dispatch(command).await {
        Ok(membership) => Ok(Json(MemberResponse {
            id: membership.id().to_string(),
            user_id: membership.user_id().map(|u| u.to_string()).unwrap_or_default(),
            display_name: None,
            email: membership.email().as_str().to_string(),
            role: membership.role().to_string(),
            status: membership.status().to_string(),
            team_name: membership.team_id().map(|t| t.to_string()),
            joined_at: membership.created_at().to_rfc3339(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "INVITE_MEMBER_FAILED".to_string(),
                message: e.to_string(),
            }),
        )),
    }
}

/// GET /organizations/{organization_id}/members - 列出成员
pub async fn list_members(
    State(state): State<AppState>,
    Path(organization_id): Path<String>,
    Query(params): Query<ListMembersQueryParams>,
) -> Result<Json<Vec<MemberResponse>>, (StatusCode, Json<ApiError>)> {
    let tenant_id = "TODO".to_string(); // TODO: 从认证上下文获取
    let team_id = params.team_id.map(|id| TeamId::new(id));
    let status = params.status.and_then(|s| s.parse().ok());
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(50);

    let query = ListMembersQuery {
        tenant_id: TenantId::new(tenant_id),
        organization_id: OrganizationId::new(organization_id),
        team_id,
        status,
        limit: page_size as usize,
        offset: ((page - 1) * page_size) as usize,
    };

    match state.query_bus.ask(query).await {
        Ok(memberships) => {
            let members: Vec<MemberResponse> = memberships
                .iter()
                .map(|m| MemberResponse {
                    id: m.id().to_string(),
                    user_id: m.user_id().map(|u| u.to_string()).unwrap_or_default(),
                    display_name: None,
                    email: m.email().as_str().to_string(),
                    role: m.role().to_string(),
                    status: m.status().to_string(),
                    team_name: m.team_id().map(|t| t.to_string()),
                    joined_at: m.created_at().to_rfc3339(),
                })
                .collect();

            Ok(Json(members))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "LIST_MEMBERS_FAILED".to_string(),
                message: e.to_string(),
            }),
        )),
    }
}

/// POST /members/{membership_id}/accept - 接受邀请
pub async fn accept_invite(
    State(state): State<AppState>,
    Path(membership_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let user_id = "system".to_string();

    let command = AcceptInviteCommand {
        membership_id: MembershipId::new(membership_id),
        user_id: UserId::new(user_id),
    };

    match state.command_bus.dispatch(command).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "ACCEPT_INVITE_FAILED".to_string(),
                message: e.to_string(),
            }),
        )),
    }
}

/// POST /members/{membership_id}/suspend - 暂停成员
pub async fn suspend_member(
    State(state): State<AppState>,
    Path(membership_id): Path<String>,
    Json(req): Json<SuspendMemberRequest>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let user_id = "system".to_string();

    let command = SuspendMemberCommand {
        membership_id: MembershipId::new(membership_id),
        reason: req.reason,
        operator_id: UserId::new(user_id),
    };

    match state.command_bus.dispatch(command).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                code: "SUSPEND_MEMBER_FAILED".to_string(),
                message: e.to_string(),
            }),
        )),
    }
}

#[derive(Debug, Deserialize)]
pub struct SuspendMemberRequest {
    pub reason: String,
}

// 辅助函数：解析 MembershipRole
impl std::str::FromStr for MembershipRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "platformadmin" => Ok(MembershipRole::PlatformAdmin),
            "orgadmin" => Ok(MembershipRole::OrgAdmin),
            "teamadmin" => Ok(MembershipRole::TeamAdmin),
            "member" => Ok(MembershipRole::Member),
            "viewer" => Ok(MembershipRole::Viewer),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }
}

impl MembershipRole {
    fn to_string(&self) -> String {
        match self {
            MembershipRole::PlatformAdmin => "PlatformAdmin".to_string(),
            MembershipRole::OrgAdmin => "OrgAdmin".to_string(),
            MembershipRole::TeamAdmin => "TeamAdmin".to_string(),
            MembershipRole::Member => "Member".to_string(),
            MembershipRole::Viewer => "Viewer".to_string(),
        }
    }
}
