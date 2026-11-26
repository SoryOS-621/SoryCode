//! Filesystem locations owned by the desktop shell.

use std::path::PathBuf;

/// User home directory, falling back to the current directory.
pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// `~/.sorycode`, the per-user configuration directory.
pub fn sorycode_dir() -> PathBuf {
    home().join(".sorycode")
}

/// Provider credentials file.
pub fn credentials_path() -> PathBuf {
    sorycode_dir().join("credentials.json")
}

/// Desktop preferences (selected provider/model/workspace).
pub fn settings_path() -> PathBuf {
    sorycode_dir().join("settings.json")
}

/// Persistent engine data (sessions, tasks, messages).
pub fn data_path() -> PathBuf {
    sorycode_dir().join("desktop").join("state.json")
}

/// Isolated working directory used when no project is open.
///
/// NOT `current_dir()` or `home()`: the app is often launched from its own
/// source tree (`tauri dev`, repo root), and the user's home contains
/// unrelated personal files. An empty scratch dir keeps agent tools
/// confined to nothing until the user selects a real project.
pub fn scratch_dir() -> PathBuf {
    sorycode_dir().join("scratch")
}

/// Canonicalize a directory and verify that it exists.
pub fn canonical_dir(path: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(path);
    let canonical = candidate
        .canonicalize()
        .map_err(|error| format!("cannot open workspace {path}: {error}"))?;
    if !canonical.is_dir() {
        return Err(format!("{path} is not a directory"));
    }
    Ok(canonical)
}
