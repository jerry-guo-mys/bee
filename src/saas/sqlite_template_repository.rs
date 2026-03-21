//! SaaS 模板读取仓储
//!
//! 当前阶段先提供 AgentTemplate 的 sqlite 读取能力，供平台模板中心替代静态文件直读。

use rusqlite::params;

use crate::saas::models::AgentTemplate;
use crate::saas::sqlite::SaasSqliteStore;

pub struct SaasTemplateRepository<'a> {
    store: &'a SaasSqliteStore,
}

impl<'a> SaasTemplateRepository<'a> {
    pub fn new(store: &'a SaasSqliteStore) -> Self {
        Self { store }
    }

    pub fn get_agent_template(&self, template_id: &str) -> anyhow::Result<Option<AgentTemplate>> {
        let mut stmt = self.store.connection().prepare(
            "SELECT id, tenant_id, name, description, prompt, tool_ids_json, model_id, knowledge_base_ids_json, created_at, updated_at
             FROM saas_agent_templates
             WHERE id = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![template_id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        let tool_ids_json: String = row.get(5)?;
        let knowledge_base_ids_json: Option<String> = row.get(7)?;
        Ok(Some(AgentTemplate {
            id: row.get(0)?,
            tenant_id: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            prompt: row.get(4)?,
            tool_ids: serde_json::from_str(&tool_ids_json).unwrap_or_default(),
            model_id: row.get(6)?,
            knowledge_base_ids: knowledge_base_ids_json
                .as_deref()
                .and_then(|value| serde_json::from_str(value).ok())
                .unwrap_or_default(),
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        }))
    }

    pub fn list_agent_templates(&self, tenant_id: &str) -> anyhow::Result<Vec<AgentTemplate>> {
        let mut stmt = self.store.connection().prepare(
            "SELECT id, tenant_id, name, description, prompt, tool_ids_json, model_id, knowledge_base_ids_json, created_at, updated_at
             FROM saas_agent_templates
             WHERE tenant_id = ?1
             ORDER BY name ASC",
        )?;
        let rows = stmt.query_map(params![tenant_id], |row| {
            let tool_ids_json: String = row.get(5)?;
            let knowledge_base_ids_json: Option<String> = row.get(7)?;
            Ok(AgentTemplate {
                id: row.get(0)?,
                tenant_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                prompt: row.get(4)?,
                tool_ids: serde_json::from_str(&tool_ids_json).unwrap_or_default(),
                model_id: row.get(6)?,
                knowledge_base_ids: knowledge_base_ids_json
                    .as_deref()
                    .and_then(|value| serde_json::from_str(value).ok())
                    .unwrap_or_default(),
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut templates = Vec::new();
        for row in rows {
            templates.push(row?);
        }
        Ok(templates)
    }

    pub fn upsert_agent_template(&self, template: &AgentTemplate) -> anyhow::Result<()> {
        self.store.connection().execute(
            "INSERT OR REPLACE INTO saas_agent_templates
             (id, tenant_id, name, description, prompt, tool_ids_json, model_id, knowledge_base_ids_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                template.id,
                template.tenant_id,
                template.name,
                template.description,
                template.prompt,
                serde_json::to_string(&template.tool_ids)?,
                template.model_id,
                serde_json::to_string(&template.knowledge_base_ids)?,
                template.created_at,
                template.updated_at
            ],
        )?;
        Ok(())
    }
}
