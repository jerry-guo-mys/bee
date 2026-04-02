//! 管理 API 处理器
//!
//! 提供多租户管理相关的 HTTP API

use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use bee::saas::TenantStatus;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use crate::server::{AppState, saas_db_path};
use bee::saas::SaasSqliteStore;

type SaasStore = Arc<Mutex<SaasSqliteStore>>;

/// 创建管理路由（需要外部提供状态）
pub(crate) fn create_router(workspace: &std::path::Path) -> Router<Arc<AppState>> {
    let saas_store = Arc::new(Mutex::new(
        SaasSqliteStore::new(saas_db_path(workspace))
            .unwrap_or_else(|_| panic!("Failed to initialize SaaS SQLite store"))
    ));

    Router::new()
        .route("/api/admin/tenants", get(list_tenants).post(create_tenant))
        .route("/api/admin/tenants/:id", get(get_tenant))
        .route("/api/admin/tenants/:id/suspend", post(suspend_tenant))
        .route("/api/admin/tenants/:id/restore", post(restore_tenant))
        .route("/api/admin/tenants/:id/archive", post(archive_tenant))
        .route("/api/admin/organizations", get(list_organizations).post(create_organization))
        .route("/api/admin/organizations/:id", get(get_organization))
        .route("/api/admin/teams", get(list_teams).post(create_team))
        .route("/api/admin/teams/:id", get(get_team))
        .route("/api/admin/members", get(list_members))
        .route("/api/admin/audit-logs", get(list_audit_logs_enhanced))
        .layer(Extension(saas_store))
}

// ==================== 租户管理 ====================

