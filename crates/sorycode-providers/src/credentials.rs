//! Client-side credential storage for model providers.
//!
//! Credentials live outside the core storage backend so that they never end up
//! in session/task databases. Environment variables always take precedence over
//! the on-disk file.

use crate::PROVIDER_PRESETS;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Path of the credentials file: `~/.sorycode/credentials.json`.
pub fn path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home)
        .join(".sorycode")
        .join("credentials.json")
}

/// Load the provider -> API key map. Missing or malformed files yield an empty
/// map: credentials are optional for free providers.
pub fn load() -> BTreeMap<String, String> {
    let path = path();
    let Ok(data) = std::fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&data) else {
        return BTreeMap::new();
    };
    let mut map = BTreeMap::new();
    if let Some(object) = value.as_object() {
        for (key, value) in object {
            if let Some(secret) = value.as_str() {
                map.insert(key.clone(), secret.to_string());
            }
        }
    }
    map
}

/// Return the API key configured for a provider, if any.
pub fn get(provider: &str) -> Option<String> {
    load().remove(provider)
}

/// Persist an API key for a provider. The file is written with owner-only
/// permissions on Unix.
pub fn save(provider: &str, api_key: &str) -> std::io::Result<()> {
    let path = path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut map = load();
    map.insert(provider.to_string(), api_key.to_string());
    let data = serde_json::to_string_pretty(&map)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    std::fs::write(&path, data)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Remove a stored API key.
pub fn remove(provider: &str) -> std::io::Result<()> {
    let path = path();
    let mut map = load();
    map.remove(provider);
    let data = serde_json::to_string_pretty(&map)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    std::fs::write(&path, data)
}

/// Resolve a provider key from the environment first, then from disk.
pub fn resolve(provider: &str) -> Option<String> {
    let preset = PROVIDER_PRESETS.iter().find(|p| p.name == provider)?;
    if let Ok(value) = std::env::var(preset.env_key) {
        if !value.is_empty() {
            return Some(value);
        }
    }
    get(provider).filter(|value| !value.is_empty())
}

/// Copy stored credentials into the process environment without overriding
/// variables that are already set.
pub fn apply_to_env() {
    for (provider, secret) in load() {
        let Some(preset) = PROVIDER_PRESETS.iter().find(|p| p.name == provider) else {
            continue;
        };
        let already_set = std::env::var(preset.env_key)
            .map(|value| !value.is_empty())
            .unwrap_or(false);
        if !already_set {
            // SAFETY: called once during single-threaded CLI startup.
            unsafe { std::env::set_var(preset.env_key, secret) };
        }
    }
}
