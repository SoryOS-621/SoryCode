//! Chat / task execution commands.

use sorycode_core::task::TaskStatus;
use sorycode_protocol::{CoreCommand, CoreResponse};
use sorycode_storage::TaskRepository;
use tauri::State;

use crate::commands::{parse_session_id, parse_task_id};
use crate::ipc::TaskDto;
use crate::state::EngineState;

#[tauri::command]
pub async fn send_message(
    state: State<'_, EngineState>,
    session_id: String,
    content: String,
) -> Result<String, String> {
    let session_id = parse_session_id(&session_id)?;
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err("message cannot be empty".into());
    }
    let app = state.app().await;
    match app
        .handle(CoreCommand::SendMessage {
            session_id,
            content,
        })
        .await
    {
        CoreResponse::TaskStarted { task_id } => Ok(task_id.to_string()),
        CoreResponse::Error { message } => Err(message),
        other => Err(format!("unexpected core response: {other:?}")),
    }
}

#[tauri::command]
pub async fn cancel_task(
    state: State<'_, EngineState>,
    task_id: String,
) -> Result<(), String> {
    let task_id = parse_task_id(&task_id)?;
    let app = state.app().await;
    match app.handle(CoreCommand::CancelTask { task_id }).await {
        CoreResponse::Ok => Ok(()),
        CoreResponse::Error { message } => Err(message),
        other => Err(format!("unexpected core response: {other:?}")),
    }
}

#[tauri::command]
pub async fn get_task_status(
    state: State<'_, EngineState>,
    task_id: String,
) -> Result<TaskDto, String> {
    let task_id = parse_task_id(&task_id)?;
    let storage = state.storage().await;
    let task = TaskRepository::get(&*storage, task_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "task not found".to_string())?;
    Ok(task.into())
}

/// Resolve a pending approval request emitted on `sorycode://approval`.
#[tauri::command]
pub async fn respond_approval(
    state: State<'_, EngineState>,
    approval_id: String,
    approved: bool,
) -> Result<(), String> {
    state.approvals().respond(&approval_id, approved)
}

/// Lightweight poll used by the UI when an event stream is unavailable.
#[tauri::command]
pub async fn get_task_state(
    state: State<'_, EngineState>,
    task_id: String,
) -> Result<TaskStatus, String> {
    Ok(get_task_status(state, task_id).await?.status)
}
