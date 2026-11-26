//! Session, history and task commands.

use sorycode_core::ids::ProjectId;
use sorycode_protocol::{CoreCommand, CoreResponse};
use sorycode_storage::{MessageRepository, SessionRepository, TaskRepository};
use tauri::State;

use crate::commands::{parse_project_id, parse_session_id};
use crate::ipc::{MessageDto, SessionDto, TaskDto};
use crate::state::EngineState;

#[tauri::command]
pub async fn create_session(
    state: State<'_, EngineState>,
    title: String,
    project_id: Option<String>,
) -> Result<SessionDto, String> {
    let project: Option<ProjectId> = project_id
        .as_deref()
        .map(parse_project_id)
        .transpose()?;
    let app = state.app().await;
    match app
        .handle(CoreCommand::CreateSession {
            project_id: project,
            title,
        })
        .await
    {
        CoreResponse::SessionCreated { session_id } => {
            let storage = state.storage().await;
            let session = SessionRepository::get(&*storage, session_id)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "session was not persisted".to_string())?;
            Ok(session.into())
        }
        CoreResponse::Error { message } => Err(message),
        other => Err(format!("unexpected core response: {other:?}")),
    }
}

#[tauri::command]
pub async fn list_sessions(
    state: State<'_, EngineState>,
    project_id: Option<String>,
) -> Result<Vec<SessionDto>, String> {
    let project: Option<ProjectId> = project_id
        .as_deref()
        .map(parse_project_id)
        .transpose()?;
    let storage = state.storage().await;
    let mut sessions = SessionRepository::list(&*storage, project)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(SessionDto::from)
        .collect::<Vec<_>>();
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(sessions)
}

#[tauri::command]
pub async fn get_history(
    state: State<'_, EngineState>,
    session_id: String,
) -> Result<Vec<MessageDto>, String> {
    let session_id = parse_session_id(&session_id)?;
    let storage = state.storage().await;
    let messages = MessageRepository::list(&*storage, session_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(messages.into_iter().map(MessageDto::from).collect())
}

#[tauri::command]
pub async fn clear_history(
    state: State<'_, EngineState>,
    session_id: String,
) -> Result<(), String> {
    let session_id = parse_session_id(&session_id)?;
    let app = state.app().await;
    match app
        .handle(CoreCommand::ClearSessionHistory { session_id })
        .await
    {
        CoreResponse::Ok => Ok(()),
        CoreResponse::Error { message } => Err(message),
        other => Err(format!("unexpected core response: {other:?}")),
    }
}

#[tauri::command]
pub async fn list_tasks(
    state: State<'_, EngineState>,
    session_id: String,
) -> Result<Vec<TaskDto>, String> {
    let session_id = parse_session_id(&session_id)?;
    let storage = state.storage().await;
    let mut tasks = TaskRepository::list_by_session(&*storage, session_id)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(TaskDto::from)
        .collect::<Vec<_>>();
    tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(tasks)
}
