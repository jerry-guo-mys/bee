use std::sync::Arc;

use axum::http::StatusCode;
use tokio::sync::mpsc;

use super::session_store::{
    group_messages_to_llm_messages, load_group_session, save_group_session, GroupChatMessage,
    WebSessionScope,
};
use super::{
    emit_event, get_or_create_vector_for_assistant, resolve_allowed_tools_for_scope, AppState,
    WorkspaceEvent, DEFAULT_MAX_TURNS,
};
use bee::agent::{create_context_with_long_term_for_assistant, process_message_stream};

pub async fn process_inbox(
    state: Arc<AppState>,
    assistant_id: &str,
) -> Result<usize, (StatusCode, String)> {
    let groups = state.groups.read().await;
    let p2p_groups: Vec<_> = groups
        .values()
        .filter(|group| {
            group.id.starts_with("p2p_") && group.member_ids.contains(&assistant_id.to_string())
        })
        .cloned()
        .collect();
    drop(groups);

    let mut processed = 0;
    for group in p2p_groups {
        let messages = load_group_session(&state.sessions_dir, &group.id);
        let last = match messages.last() {
            Some(message) => message,
            None => continue,
        };
        if last.role != "assistant" {
            continue;
        }
        let from = last.assistant_id.as_deref().unwrap_or("");
        if from == assistant_id {
            continue;
        }
        let from_name = state
            .assistants
            .iter()
            .find(|assistant| assistant.id == from)
            .map(|assistant| assistant.name.as_str())
            .unwrap_or(from);
        let user_input = format!("[来自 {}] {}", from_name, last.content);

        let vector = get_or_create_vector_for_assistant(&state, assistant_id).await;
        let mut context = create_context_with_long_term_for_assistant(
            &state.config,
            DEFAULT_MAX_TURNS,
            Some(&state.workspace),
            vector,
            Some(assistant_id),
        );
        let llm_history =
            group_messages_to_llm_messages(&messages[..messages.len() - 1], &state.assistants);
        context.set_messages(llm_history);

        let (tx, _rx) = mpsc::unbounded_channel();
        let components = state.components.read().await.clone();
        let prompt = state
            .assistant_prompts
            .read()
            .await
            .get(assistant_id)
            .cloned();
        let scope = WebSessionScope {
            tenant_id: Some("tenant-default".to_string()),
            organization_id: Some("org-default".to_string()),
            team_id: None,
            agent_instance_id: Some(assistant_id.to_string()),
            user_id: Some(group.id.clone()),
        };
        let allowed = resolve_allowed_tools_for_scope(&state, assistant_id, &scope).await;
        let reply = process_message_stream(
            components.as_ref(),
            &mut context,
            &user_input,
            tx,
            prompt.as_deref(),
            None,
            Some(allowed.as_slice()),
            Some(assistant_id),
        )
        .await
        .unwrap_or_else(|err| format!("Error: {}", err));

        let mut all_messages = messages.clone();
        all_messages.push(GroupChatMessage {
            role: "assistant".to_string(),
            content: reply.clone(),
            assistant_id: Some(assistant_id.to_string()),
        });
        save_group_session(
            &state.sessions_dir,
            &group.id,
            &all_messages,
            DEFAULT_MAX_TURNS,
        );

        let preview =
            reply.chars().take(80).collect::<String>() + if reply.len() > 80 { "…" } else { "" };
        emit_event(
            &state.event_bus,
            WorkspaceEvent::MessageCreated {
                group_id: group.id.clone(),
                from: Some(assistant_id.to_string()),
                to: Some(from.to_string()),
                content_preview: preview,
            },
        );
        processed += 1;
    }

    Ok(processed)
}
