//! SoryCode native desktop shell.
//!
//! Responsibilities: window lifecycle, native capabilities, IPC and event
//! forwarding. Chat/code/work features live in `apps/app`; visual components
//! live in `apps/ui`; the agent engine lives in `crates/`.

mod approvals;
mod commands;
mod events;
mod ipc;
mod state;
mod system;

use sorycode_events::EventSink;
use std::sync::Arc;
use tauri::Manager;

use events::TauriSink;
use state::EngineState;

pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let sink: Arc<dyn EventSink> = Arc::new(TauriSink::new(app.handle().clone()));
            let approvals = Arc::new(approvals::TauriApproval::new(app.handle().clone()));
            let state = tauri::async_runtime::block_on(EngineState::new(sink, approvals))
                .map_err(std::io::Error::other)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::providers::list_providers,
            commands::providers::bootstrap,
            commands::providers::get_selection,
            commands::providers::select_provider,
            commands::providers::select_model,
            commands::providers::connect_provider,
            commands::sessions::create_session,
            commands::sessions::list_sessions,
            commands::sessions::get_history,
            commands::sessions::clear_history,
            commands::sessions::list_tasks,
            commands::chat::send_message,
            commands::chat::cancel_task,
            commands::chat::get_task_status,
            commands::chat::get_task_state,
            commands::chat::respond_approval,
            commands::projects::list_projects,
            commands::projects::create_project,
            commands::projects::open_project,
            commands::system::get_workspace,
            commands::system::open_workspace,
            commands::system::pick_workspace_folder,
            commands::system::list_workspace_files,
            commands::system::read_workspace_file,
            commands::system::get_app_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the SoryCode desktop application");
}
