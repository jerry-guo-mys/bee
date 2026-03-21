use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use bee::saas::{SaasSqliteStore, SaasTemplateRepository};
use bee::tools::tool_call_schema_json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct AssistantInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssistantEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub prompt: String,
    #[serde(default)]
    pub prompt_text: Option<String>,
    #[serde(default)]
    pub skills: Option<Vec<String>>,
    #[serde(default)]
    pub knowledge_bases: Option<Vec<String>>,
}

pub fn platform_template_id(assistant_id: &str) -> String {
    format!("platform-template-{}", assistant_id)
}

#[derive(Debug, Deserialize)]
struct AssistantsConfig {
    assistants: Vec<AssistantEntry>,
}

#[derive(Debug, Deserialize)]
struct SingleSkillConfig {
    assistant: AssistantEntry,
}

pub fn load_skills_overrides(config_base: &Path) -> HashMap<String, Vec<String>> {
    let paths = [
        config_base.join("assistant_skills.json"),
        Path::new("config/assistant_skills.json").to_path_buf(),
        Path::new("../config/assistant_skills.json").to_path_buf(),
    ];
    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(overrides) = serde_json::from_str(&content) {
                return overrides;
            }
        }
    }
    HashMap::new()
}

pub fn save_skills_overrides(
    config_base: &Path,
    overrides: &HashMap<String, Vec<String>>,
) -> std::io::Result<()> {
    let path = config_base.join("assistant_skills.json");
    std::fs::create_dir_all(config_base).ok();
    let content = serde_json::to_string_pretty(overrides)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    std::fs::write(path, content)
}

pub fn load_knowledge_overrides(config_base: &Path) -> HashMap<String, Vec<String>> {
    let paths = [
        config_base.join("assistant_knowledge.json"),
        Path::new("config/assistant_knowledge.json").to_path_buf(),
        Path::new("../config/assistant_knowledge.json").to_path_buf(),
    ];
    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(overrides) = serde_json::from_str(&content) {
                return overrides;
            }
        }
    }
    HashMap::new()
}

pub fn save_knowledge_overrides(
    config_base: &Path,
    overrides: &HashMap<String, Vec<String>>,
) -> std::io::Result<()> {
    let path = config_base.join("assistant_knowledge.json");
    std::fs::create_dir_all(config_base).ok();
    let content = serde_json::to_string_pretty(overrides)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    std::fs::write(path, content)
}

pub fn load_assistants(
    config_base: &Path,
    tool_descriptions: &[(String, String)],
    template_db_path: Option<&Path>,
    tenant_id: &str,
) -> (
    Vec<AssistantInfo>,
    HashMap<String, String>,
    HashMap<String, Vec<String>>,
    HashMap<String, AssistantEntry>,
) {
    let mut entries = load_assistants_from_templates(template_db_path, tenant_id)
        .filter(|templates| !templates.is_empty())
        .unwrap_or_else(|| load_assistants_from_files(config_base));

    for entry in load_skill_assistants(config_base) {
        if let Some(existing) = entries.iter_mut().find(|current| current.id == entry.id) {
            *existing = entry;
        } else {
            entries.push(entry);
        }
    }

    let overrides = load_skills_overrides(config_base);
    let tool_schema = tool_call_schema_json();
    let base = resolve_config_base(config_base);
    let all_names: HashSet<_> = tool_descriptions
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();

    let mut prompts = HashMap::new();
    let mut skills_map = HashMap::new();
    let mut entries_map = HashMap::new();

    for entry in &entries {
        let allowed = resolve_allowed_tools(entry, &overrides, &all_names, tool_descriptions);
        skills_map.insert(entry.id.clone(), allowed.clone());
        entries_map.insert(entry.id.clone(), entry.clone());
        let full_prompt =
            build_prompt_with_skills(&base, entry, &allowed, tool_descriptions, &tool_schema);
        prompts.insert(entry.id.clone(), full_prompt);
    }

    let assistants = entries
        .iter()
        .map(|entry| AssistantInfo {
            id: entry.id.clone(),
            name: entry.name.clone(),
            description: entry.description.clone(),
            skills: Some(skills_map.get(&entry.id).cloned().unwrap_or_default()),
        })
        .collect();

    (assistants, prompts, skills_map, entries_map)
}

