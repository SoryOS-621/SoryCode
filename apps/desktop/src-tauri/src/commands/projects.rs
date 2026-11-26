//! Project commands.

use sorycode_core::project::Project;
use sorycode_protocol::{CoreCommand, CoreResponse};
use sorycode_storage::ProjectRepository;
use tauri::State;

use crate::commands::parse_project_id;
use crate::ipc::ProjectDto;
use crate::state::EngineState;

#[tauri::command]
pub async fn list_projects(state: State<'_, EngineState>) -> Result<Vec<ProjectDto>, String> {
    let storage = state.storage().await;
    let projects = ProjectRepository::list(&*storage)
        .await
        .map_err(|error| error.to_string())?;
    Ok(projects.into_iter().map(ProjectDto::from).collect())
}

#[tauri::command]
pub async fn create_project(
    state: State<'_, EngineState>,
    name: String,
    root_path: String,
) -> Result<ProjectDto, String> {
    let app = state.app().await;
    match app
        .handle(CoreCommand::CreateProject { name, root_path })
        .await
    {
        CoreResponse::ProjectCreated { project_id } => {
            let storage = state.storage().await;
            let project: Project = ProjectRepository::get(&*storage, project_id)
                .await
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "project was not persisted".to_string())?;
            Ok(project.into())
        }
        CoreResponse::Error { message } => Err(message),
        other => Err(format!("unexpected core response: {other:?}")),
    }
}

#[tauri::command]
pub async fn open_project(
    state: State<'_, EngineState>,
    project_id: String,
) -> Result<(), String> {
    let project_id = parse_project_id(&project_id)?;
    let app = state.app().await;
    match app.handle(CoreCommand::OpenProject { project_id }).await {
        CoreResponse::Ok => {}
        CoreResponse::Error { message } => return Err(message),
        other => return Err(format!("unexpected core response: {other:?}")),
    }
    // Opening a project must also switch the active workspace, otherwise the
    // file explorer and agent keep working on the previous (or scratch) dir.
    let storage = state.storage().await;
    let project = ProjectRepository::get(&*storage, project_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "project not found".to_string())?;
    state.set_workspace(&project.root_path).await?;
    Ok(())
}
