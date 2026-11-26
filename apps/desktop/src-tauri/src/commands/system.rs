//! Native system capabilities exposed to the application layer.
//!
//! These commands are the only place where the desktop shell touches the local
//! filesystem directly. Business operations still go through the Core tools.

use tauri::{AppHandle, State};

use crate::ipc::{AppInfoDto, FileEntryDto, WorkspaceDto};
use crate::state::EngineState;
use crate::system::{paths, workspace};

/// Open the native folder picker and return the chosen directory, if any.
///
/// Pure shell capability: picking a folder does not switch the workspace —
/// the frontend feeds the result into `open_workspace` so a single code path
/// handles validation, persistence and engine rebuild.
#[tauri::command]
pub async fn pick_workspace_folder(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .blocking_pick_folder()
            .and_then(|path| path.as_path().map(|path| path.display().to_string()))
    })
    .await
    .map_err(|error| format!("folder picker failed: {error}"))?;
    Ok(picked)
}

#[tauri::command]
pub async fn get_workspace(
    state: State<'_, EngineState>,
) -> Result<Option<WorkspaceDto>, String> {
    Ok(state.workspace().await)
}

#[tauri::command]
pub async fn open_workspace(
    state: State<'_, EngineState>,
    path: String,
) -> Result<WorkspaceDto, String> {
    state.set_workspace(&path).await
}

#[tauri::command]
pub async fn list_workspace_files(
    state: State<'_, EngineState>,
    path: Option<String>,
) -> Result<Vec<FileEntryDto>, String> {
    let Some(root) = state.project_root().await else {
        return Ok(Vec::new());
    };
    workspace::list_files(&root, path.as_deref().unwrap_or(""))
}

#[tauri::command]
pub async fn read_workspace_file(
    state: State<'_, EngineState>,
    path: String,
) -> Result<String, String> {
    let root = state
        .project_root()
        .await
        .ok_or_else(|| "no workspace selected".to_string())?;
    workspace::read_file(&root, &path)
}

#[tauri::command]
pub async fn get_app_info() -> AppInfoDto {
    AppInfoDto {
        version: env!("CARGO_PKG_VERSION").to_string(),
        data_path: paths::data_path().to_string_lossy().to_string(),
        credentials_path: paths::credentials_path().to_string_lossy().to_string(),
    }
}
