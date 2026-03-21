//! 工具访问策略服务
//!
//! 提供租户/组织/团队级工具白名单与黑名单覆盖能力。

use rusqlite::params;

use crate::saas::models::ToolAccessPolicy;
use crate::saas::sqlite::SaasSqliteStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPolicyScope {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub team_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPolicyInput {
    pub scope: ToolPolicyScope,
    pub allowed_tool_ids: Vec<String>,
    pub denied_tool_ids: Vec<String>,
}

pub fn upsert_tool_policy(
    store: &SaasSqliteStore,
    input: ToolPolicyInput,
) -> anyhow::Result<ToolAccessPolicy> {
    let now = chrono::Utc::now().to_rfc3339();
    let id = policy_id(&input.scope);
    let policy = ToolAccessPolicy {
        id,
        tenant_id: input.scope.tenant_id,
        organization_id: input.scope.organization_id,
        team_id: input.scope.team_id,
        allowed_tool_ids: dedup_preserve_order(input.allowed_tool_ids),
        denied_tool_ids: dedup_preserve_order(input.denied_tool_ids),
        created_at: now.clone(),
        updated_at: now,
    };

    store.connection().execute(
        "INSERT OR REPLACE INTO saas_tool_access_policies
         (id, tenant_id, organization_id, team_id, allowed_tool_ids_json, denied_tool_ids_json, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, COALESCE((SELECT created_at FROM saas_tool_access_policies WHERE id = ?1), ?7), ?8)",
        params![
            policy.id,
            policy.tenant_id,
            policy.organization_id,
            policy.team_id,
            serde_json::to_string(&policy.allowed_tool_ids)?,
            serde_json::to_string(&policy.denied_tool_ids)?,
            policy.created_at,
            policy.updated_at
        ],
    )?;
    Ok(policy)
}

pub fn list_tool_policies(
    store: &SaasSqliteStore,
    tenant_id: &str,
    organization_id: Option<&str>,
) -> anyhow::Result<Vec<ToolAccessPolicy>> {
    let mut policies = Vec::new();
    if let Some(organization_id) = organization_id {
        let mut stmt = store.connection().prepare(
            "SELECT id, tenant_id, organization_id, team_id, allowed_tool_ids_json, denied_tool_ids_json, created_at, updated_at
             FROM saas_tool_access_policies
             WHERE tenant_id = ?1 AND (organization_id IS NULL OR organization_id = ?2)
             ORDER BY organization_id ASC, team_id ASC",
        )?;
        let rows = stmt.query_map(params![tenant_id, organization_id], map_tool_policy_row)?;
        for row in rows {
            policies.push(row?);
        }
    } else {
        let mut stmt = store.connection().prepare(
            "SELECT id, tenant_id, organization_id, team_id, allowed_tool_ids_json, denied_tool_ids_json, created_at, updated_at
             FROM saas_tool_access_policies
             WHERE tenant_id = ?1
             ORDER BY organization_id ASC, team_id ASC",
        )?;
        let rows = stmt.query_map(params![tenant_id], map_tool_policy_row)?;
        for row in rows {
            policies.push(row?);
        }
    }
    Ok(policies)
}

pub fn resolve_effective_tool_allowlist(
    store: &SaasSqliteStore,
    scope: &ToolPolicyScope,
    default_tools: &[String],
) -> anyhow::Result<Vec<String>> {
    let mut tools = default_tools.to_vec();

    if let Some(policy) = load_policy(
        store,
        &ToolPolicyScope {
            tenant_id: scope.tenant_id.clone(),
            organization_id: None,
            team_id: None,
        },
    )? {
        apply_policy(&mut tools, &policy);
    }

    if let Some(organization_id) = scope.organization_id.as_ref() {
        if let Some(policy) = load_policy(
            store,
            &ToolPolicyScope {
                tenant_id: scope.tenant_id.clone(),
                organization_id: Some(organization_id.clone()),
                team_id: None,
            },
        )? {
            apply_policy(&mut tools, &policy);
        }
    }

    if let (Some(organization_id), Some(team_id)) =
        (scope.organization_id.as_ref(), scope.team_id.as_ref())
    {
        if let Some(policy) = load_policy(
            store,
            &ToolPolicyScope {
                tenant_id: scope.tenant_id.clone(),
                organization_id: Some(organization_id.clone()),
                team_id: Some(team_id.clone()),
            },
        )? {
            apply_policy(&mut tools, &policy);
        }
    }

    Ok(dedup_preserve_order(tools))
}

fn load_policy(
    store: &SaasSqliteStore,
    scope: &ToolPolicyScope,
) -> anyhow::Result<Option<ToolAccessPolicy>> {
    let mut stmt = store.connection().prepare(
        "SELECT id, tenant_id, organization_id, team_id, allowed_tool_ids_json, denied_tool_ids_json, created_at, updated_at
         FROM saas_tool_access_policies
         WHERE id = ?1
         LIMIT 1",
    )?;
    let mut rows = stmt.query(params![policy_id(scope)])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(map_tool_policy_row(row).map(Some)?)
}

fn apply_policy(tools: &mut Vec<String>, policy: &ToolAccessPolicy) {
    tools.retain(|tool| !policy.denied_tool_ids.iter().any(|deny| deny == tool));
    for tool in &policy.allowed_tool_ids {
        if !tools.iter().any(|existing| existing == tool) {
            tools.push(tool.clone());
        }
    }
}

fn map_tool_policy_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ToolAccessPolicy> {
    let allowed_json: String = row.get(4)?;
    let denied_json: String = row.get(5)?;
    Ok(ToolAccessPolicy {
        id: row.get(0)?,
        tenant_id: row.get(1)?,
        organization_id: row.get(2)?,
        team_id: row.get(3)?,
        allowed_tool_ids: serde_json::from_str(&allowed_json).unwrap_or_default(),
        denied_tool_ids: serde_json::from_str(&denied_json).unwrap_or_default(),
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn policy_id(scope: &ToolPolicyScope) -> String {
    let organization_id = scope.organization_id.as_deref().unwrap_or("global");
    let team_id = scope.team_id.as_deref().unwrap_or("global");
    format!(
        "tool-policy-{}-{}-{}",
        sanitize_segment(&scope.tenant_id),
        sanitize_segment(organization_id),
        sanitize_segment(team_id)
    )
}

fn sanitize_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn dedup_preserve_order(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            result.push(value);
        }
    }
    result
}

pub fn default_low_risk_tools(tools: &[String]) -> Vec<String> {
    let high_risk = [
        "shell",
        "code_edit",
        "code_write",
        "git_commit",
        "create",
        "create_group",
        "send",
        "browser",
    ];
    tools
        .iter()
        .filter(|tool| !high_risk.contains(&tool.as_str()))
        .cloned()
        .collect()
}
