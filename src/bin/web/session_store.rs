use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bee::config::AppConfig;
use bee::memory::InMemoryVectorLongTerm;
use bee::memory::{
    append_daily_log, assistant_memory_root, lessons_path, preferences_path, procedural_path,
    ConversationMemory, Message,
};
use bee::react::ContextManager;
use tokio::sync::RwLock;

use super::assistant_catalog::AssistantInfo;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SessionSnapshot {
    pub messages: Vec<Message>,
    pub max_turns: usize,
    #[serde(default)]
    pub scope: Option<WebSessionScope>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebSessionScope {
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub organization_id: Option<String>,
    #[serde(default)]
    pub team_id: Option<String>,
    #[serde(default)]
    pub agent_instance_id: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
}

impl WebSessionScope {
    pub fn key_suffix(&self) -> String {
        [
            self.tenant_id.as_deref().unwrap_or("tenant-default"),
            self.organization_id.as_deref().unwrap_or("org-default"),
            self.team_id.as_deref().unwrap_or("team-default"),
            self.agent_instance_id.as_deref().unwrap_or("agent-default"),
            self.user_id.as_deref().unwrap_or("user-default"),
        ]
        .into_iter()
        .map(sanitize_scope_segment)
        .collect::<Vec<_>>()
        .join("--")
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assistant_id: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GroupChatSnapshot {
    messages: Vec<GroupChatMessage>,
    max_turns: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupInfo {
    pub id: String,
    pub name: Option<String>,
    pub member_ids: Vec<String>,
    pub created_at: String,
}

pub fn session_key(
    session_id: &str,
    assistant_id: &str,
    scope: Option<&WebSessionScope>,
) -> String {
    match scope {
        Some(scope) => format!("{}::{}::{}", session_id, assistant_id, scope.key_suffix()),
        None => format!("{}::{}", session_id, assistant_id),
    }
}

pub fn group_session_path(sessions_dir: &Path, group_id: &str) -> PathBuf {
    let safe_gid = group_id.replace('/', "_").replace('\\', "_");
    sessions_dir.join(format!("group_{}.json", safe_gid))
}

pub fn session_path(
    sessions_dir: &Path,
    session_id: &str,
    assistant_id: &str,
    scope: Option<&WebSessionScope>,
) -> PathBuf {
    let safe_sid = session_id.replace('/', "_").replace('\\', "_");
    let safe_aid = assistant_id.replace('/', "_").replace('\\', "_");
    let aid = if safe_aid.is_empty() {
        "default"
    } else {
        safe_aid.as_str()
    };
    let suffix = scope
        .map(|scope| format!("---{}", scope.key_suffix()))
        .unwrap_or_default();
    sessions_dir.join(format!("{}---{}{}.json", safe_sid, aid, suffix))
}

pub fn load_groups_from_disk(path: &Path) -> Arc<RwLock<HashMap<String, GroupInfo>>> {
    let map: HashMap<String, GroupInfo> = std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default();
    Arc::new(RwLock::new(map))
}

pub fn save_groups_to_disk(path: &Path, groups: &HashMap<String, GroupInfo>) {
    if let Ok(json) = serde_json::to_string_pretty(groups) {
        let _ = std::fs::write(path, json);
    }
}

pub fn load_group_session(sessions_dir: &Path, group_id: &str) -> Vec<GroupChatMessage> {
    let path = group_session_path(sessions_dir, group_id);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_json::from_str::<GroupChatSnapshot>(&content).ok())
        .map(|snapshot| snapshot.messages)
        .unwrap_or_default()
}

pub fn save_group_session(
    sessions_dir: &Path,
    group_id: &str,
    messages: &[GroupChatMessage],
    max_turns: usize,
) {
    let path = group_session_path(sessions_dir, group_id);
    let snapshot = GroupChatSnapshot {
        messages: messages.to_vec(),
        max_turns,
    };
    if let Ok(json) = serde_json::to_string_pretty(&snapshot) {
        let _ = std::fs::write(path, json);
    }
}

pub fn group_messages_to_llm_messages(
    messages: &[GroupChatMessage],
    assistants: &[AssistantInfo],
) -> Vec<Message> {
    messages
        .iter()
        .map(|message| match message.role.as_str() {
            "user" => Message::user(&message.content),
            "assistant" => {
                let label = message
                    .assistant_id
                    .as_ref()
                    .and_then(|id| assistants.iter().find(|assistant| assistant.id == *id))
                    .map(|assistant| assistant.name.as_str())
                    .unwrap_or("Assistant");
                Message::assistant(format!("{}: {}", label, message.content))
            }
            _ => Message::assistant(&message.content),
        })
        .collect()
}

pub fn load_session_from_disk(
    sessions_dir: &Path,
    session_id: &str,
    assistant_id: &str,
    workspace: &Path,
    cfg: &AppConfig,
    vector_for_assistant: Option<Arc<InMemoryVectorLongTerm>>,
    requested_scope: Option<&WebSessionScope>,
) -> Option<ContextManager> {
    let path = session_path(sessions_dir, session_id, assistant_id, requested_scope);
    let data = std::fs::read_to_string(&path).ok().or_else(|| {
        if assistant_id == "default" && requested_scope.is_none() {
            let legacy_path =
                sessions_dir.join(format!("{}.json", session_id.replace(['/', '\\'], "_")));
            std::fs::read_to_string(&legacy_path).ok()
        } else {
            None
        }
    })?;
    let snapshot: SessionSnapshot = serde_json::from_str(&data).ok()?;
    let conversation = ConversationMemory::from_messages(snapshot.messages, snapshot.max_turns);
    let assistant_root =
        scoped_assistant_memory_root(workspace, assistant_id, snapshot.scope.as_ref());
    std::fs::create_dir_all(&assistant_root).ok();
    let long_term: Arc<dyn bee::memory::LongTermMemory> = if let Some(vector) = vector_for_assistant
    {
        vector
    } else {
        Arc::new(bee::memory::FileLongTerm::new(
            bee::memory::long_term_path(&assistant_root),
            2000,
        ))
    };
    let mut context = ContextManager::new(snapshot.max_turns)
        .with_long_term(long_term)
        .with_lessons_path(lessons_path(&assistant_root))
        .with_procedural_path(procedural_path(&assistant_root))
        .with_preferences_path(preferences_path(&assistant_root))
        .with_auto_lesson_on_hallucination(cfg.evolution.auto_lesson_on_hallucination)
        .with_record_tool_success(cfg.evolution.record_tool_success);
    context.conversation = conversation;
    Some(context)
}

pub fn save_session_to_disk(
    sessions_dir: &Path,
    workspace: &Path,
    session_id: &str,
    assistant_id: &str,
    context: &ContextManager,
    scope: Option<&WebSessionScope>,
) {
    let scope = scope
        .cloned()
        .unwrap_or_else(|| default_web_session_scope(session_id, assistant_id));
    let path = session_path(sessions_dir, session_id, assistant_id, Some(&scope));
    let snapshot = SessionSnapshot {
        messages: context.messages().to_vec(),
        max_turns: context.conversation.max_turns(),
        scope: Some(scope.clone()),
    };
    if let Ok(json) = serde_json::to_string_pretty(&snapshot) {
        let _ = std::fs::write(path, json);
    }
    let assistant_root = scoped_assistant_memory_root(workspace, assistant_id, Some(&scope));
    std::fs::create_dir_all(assistant_root.join("logs")).ok();
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let _ = append_daily_log(
        &assistant_root,
        &date,
        &format!("{}:{}", session_id, assistant_id),
        context.messages(),
    );
}

fn default_web_session_scope(session_id: &str, assistant_id: &str) -> WebSessionScope {
    WebSessionScope {
        tenant_id: Some("tenant-default".to_string()),
        organization_id: Some("org-default".to_string()),
        team_id: None,
        agent_instance_id: Some(assistant_id.to_string()),
        user_id: Some(session_id.to_string()),
    }
}

fn scoped_assistant_memory_root(
    workspace: &Path,
    assistant_id: &str,
    scope: Option<&WebSessionScope>,
) -> PathBuf {
    let Some(scope) = scope else {
        return assistant_memory_root(workspace, assistant_id);
    };

    let mut root = workspace.join(".bee").join("web_scopes");
    root.push(sanitize_scope_segment(
        scope.tenant_id.as_deref().unwrap_or("tenant-default"),
    ));
    root.push(sanitize_scope_segment(
        scope.organization_id.as_deref().unwrap_or("org-default"),
    ));
    if let Some(team_id) = scope.team_id.as_deref() {
        root.push("teams");
        root.push(sanitize_scope_segment(team_id));
    }
    if let Some(user_id) = scope.user_id.as_deref() {
        root.push("users");
        root.push(sanitize_scope_segment(user_id));
    }
    root.push("assistants");
    root.push(sanitize_scope_segment(assistant_id));
    root
}

fn sanitize_scope_segment(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "default".to_string()
    } else {
        sanitized
    }
}
