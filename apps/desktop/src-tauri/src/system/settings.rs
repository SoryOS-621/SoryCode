//! Desktop preferences persisted as JSON next to the credentials.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub workspace: Option<String>,
}

pub fn load() -> Settings {
    let path = super::paths::settings_path();
    let Ok(data) = std::fs::read_to_string(path) else {
        return Settings::default();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save(settings: &Settings) -> Result<(), String> {
    let path = super::paths::settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create settings directory: {error}"))?;
    }
    let data = serde_json::to_string_pretty(settings)
        .map_err(|error| format!("cannot encode settings: {error}"))?;
    std::fs::write(path, data).map_err(|error| format!("cannot write settings: {error}"))
}

/// The workspace the user explicitly picked, if any.
///
/// `None` means "no project open": the UI shows an empty explorer and the
/// agent runs in an isolated scratch directory until the user selects,
/// clones or creates a project.
pub fn selected_workspace() -> Option<PathBuf> {
    load()
        .workspace
        .and_then(|path| super::paths::canonical_dir(&path).ok())
}
