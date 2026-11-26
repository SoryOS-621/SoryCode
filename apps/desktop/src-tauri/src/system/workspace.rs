//! Read-only workspace inspection used by the desktop UI.
//!
//! Every path is canonicalized and confined to the workspace root so a
//! compromised renderer cannot escape the project directory.

use crate::ipc::FileEntryDto;
use std::path::{Path, PathBuf};

const MAX_DEPTH: usize = 4;
const MAX_ENTRIES: usize = 2_000;
const MAX_FILE_BYTES: u64 = 256 * 1024;

const IGNORED: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    ".sorycode",
    ".next",
    "__pycache__",
];

fn confined(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("invalid workspace: {error}"))?;
    let candidate = root.join(relative);
    let canonical = candidate
        .canonicalize()
        .map_err(|error| format!("cannot access {relative}: {error}"))?;
    if !canonical.starts_with(&root) {
        return Err("path escapes the workspace".into());
    }
    Ok(canonical)
}

/// List files below `relative` (workspace-relative), depth-first.
pub fn list_files(root: &Path, relative: &str) -> Result<Vec<FileEntryDto>, String> {
    let start = confined(root, relative)?;
    let root = root
        .canonicalize()
        .map_err(|error| format!("invalid workspace: {error}"))?;
    let mut entries = Vec::new();
    walk(&root, &start, 0, &mut entries)?;
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.path.cmp(&b.path)));
    Ok(entries)
}

fn walk(
    root: &Path,
    directory: &Path,
    depth: usize,
    entries: &mut Vec<FileEntryDto>,
) -> Result<(), String> {
    if depth > MAX_DEPTH || entries.len() >= MAX_ENTRIES {
        return Ok(());
    }
    let read = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
    for item in read.flatten() {
        if entries.len() >= MAX_ENTRIES {
            break;
        }
        let path = item.path();
        let name = item.file_name().to_string_lossy().to_string();
        if IGNORED.contains(&name.as_str()) {
            continue;
        }
        let Ok(metadata) = item.metadata() else { continue };
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        if metadata.is_dir() {
            entries.push(FileEntryDto {
                path: relative,
                name,
                is_dir: true,
            });
            walk(root, &path, depth + 1, entries)?;
        } else if metadata.is_file() {
            entries.push(FileEntryDto {
                path: relative,
                name,
                is_dir: false,
            });
        }
    }
    Ok(())
}

/// Read a UTF-8 text file from the workspace.
pub fn read_file(root: &Path, relative: &str) -> Result<String, String> {
    let path = confined(root, relative)?;
    let metadata = std::fs::metadata(&path).map_err(|error| format!("cannot stat file: {error}"))?;
    if !metadata.is_file() {
        return Err("not a file".into());
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(format!(
            "file is too large ({} bytes, limit {MAX_FILE_BYTES})",
            metadata.len()
        ));
    }
    std::fs::read_to_string(path).map_err(|error| format!("cannot read file: {error}"))
}

/// Current git branch of the workspace, if it is a git repository.
pub fn git_branch(root: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() { None } else { Some(branch) }
}
