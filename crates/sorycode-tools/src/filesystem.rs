use super::Tool;
use async_trait::async_trait;
use ignore::WalkBuilder;
use serde_json::Value;
use sorycode_core::tool::{ToolContext, ToolError, ToolOutput};
use std::path::{Path, PathBuf};

/// Resolve `p` against the tool working dir. Relative paths are joined;
/// absolute paths are kept as-is. The result is normalized (no `.`/`..`
/// components resolution against the filesystem, which is fine for tools).
pub fn resolve(base: &str, p: &str) -> PathBuf {
    let path = Path::new(p);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(base).join(path)
    }
}

/// Resolve a path that must stay inside `base` (working_dir). Used for
/// mutating tools so the agent cannot write outside the project.
pub fn resolve_confined(base: &str, p: &str) -> Result<PathBuf, ToolError> {
    let base_path = Path::new(base);
    let full = resolve(base, p);
    // Normalize lexically: resolve `.` and `..` without touching the fs so
    // non-existent paths (writes) are still checked.
    let mut normalized = PathBuf::new();
    for comp in full.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() {
                    return Err(ToolError::PermissionDenied(format!(
                        "path escapes working dir: {}",
                        p
                    )));
                }
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    let base_norm: PathBuf = base_path
        .components()
        .filter(|c| !matches!(c, std::path::Component::CurDir))
        .collect();
    if !normalized.starts_with(&base_norm) {
        return Err(ToolError::PermissionDenied(format!(
            "path escapes working dir: {}",
            p
        )));
    }
    Ok(normalized)
}

fn str_arg<'a>(input: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    input
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::InvalidInput(format!("{} required", key)))
}

fn opt_usize(input: &Value, key: &str) -> Option<usize> {
    input.get(key).and_then(|v| v.as_u64()).map(|n| n as usize)
}

pub struct ReadFileTool;
#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }
    fn description(&self) -> &'static str {
        "Read a file. Optional offset/limit select a 1-based line range (e.g. offset=10, limit=50)."
    }
    async fn execute(&self, input: Value, context: ToolContext) -> Result<ToolOutput, ToolError> {
        let path = str_arg(&input, "path")?;
        let full = resolve(&context.working_dir, path);
        let content = match tokio::fs::read_to_string(&full).await {
            Ok(c) => c,
            Err(e) => return Ok(ToolOutput::err(format!("read failed: {}", e))),
        };
        let offset = opt_usize(&input, "offset").unwrap_or(1).max(1);
        let limit = opt_usize(&input, "limit");
        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();
        let start = (offset - 1).min(total);
        let end = match limit {
            Some(l) => (start + l).min(total),
            None => total,
        };
        let mut out = String::new();
        for (i, line) in lines[start..end].iter().enumerate() {
            out.push_str(&format!("{:>6}\t{}\n", start + i + 1, line));
        }
        if start > 0 || end < total {
            out.push_str(&format!("\n[lines {}-{} of {}]", start + 1, end, total));
        }
        Ok(ToolOutput::ok(out))
    }
}

pub struct WriteFileTool;
#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &'static str {
        "write_file"
    }
    fn description(&self) -> &'static str {
        "Write content to a file, creating parent directories. Confined to the working dir."
    }
    async fn execute(&self, input: Value, context: ToolContext) -> Result<ToolOutput, ToolError> {
        let path = str_arg(&input, "path")?;
        let content = input.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let full = resolve_confined(&context.working_dir, path)?;
        if let Some(parent) = full.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ToolError::Io(e.to_string()))?;
        }
        match tokio::fs::write(&full, content).await {
            Ok(_) => Ok(ToolOutput::ok(format!("wrote {}", path))),
            Err(e) => Ok(ToolOutput::err(format!("write failed: {}", e))),
        }
    }
}

pub struct EditFileTool;
#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &'static str {
        "edit_file"
    }
    fn description(&self) -> &'static str {
        "Replace `old_string` with `new_string` in a file. Fails if old_string is missing or ambiguous (use replace_all=true to replace every occurrence)."
    }
    async fn execute(&self, input: Value, context: ToolContext) -> Result<ToolOutput, ToolError> {
        let path = str_arg(&input, "path")?;
        let old = str_arg(&input, "old_string")?;
        let new = input
            .get("new_string")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let replace_all = input
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let full = resolve_confined(&context.working_dir, path)?;
        let content = tokio::fs::read_to_string(&full)
            .await
            .map_err(|e| ToolError::Io(format!("read failed: {}", e)))?;
        let count = content.matches(old).count();
        if count == 0 {
            return Ok(ToolOutput::err("old_string not found in file"));
        }
        if count > 1 && !replace_all {
            return Ok(ToolOutput::err(format!(
                "old_string found {} times; set replace_all=true or provide more context",
                count
            )));
        }
        let updated = if replace_all {
            content.replace(old, new)
        } else {
            content.replacen(old, new, 1)
        };
        tokio::fs::write(&full, updated)
            .await
            .map_err(|e| ToolError::Io(format!("write failed: {}", e)))?;
        Ok(ToolOutput::ok(format!(
            "edited {} ({} replacement{})",
            path,
            if replace_all { count } else { 1 },
            if count > 1 { "s" } else { "" }
        )))
    }
}

pub struct ListFilesTool;
#[async_trait]
impl Tool for ListFilesTool {
    fn name(&self) -> &'static str {
        "list_files"
    }
    fn description(&self) -> &'static str {
        "Recursively list files under a directory. Respects .gitignore and skips hidden files. Params: path, max_depth (default 4), limit (default 500)."
    }
    async fn execute(&self, input: Value, context: ToolContext) -> Result<ToolOutput, ToolError> {
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let max_depth = opt_usize(&input, "max_depth").unwrap_or(4);
        let limit = opt_usize(&input, "limit").unwrap_or(500);
        let full = resolve_confined(&context.working_dir, path)?;
        if !full.is_dir() {
            return Ok(ToolOutput::err(format!("not a directory: {}", path)));
        }
        let mut out = Vec::new();
        let walker = WalkBuilder::new(&full)
            .max_depth(Some(max_depth))
            .standard_filters(true)
            .build();
        for entry in walker {
            let Ok(entry) = entry else { continue };
            if entry.depth() == 0 {
                continue;
            }
            let p = entry.path();
            let rel = p.strip_prefix(&full).unwrap_or(p).display().to_string();
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                out.push(format!("{}/", rel));
            } else {
                out.push(rel);
            }
            if out.len() >= limit {
                break;
            }
        }
        out.sort();
        if out.len() >= limit {
            out.push(format!("[truncated at {} entries]", limit));
        }
        Ok(ToolOutput::ok(out.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confined_allows_inside_paths() {
        assert!(resolve_confined("/proj", "src/main.rs").is_ok());
        assert!(resolve_confined("/proj", "./src/../src/main.rs").is_ok());
    }

    #[test]
    fn confined_blocks_escape() {
        assert!(resolve_confined("/proj", "../etc/passwd").is_err());
        assert!(resolve_confined("/proj", "src/../../etc/passwd").is_err());
        assert!(resolve_confined("/proj", "/etc/passwd").is_err());
    }
}
