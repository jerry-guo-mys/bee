//! SaaS 模板目录
//!
//! 将静态 assistants 配置转换为平台级 Agent 模板种子，作为后续租户覆盖与团队实例化的基础来源。

use std::path::{Path, PathBuf};

use crate::saas::models::AgentTemplate;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TemplateAssistantEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub prompt: String,
    #[serde(default)]
    pub skills: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize)]
struct TemplateAssistantsConfig {
    assistants: Vec<TemplateAssistantEntry>,
}

pub fn load_platform_agent_templates(
    config_base: &Path,
    tenant_id: &str,
) -> anyhow::Result<Vec<AgentTemplate>> {
    let now = chrono::Utc::now().to_rfc3339();
    let base = resolve_config_base(config_base);
    let entries = load_template_entries(config_base)?;

    Ok(entries
        .into_iter()
        .map(|entry| {
            let prompt = resolve_prompt_content(&base, &entry);
            AgentTemplate {
                id: format!("platform-template-{}", entry.id),
                tenant_id: tenant_id.to_string(),
                name: entry.name,
                description: Some(entry.description),
                prompt: Some(prompt),
                tool_ids: entry.skills.unwrap_or_default(),
                model_id: None,
                knowledge_base_ids: Vec::new(),
                created_at: now.clone(),
                updated_at: now.clone(),
            }
        })
        .collect())
}

fn load_template_entries(config_base: &Path) -> anyhow::Result<Vec<TemplateAssistantEntry>> {
    let toml_path = [
        config_base.join("assistants.toml"),
        Path::new("config/assistants.toml").to_path_buf(),
        Path::new("../config/assistants.toml").to_path_buf(),
    ]
    .into_iter()
    .find(|path| path.exists())
    .ok_or_else(|| anyhow::anyhow!("assistants.toml not found"))?;

    let content = std::fs::read_to_string(&toml_path)?;
    let parsed = toml::from_str::<TemplateAssistantsConfig>(&content)?;
    Ok(parsed.assistants)
}

fn resolve_config_base(config_base: &Path) -> PathBuf {
    if config_base.is_absolute() {
        config_base.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(config_base)
    }
}

fn resolve_prompt_content(base: &Path, entry: &TemplateAssistantEntry) -> String {
    let prompt_path = [
        base.join(&entry.prompt),
        Path::new("config").join(&entry.prompt),
        Path::new("../config").join(&entry.prompt),
    ]
    .into_iter()
    .find(|path| path.exists());

    prompt_path
        .and_then(|path| std::fs::read_to_string(path).ok())
        .unwrap_or_else(|| format!("You are {}, a helpful assistant.", entry.name))
}