pub fn build_prompt_with_skills(
    config_base: &Path,
    entry: &AssistantEntry,
    skills: &[String],
    tool_descriptions: &[(String, String)],
    tool_schema: &str,
) -> String {
    let tool_list = tool_descriptions
        .iter()
        .filter(|(name, _)| skills.contains(name))
        .map(|(name, desc)| format!("- {}: {}", name, desc))
        .collect::<Vec<_>>()
        .join("\n");

    let content = resolve_prompt_content(config_base, entry);
    let tools_section = if tool_list.is_empty() {
        String::new()
    } else {
        format!("\n\nAvailable tools:\n{}\n", tool_list)
    };

    if tool_schema.is_empty() {
        format!("{}{}", content, tools_section)
    } else {
        format!(
            "{}{}\n\n## Tool call JSON Schema (you must output valid JSON matching this)\n```json\n{}\n```",
            content, tools_section, tool_schema
        )
    }
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

fn load_assistants_from_templates(
    template_db_path: Option<&Path>,
    tenant_id: &str,
) -> Option<Vec<AssistantEntry>> {
    let db_path = template_db_path?;
    let store = SaasSqliteStore::new(db_path).ok()?;
    let repo = SaasTemplateRepository::new(&store);
    let templates = repo.list_agent_templates(tenant_id).ok()?;
    Some(
        templates
            .into_iter()
            .map(|template| AssistantEntry {
                id: template
                    .id
                    .trim_start_matches("platform-template-")
                    .to_string(),
                name: template.name,
                description: template
                    .description
                    .unwrap_or_else(|| "平台模板助手".to_string()),
                prompt: String::new(),
                prompt_text: template.prompt,
                skills: Some(template.tool_ids),
                knowledge_bases: Some(template.knowledge_base_ids),
            })
            .collect(),
    )
}

fn load_assistants_from_files(config_base: &Path) -> Vec<AssistantEntry> {
    let toml_path = [
        config_base.join("assistants.toml"),
        Path::new("config/assistants.toml").to_path_buf(),
        Path::new("../config/assistants.toml").to_path_buf(),
    ]
    .into_iter()
    .find(|path| path.exists());

    match toml_path.and_then(|path| std::fs::read_to_string(path).ok()) {
        Some(content) => toml::from_str::<AssistantsConfig>(&content)
            .map(|config| config.assistants)
            .unwrap_or_default(),
        None => vec![AssistantEntry {
            id: "default".to_string(),
            name: "通用助手".to_string(),
            description: "全能型个人助手".to_string(),
            prompt: "prompts/system.md".to_string(),
            prompt_text: None,
            skills: None,
            knowledge_bases: None,
        }],
    }
}

fn load_skill_assistants(config_base: &Path) -> Vec<AssistantEntry> {
    let skills_dirs = [
        config_base.join("skills"),
        Path::new("config/skills").to_path_buf(),
        Path::new("../config/skills").to_path_buf(),
    ];

    for skills_dir in skills_dirs {
        if let Ok(entries) = std::fs::read_dir(&skills_dir) {
            let mut assistants = Vec::new();
            for entry in entries.flatten() {
                let path = entry.path();
                let stem = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                if stem.starts_with('_') || stem.starts_with('.') {
                    continue;
                }
                if path.extension().map_or(true, |ext| ext != "toml") {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(parsed) = toml::from_str::<SingleSkillConfig>(&content) {
                        assistants.push(parsed.assistant);
                    }
                }
            }
            return assistants;
        }
    }

    Vec::new()
}

fn resolve_allowed_tools(
    entry: &AssistantEntry,
    overrides: &HashMap<String, Vec<String>>,
    all_names: &HashSet<&str>,
    tool_descriptions: &[(String, String)],
) -> Vec<String> {
    overrides
        .get(&entry.id)
        .cloned()
        .or_else(|| match &entry.skills {
            Some(skills) if !skills.is_empty() => Some(
                skills
                    .iter()
                    .filter(|name| all_names.contains(name.as_str()))
                    .cloned()
                    .collect(),
            ),
            _ => Some(
                tool_descriptions
                    .iter()
                    .map(|(name, _)| name.clone())
                    .collect(),
            ),
        })
        .unwrap_or_else(|| {
            tool_descriptions
                .iter()
                .map(|(name, _)| name.clone())
                .collect()
        })
}

fn resolve_prompt_content(base: &Path, entry: &AssistantEntry) -> String {
    if let Some(prompt_text) = &entry.prompt_text {
        return prompt_text.clone();
    }
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
