//! SaaS 权限服务
//!
//! 提供最小角色解析与管理操作授权判断。

use anyhow::Context;
use rusqlite::params;

use crate::saas::models::MembershipRole;
use crate::saas::sqlite::SaasSqliteStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessRequirement {
    PlatformAdmin,
    OrgAdmin,
    TeamAdmin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessContext {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub team_id: Option<String>,
    pub user_id: String,
}

pub fn ensure_access(
    store: &SaasSqliteStore,
    ctx: &AccessContext,
    requirement: AccessRequirement,
) -> Result<MembershipRole, String> {
    let roles = list_user_roles(
        store,
        &ctx.tenant_id,
        ctx.organization_id.as_deref(),
        &ctx.user_id,
    )
    .map_err(|err| err.to_string())?;
    let strongest = strongest_role(&roles, ctx.team_id.as_deref());
    if role_satisfies(strongest.as_ref(), requirement, ctx.team_id.as_deref()) {
        return strongest.ok_or_else(|| "role resolution failed".to_string());
    }

    if can_bootstrap_empty_tenant(store, ctx, requirement).map_err(|err| err.to_string())? {
        return Ok(MembershipRole::PlatformAdmin);
    }

    Err("permission denied".to_string())
}

fn list_user_roles(
    store: &SaasSqliteStore,
    tenant_id: &str,
    organization_id: Option<&str>,
    user_id: &str,
) -> anyhow::Result<Vec<(MembershipRole, Option<String>)>> {
    let mut roles = Vec::new();
    if let Some(organization_id) = organization_id {
        let mut stmt = store.connection().prepare(
            "SELECT role, team_id
             FROM saas_memberships
             WHERE tenant_id = ?1 AND organization_id = ?2 AND user_id = ?3",
        )?;
        let rows = stmt.query_map(params![tenant_id, organization_id, user_id], |row| {
            Ok((
                parse_role(&row.get::<_, String>(0)?),
                row.get::<_, Option<String>>(1)?,
            ))
        })?;
        for row in rows {
            roles.push(row?);
        }
    } else {
        let mut stmt = store.connection().prepare(
            "SELECT role, team_id
             FROM saas_memberships
             WHERE tenant_id = ?1 AND user_id = ?2",
        )?;
        let rows = stmt.query_map(params![tenant_id, user_id], |row| {
            Ok((
                parse_role(&row.get::<_, String>(0)?),
                row.get::<_, Option<String>>(1)?,
            ))
        })?;
        for row in rows {
            roles.push(row?);
        }
    }
    Ok(roles)
}

fn strongest_role(
    roles: &[(MembershipRole, Option<String>)],
    requested_team_id: Option<&str>,
) -> Option<MembershipRole> {
    if roles
        .iter()
        .any(|(role, _)| matches!(role, MembershipRole::PlatformAdmin))
    {
        return Some(MembershipRole::PlatformAdmin);
    }
    if roles
        .iter()
        .any(|(role, _)| matches!(role, MembershipRole::OrgAdmin))
    {
        return Some(MembershipRole::OrgAdmin);
    }
    if let Some(team_id) = requested_team_id {
        if roles.iter().any(|(role, membership_team_id)| {
            matches!(role, MembershipRole::TeamAdmin)
                && membership_team_id.as_deref() == Some(team_id)
        }) {
            return Some(MembershipRole::TeamAdmin);
        }
    }
    if roles
        .iter()
        .any(|(role, _)| matches!(role, MembershipRole::Member))
    {
        return Some(MembershipRole::Member);
    }
    None
}

fn role_satisfies(
    role: Option<&MembershipRole>,
    requirement: AccessRequirement,
    requested_team_id: Option<&str>,
) -> bool {
    match (role, requirement) {
        (Some(MembershipRole::PlatformAdmin), _) => true,
        (Some(MembershipRole::OrgAdmin), AccessRequirement::OrgAdmin) => true,
        (Some(MembershipRole::OrgAdmin), AccessRequirement::TeamAdmin) => true,
        (Some(MembershipRole::TeamAdmin), AccessRequirement::TeamAdmin) => {
            requested_team_id.is_some()
        }
        _ => false,
    }
}

fn can_bootstrap_empty_tenant(
    store: &SaasSqliteStore,
    ctx: &AccessContext,
    requirement: AccessRequirement,
) -> anyhow::Result<bool> {
    if !matches!(requirement, AccessRequirement::PlatformAdmin) {
        return Ok(false);
    }
    if ctx.user_id != "user-default" && ctx.user_id != "legacy-user" {
        return Ok(false);
    }
    let count: i64 = store
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM saas_memberships WHERE tenant_id = ?1",
            params![ctx.tenant_id],
            |row| row.get(0),
        )
        .context("count tenant memberships")?;
    Ok(count == 0)
}

fn parse_role(value: &str) -> MembershipRole {
    match value {
        "platform_admin" => MembershipRole::PlatformAdmin,
        "team_admin" => MembershipRole::TeamAdmin,
        "member" => MembershipRole::Member,
        _ => MembershipRole::OrgAdmin,
    }
}