#[derive(Debug, Serialize)]
pub struct TenantListResponse {
    pub tenants: Vec<TenantSummary>,
    pub total: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct TenantSummary {
    pub id: String,
    pub name: String,
    pub status: String,
    pub organization_count: usize,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct TenantDetailResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub organizations: Vec<OrganizationSummary>,
}

#[derive(Debug, Deserialize)]
pub struct ListTenantsQuery {
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
}

async fn list_tenants(
    Query(params): Query<ListTenantsQuery>,
    Extension(saas_store): Extension<SaasStore>,
) -> Result<Json<TenantListResponse>, (StatusCode, String)> {
    let saas_store = saas_store.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let tenants = saas_store.list_tenants().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    drop(saas_store);

    let summaries: Vec<TenantSummary> = tenants
        .into_iter()
        .filter(|t| params.status.as_ref().map_or(true, |status| &t.status.to_string() == status))
        .map(|t| TenantSummary { id: t.id, name: t.name, status: t.status.to_string(), organization_count: 0, created_at: t.created_at })
        .collect();

    Ok(Json(TenantListResponse { tenants: summaries.clone(), total: summaries.len() }))
}

async fn create_tenant(
    Extension(saas_store): Extension<SaasStore>,
    Json(payload): Json<CreateTenantRequest>,
) -> Result<Json<TenantSummary>, (StatusCode, String)> {
    let now = Utc::now().to_rfc3339();
    let tenant = bee::saas::Tenant { id: Uuid::new_v4().to_string(), name: payload.name.clone(), status: TenantStatus::Active, created_at: now.clone(), updated_at: now };
    { let saas_store = saas_store.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?; saas_store.create_tenant(&tenant).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?; }
    Ok(Json(TenantSummary { id: tenant.id.clone(), name: tenant.name.clone(), status: tenant.status.to_string(), organization_count: 0, created_at: tenant.created_at.clone() }))
}

async fn get_tenant(Path(id): Path<String>, Extension(saas_store): Extension<SaasStore>) -> Result<Json<TenantDetailResponse>, (StatusCode, String)> {
    let saas_store = saas_store.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let tenant = saas_store.get_tenant(&id).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?.ok_or_else(|| (StatusCode::NOT_FOUND, "Tenant not found".to_string()))?;
    let organizations = saas_store.list_organizations(Some(&id)).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?.into_iter().map(|o| OrganizationSummary { id: o.id, tenant_id: o.tenant_id, name: o.name, slug: o.slug.unwrap_or_default(), member_count: 0, member_limit: None, created_at: o.created_at }).collect();
    Ok(Json(TenantDetailResponse { id: tenant.id, name: tenant.name, status: tenant.status.to_string(), created_at: tenant.created_at, updated_at: tenant.updated_at, organizations }))
}

async fn suspend_tenant(Path(id): Path<String>, Extension(saas_store): Extension<SaasStore>) -> Result<StatusCode, (StatusCode, String)> {
    let saas_store = saas_store.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    saas_store.update_tenant_status(&id, &TenantStatus::Suspended).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn restore_tenant(Path(id): Path<String>, Extension(saas_store): Extension<SaasStore>) -> Result<StatusCode, (StatusCode, String)> {
    let saas_store = saas_store.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    saas_store.update_tenant_status(&id, &TenantStatus::Active).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn archive_tenant(Path(id): Path<String>, Extension(saas_store): Extension<SaasStore>) -> Result<StatusCode, (StatusCode, String)> {
    let saas_store = saas_store.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    saas_store.update_tenant_status(&id, &TenantStatus::Archived).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

// ==================== 组织管理 ====================

#[derive(Debug, Serialize)]
pub struct OrganizationListResponse { pub organizations: Vec<OrganizationSummary>, pub total: usize }
#[derive(Debug, Serialize, Clone)]
pub struct OrganizationSummary { pub id: String, pub tenant_id: String, pub name: String, pub slug: String, pub member_count: usize, pub member_limit: Option<i32>, pub created_at: String }
#[derive(Debug, Deserialize)]
pub struct ListOrganizationsQuery { pub tenant_id: Option<String> }
#[derive(Debug, Deserialize)]
pub struct CreateOrganizationRequest { pub tenant_id: String, pub name: String, pub slug: Option<String> }

async fn list_organizations(Query(params): Query<ListOrganizationsQuery>, Extension(saas_store): Extension<SaasStore>) -> Result<Json<OrganizationListResponse>, (StatusCode, String)> {
    let saas_store = saas_store.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let organizations = saas_store.list_organizations(params.tenant_id.as_deref()).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let summaries: Vec<OrganizationSummary> = organizations.into_iter().map(|o| OrganizationSummary { id: o.id, tenant_id: o.tenant_id, name: o.name, slug: o.slug.unwrap_or_default(), member_count: 0, member_limit: None, created_at: o.created_at }).collect();
    Ok(Json(OrganizationListResponse { organizations: summaries.clone(), total: summaries.len() }))
}

async fn create_organization(Extension(saas_store): Extension<SaasStore>, Json(payload): Json<CreateOrganizationRequest>) -> Result<Json<OrganizationSummary>, (StatusCode, String)> {
    let now = Utc::now().to_rfc3339();
    let org = bee::saas::Organization { id: Uuid::new_v4().to_string(), tenant_id: payload.tenant_id.clone(), name: payload.name.clone(), slug: payload.slug.clone(), industry: None, description: None, created_at: now.clone(), updated_at: now };
    { let saas_store = saas_store.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?; saas_store.create_organization(&org).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?; }
    Ok(Json(OrganizationSummary { id: org.id.clone(), tenant_id: org.tenant_id.clone(), name: org.name.clone(), slug: org.slug.unwrap_or_default(), member_count: 0, member_limit: None, created_at: org.created_at.clone() }))
}

async fn get_organization(Path(id): Path<String>, Extension(saas_store): Extension<SaasStore>) -> Result<Json<OrganizationSummary>, (StatusCode, String)> {
    let saas_store = saas_store.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let org = saas_store.get_organization(&id).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?.ok_or_else(|| (StatusCode::NOT_FOUND, "Organization not found".to_string()))?;
    Ok(Json(OrganizationSummary { id: org.id, tenant_id: org.tenant_id, name: org.name, slug: org.slug.unwrap_or_default(), member_count: 0, member_limit: None, created_at: org.created_at }))
}

// ==================== 团队管理 ====================

#[derive(Debug, Serialize)]
pub struct TeamListResponse { pub teams: Vec<TeamSummary>, pub total: usize }
#[derive(Debug, Serialize, Clone)]
pub struct TeamSummary { pub id: String, pub organization_id: String, pub name: String, pub code: Option<String>, pub description: Option<String>, pub parent_team_id: Option<String>, pub member_count: usize, pub created_at: String }
#[derive(Debug, Deserialize)]
pub struct ListTeamsQuery { pub organization_id: Option<String> }
#[derive(Debug, Deserialize)]
pub struct CreateTeamRequest { pub organization_id: String, pub name: String, pub code: Option<String>, pub description: Option<String>, pub parent_team_id: Option<String> }

async fn list_teams(Query(params): Query<ListTeamsQuery>, Extension(saas_store): Extension<SaasStore>) -> Result<Json<TeamListResponse>, (StatusCode, String)> {
    let saas_store = saas_store.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let teams = saas_store.list_teams(params.organization_id.as_deref()).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let summaries: Vec<TeamSummary> = teams.into_iter().map(|t| TeamSummary { id: t.id, organization_id: t.organization_id, name: t.name, code: t.code, description: t.description, parent_team_id: t.parent_team_id, member_count: 0, created_at: t.created_at }).collect();
    Ok(Json(TeamListResponse { teams: summaries.clone(), total: summaries.len() }))
}

async fn create_team(Extension(saas_store): Extension<SaasStore>, Json(payload): Json<CreateTeamRequest>) -> Result<Json<TeamSummary>, (StatusCode, String)> {
    let now = Utc::now().to_rfc3339();
    // Handle empty string as None for parent_team_id to avoid FK constraint failure
    let parent_team_id = payload.parent_team_id.filter(|s| !s.is_empty());
    let team = bee::saas::Team { id: Uuid::new_v4().to_string(), tenant_id: String::new(), organization_id: payload.organization_id.clone(), name: payload.name.clone(), code: payload.code.clone(), description: payload.description.clone(), parent_team_id: parent_team_id.clone(), created_at: now.clone(), updated_at: now };
    { let saas_store = saas_store.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?; saas_store.create_team(&team).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?; }
    Ok(Json(TeamSummary { id: team.id.clone(), organization_id: team.organization_id.clone(), name: team.name.clone(), code: team.code.clone(), description: team.description.clone(), parent_team_id: team.parent_team_id.clone(), member_count: 0, created_at: team.created_at.clone() }))
}

async fn get_team(Path(id): Path<String>, Extension(saas_store): Extension<SaasStore>) -> Result<Json<TeamSummary>, (StatusCode, String)> {
    let saas_store = saas_store.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let team = saas_store.get_team(&id).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?.ok_or_else(|| (StatusCode::NOT_FOUND, "Team not found".to_string()))?;
    Ok(Json(TeamSummary { id: team.id, organization_id: team.organization_id, name: team.name, code: team.code, description: team.description, parent_team_id: team.parent_team_id, member_count: 0, created_at: team.created_at }))
}

// ==================== 成员管理 ====================

#[derive(Debug, Serialize)]
pub struct MemberListResponse { pub members: Vec<MemberSummary>, pub total: usize }
#[derive(Debug, Serialize, Clone)]
pub struct MemberSummary { pub id: String, pub tenant_id: String, pub organization_id: String, pub team_id: Option<String>, pub user_id: String, pub email: Option<String>, pub role: String, pub status: String, pub created_at: String }
#[derive(Debug, Deserialize)]
pub struct ListMembersQuery { pub tenant_id: Option<String>, pub organization_id: Option<String>, pub role: Option<String> }

async fn list_members(Query(params): Query<ListMembersQuery>, Extension(saas_store): Extension<SaasStore>) -> Result<Json<MemberListResponse>, (StatusCode, String)> {
    let saas_store = saas_store.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let memberships = saas_store.list_memberships(params.tenant_id.as_deref(), params.organization_id.as_deref()).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let summaries: Vec<MemberSummary> = memberships.into_iter().filter(|m| params.role.as_ref().map_or(true, |role| &m.role.to_string() == role)).map(|m| MemberSummary { id: m.id, tenant_id: m.tenant_id, organization_id: m.organization_id, team_id: m.team_id, user_id: m.user_id, email: None, role: m.role.to_string(), status: "active".to_string(), created_at: m.created_at }).collect();
    Ok(Json(MemberListResponse { members: summaries.clone(), total: summaries.len() }))
}

// ==================== 审计日志增强 ====================

#[derive(Debug, Deserialize)]
pub struct ListAuditLogsQuery { pub tenant_id: Option<String>, pub organization_id: Option<String>, pub limit: Option<usize> }

async fn list_audit_logs_enhanced(Query(params): Query<ListAuditLogsQuery>, Extension(saas_store): Extension<SaasStore>) -> Result<Json<Vec<bee::saas::AuditLogRecord>>, (StatusCode, String)> {
    let saas_store = saas_store.lock().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let limit = params.limit.unwrap_or(100) as i64;
    let logs = saas_store.list_audit_logs(params.tenant_id.as_deref(), params.organization_id.as_deref(), limit).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(logs))
}
