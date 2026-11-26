//! Provider discovery, selection and credential commands.

use sorycode_providers::{credentials, PROVIDER_PRESETS};
use sorycode_storage::SessionRepository;
use tauri::State;

use crate::ipc::{BootstrapDto, ProviderDto, SelectionDto};
use crate::state::EngineState;

#[tauri::command]
pub async fn list_providers() -> Vec<ProviderDto> {
    PROVIDER_PRESETS
        .iter()
        .map(|preset| {
            ProviderDto::from_preset(preset, credentials::resolve(preset.name).is_some())
        })
        .collect()
}

#[tauri::command]
pub async fn bootstrap(state: State<'_, EngineState>) -> Result<BootstrapDto, String> {
    let providers = list_providers().await;
    let selection = state.selection().await;
    let workspace = state.workspace().await;
    let storage = state.storage().await;
    let mut sessions = SessionRepository::list(&*storage, None)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(Into::into)
        .collect::<Vec<crate::ipc::SessionDto>>();
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(BootstrapDto {
        providers,
        selection,
        workspace,
        sessions,
    })
}

#[tauri::command]
pub async fn get_selection(state: State<'_, EngineState>) -> Result<SelectionDto, String> {
    Ok(state.selection().await)
}

#[tauri::command]
pub async fn select_provider(
    state: State<'_, EngineState>,
    provider: String,
) -> Result<SelectionDto, String> {
    state.select_provider(&provider).await
}

#[tauri::command]
pub async fn select_model(
    state: State<'_, EngineState>,
    model: String,
) -> Result<SelectionDto, String> {
    state.select_model(&model).await
}

#[tauri::command]
pub async fn connect_provider(
    state: State<'_, EngineState>,
    provider: String,
    api_key: String,
) -> Result<SelectionDto, String> {
    state.connect_provider(&provider, &api_key).await
}
