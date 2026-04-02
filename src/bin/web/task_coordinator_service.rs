use std::sync::Arc;

use axum::{body::Body, http::StatusCode, response::Response};
use bytes::Bytes;
use futures_util::stream;
use tokio::sync::mpsc;

use super::session_store::{save_session_to_disk, WebSessionScope};
use super::task_repository;
use super::task_service::{status_label, TaskStatus};
use super::{
    emit_event, get_or_create_vector_for_assistant, resolve_allowed_tools_for_scope, AppState,
    WorkspaceEvent, DEFAULT_MAX_TURNS,
};
use bee::agent::{create_context_with_long_term_for_assistant, process_message_stream};
use bee::react::ReactEvent;

const COORDINATOR_INSTRUCTION: &str = "\n\n你是指定任务的统筹负责人。请使用 list_agents 查看可用 agent，使用 create 创建 specialized 子 agent，使用 create_group 组建团队，使用 send 分配职责和发起协作。完成后简要总结。";

pub async fn start_task(
    state: Arc<AppState>,
    task_id: String,
) -> Result<Response, (StatusCode, String)> {
    let task = task_repository::get_task(&state.workspace, state.task_persistence, &task_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "task not found".to_string()))?;
    let coordinator_id = task
        .coordinator_id
        .as_ref()
        .filter(|id| !id.is_empty())
        .cloned()
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "task has no coordinator_id, please assign one first".to_string(),
            )
        })?;

    let prompt = state
        .assistant_prompts
        .read()
        .await
        .get(&coordinator_id)
        .cloned();
    let base_prompt = prompt.as_deref().unwrap_or("");
    let system_prompt = format!("{}{}", base_prompt, COORDINATOR_INSTRUCTION);
    let description = task.description.as_deref().unwrap_or("无");
    let user_message = format!(
        "请统筹以下任务：\n\n【任务标题】{}\n【任务描述】{}\n\n请分析任务、创建或调用 agent、组队、分配职责、建立协作流程。",
        task.title, description
    );
    let session_key = format!("task_coord_{}", task_id);
    let scope = WebSessionScope {
        tenant_id: Some("tenant-default".to_string()),
        organization_id: Some("org-default".to_string()),
        team_id: task
            .group_id
            .clone()
            .or_else(|| Some(format!("task_{}", task_id))),
        agent_instance_id: Some(coordinator_id.clone()),
        user_id: Some(session_key.clone()),
    };
    let vector = get_or_create_vector_for_assistant(&state, &coordinator_id).await;
    let mut context = {
        let mut sessions = state.sessions.write().await;
        sessions.remove(&session_key).unwrap_or_else(|| {
            create_context_with_long_term_for_assistant(
                &state.config,
                DEFAULT_MAX_TURNS,
                Some(&state.workspace),
                vector,
                Some(&coordinator_id),
            )
        })
    };
    let allowed = resolve_allowed_tools_for_scope(&state, &coordinator_id, &scope).await;
    let components = state.components.read().await.clone();
    let (event_tx, event_rx) = mpsc::unbounded_channel::<ReactEvent>();
    let state_spawn = Arc::clone(&state);
    let task_id_clone = task_id.clone();
    let coordinator_id_clone = coordinator_id.clone();
    let scope_clone = scope.clone();
    let task_persistence = state.task_persistence;
    tokio::spawn(async move {
        let _ = process_message_stream(
            components.as_ref(),
            &mut context,
            &user_message,
            event_tx,
            Some(system_prompt.as_str()),
            None,
            Some(allowed.as_slice()),
            Some(&coordinator_id_clone),
        )
        .await;
        save_session_to_disk(
            &state_spawn.sessions_dir,
            &state_spawn.workspace,
            &format!("task_coord_{}", task_id_clone),
            &coordinator_id_clone,
            &context,
            Some(&scope_clone),
        );
        if let Ok(Some(updated)) = task_repository::patch_task(
            &state_spawn.workspace,
            task_persistence,
            &task_id_clone,
            |task| {
                task.status = TaskStatus::InProgress;
                task.updated_at = chrono::Utc::now().to_rfc3339();
            },
        ) {
            emit_event(
                &state_spawn.event_bus,
                WorkspaceEvent::TaskUpdated {
                    id: updated.id.clone(),
                    status: status_label(TaskStatus::InProgress).to_string(),
                },
            );
        }
    });

    let first_line = format!(
        "{}\n",
        serde_json::to_string(&serde_json::json!({
            "type": "session_id",
            "session_id": format!("task_{}", task_id)
        }))
        .unwrap()
    );
    let second_line = format!(
        "{}\n",
        serde_json::to_string(&serde_json::json!({
            "type": "coordinator_start",
            "task_id": task_id,
            "coordinator_id": coordinator_id
        }))
        .unwrap()
    );
    let pending = vec![first_line, second_line];
    let stream = stream::unfold(
        (event_rx, pending),
        move |(mut event_rx, mut pending)| async move {
            if !pending.is_empty() {
                let line = pending.remove(0);
                return Some((
                    Ok::<_, std::convert::Infallible>(Bytes::from(line)),
                    (event_rx, pending),
                ));
            }
            match event_rx.recv().await {
                Some(event) => {
                    let line = format!("{}\n", serde_json::to_string(&event).unwrap());
                    Some((
                        Ok::<_, std::convert::Infallible>(Bytes::from(line)),
                        (event_rx, vec![]),
                    ))
                }
                None => None,
            }
        },
    );
    let mut response = Response::new(Body::from_stream(stream));
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        "application/x-ndjson; charset=utf-8".parse().unwrap(),
    );
    Ok(response)
}
