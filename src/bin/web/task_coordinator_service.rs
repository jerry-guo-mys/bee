use std::sync::Arc;

use axum::{body::Body, http::StatusCode, response::Response};
use bytes::Bytes;
use futures_util::stream;
use tokio::sync::mpsc;

use super::session_store::save_session_to_disk;
use super::task_service::{load_tasks, save_tasks, status_label, TaskStatus};
use super::{
    emit_event, get_or_create_vector_for_assistant, AppState, WorkspaceEvent, DEFAULT_MAX_TURNS,
};
use bee::agent::{create_context_with_long_term_for_assistant, process_message_stream};
use bee::react::ReactEvent;

const COORDINATOR_INSTRUCTION: &str = "\n\n你是指定任务的统筹负责人。请使用 list_agents 查看可用 agent，使用 create 创建 specialized 子 agent，使用 create_group 组建团队，使用 send 分配职责和发起协作。完成后简要总结。";

pub async fn start_task(
    state: Arc<AppState>,
    task_id: String,
) -> Result<Response, (StatusCode, String)> {
    let tasks = load_tasks(&state.workspace);
    let task = tasks
        .iter()
        .find(|task| task.id == task_id)
        .cloned()
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
    let allowed = state
        .assistant_skills
        .read()
        .await
        .get(&coordinator_id)
        .cloned();
    let components = state.components.read().await.clone();
    let (event_tx, event_rx) = mpsc::unbounded_channel::<ReactEvent>();
    let state_spawn = Arc::clone(&state);
    let task_id_clone = task_id.clone();
    let coordinator_id_clone = coordinator_id.clone();
    tokio::spawn(async move {
        let _ = process_message_stream(
            components.as_ref(),
            &mut context,
            &user_message,
            event_tx,
            Some(system_prompt.as_str()),
            None,
            allowed.as_deref(),
            Some(&coordinator_id_clone),
        )
        .await;
        save_session_to_disk(
            &state_spawn.sessions_dir,
            &state_spawn.workspace,
            &format!("task_coord_{}", task_id_clone),
            &coordinator_id_clone,
            &context,
        );
        let mut tasks = load_tasks(&state_spawn.workspace);
        let updated = tasks
            .iter_mut()
            .find(|task| task.id == task_id_clone)
            .map(|task| {
                task.status = TaskStatus::InProgress;
                task.updated_at = chrono::Utc::now().to_rfc3339();
                task.id.clone()
            });
        if let Some(id) = updated {
            save_tasks(&state_spawn.workspace, &tasks);
            emit_event(
                &state_spawn.event_bus,
                WorkspaceEvent::TaskUpdated {
                    id,
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
