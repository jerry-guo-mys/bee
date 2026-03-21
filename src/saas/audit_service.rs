//! 审计日志服务
//!
//! 提供最小审计留痕与查询能力，供 Phase 5 的权限与管理操作接入。

use anyhow::Context;
use rusqlite::params;

use crate::saas::models::AuditLogRecord;
use crate::saas::sqlite::SaasSqliteStore;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditActor {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub team_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditLogInput {
    pub actor: AuditActor,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub detail_json: Option<String>,
}

pub fn append_audit_log(
    store: &SaasSqliteStore,
    input: AuditLogInput,
) -> anyhow::Result<AuditLogRecord> {
    let record = AuditLogRecord {
        id: format!("audit_{}", uuid::Uuid::new_v4().simple()),
        tenant_id: input.actor.tenant_id,
        organization_id: input.actor.organization_id,
        team_id: input.actor.team_id,
        user_id: input.actor.user_id,
        action: input.action,
        resource_type: input.resource_type,
        resource_id: input.resource_id,
        detail_json: input.detail_json,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    store.connection().execute(
        "INSERT INTO saas_audit_logs
         (id, tenant_id, organization_id, team_id, user_id, action, resource_type, resource_id, detail_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            record.id,
            record.tenant_id,
            record.organization_id,
            record.team_id,
            record.user_id,
            record.action,
            record.resource_type,
            record.resource_id,
            record.detail_json,
            record.created_at
        ],
    )?;
    Ok(record)
}

pub fn list_audit_logs(
    store: &SaasSqliteStore,
    tenant_id: &str,
    organization_id: Option<&str>,
    limit: usize,
) -> anyhow::Result<Vec<AuditLogRecord>> {
    let limit = limit.max(1) as i64;
    let mut logs = Vec::new();
    if let Some(organization_id) = organization_id {
        let mut stmt = store.connection().prepare(
            "SELECT id, tenant_id, organization_id, team_id, user_id, action, resource_type, resource_id, detail_json, created_at
             FROM saas_audit_logs
             WHERE tenant_id = ?1 AND organization_id = ?2
             ORDER BY created_at DESC
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![tenant_id, organization_id, limit], map_audit_log_row)?;
        for row in rows {
            logs.push(row?);
        }
    } else {
        let mut stmt = store.connection().prepare(
            "SELECT id, tenant_id, organization_id, team_id, user_id, action, resource_type, resource_id, detail_json, created_at
             FROM saas_audit_logs
             WHERE tenant_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![tenant_id, limit], map_audit_log_row)?;
        for row in rows {
            logs.push(row?);
        }
    }
    Ok(logs)
}

fn map_audit_log_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AuditLogRecord> {
    Ok(AuditLogRecord {
        id: row.get(0)?,
        tenant_id: row.get(1)?,
        organization_id: row.get(2)?,
        team_id: row.get(3)?,
        user_id: row.get(4)?,
        action: row.get(5)?,
        resource_type: row.get(6)?,
        resource_id: row.get(7)?,
        detail_json: row.get(8)?,
        created_at: row.get(9)?,
    })
}

pub fn detail_json(value: serde_json::Value) -> anyhow::Result<String> {
    serde_json::to_string(&value).context("serialize audit detail json")
}
