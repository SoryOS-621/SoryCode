use async_trait::async_trait;
use ignore::WalkBuilder;
use regex::Regex;
use sorycode_core::tool::{ToolContext, ToolError, ToolOutput};
use serde_json::Value;
use std::path::PathBuf;
use super::Tool;
use super::filesystem::resolve_confined;

pub struct SearchTool;
#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &'static str {
        "search"
    }
    fn description(&self) -> &'static str {
        "Search file contents with a regex (ripgrep-style). Respects .gitignore and skips hidden files. Params: pattern (required), path, glob (e.g. \"*.rs\"), limit (default 200)."
    }
    async fn execute(&self, input: Value, context: ToolContext) -> Result<ToolOutput, ToolError> {
        let pattern = input
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("pattern required".into()))?;
        let re = Regex::new(pattern).map_err(|e| ToolError::InvalidInput(e.to_string()))?;
        let root = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let glob = input.get("glob").and_then(|v| v.as_str());
        let limit = input
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(200);
        let full = resolve_confined(&context.working_dir, root)?;
        let base = PathBuf::from(&context.working_dir);

        let mut results = Vec::new();
        let mut builder = WalkBuilder::new(&full);
        builder.max_depth(Some(12)).standard_filters(true);
        if let Some(g) = glob {
            let mut globset = ignore::overrides::OverrideBuilder::new(&full);
            if globset.add(g).is_ok() {
                if let Ok(matcher) = globset.build() {
                    builder.overrides(matcher);
                }
            }
        }
        for entry in builder.build() {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                continue;
            }
            let path = entry.path();
            // skip likely-binary files by extension
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "ico" | "pdf" | "zip" | "gz" | "wasm" | "so" | "dylib" | "exe" | "dll" | "bin" | "mp3" | "mp4" | "wav" | "ogg" | "webm" | "ttf" | "otf" | "woff" | "woff2") {
                    continue;
                }
            }
            let Ok(content) = std::fs::read_to_string(path) else {
                continue;
            };
            for (i, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    let rel = path.strip_prefix(&base).unwrap_or(path).display().to_string();
                    results.push(format!("{}:{}: {}", rel, i + 1, line.trim()));
                    if results.len() >= limit {
                        break;
                    }
                }
            }
            if results.len() >= limit {
                break;
            }
        }
        if results.len() >= limit {
            results.push(format!("[truncated at {} matches]", limit));
        }
        Ok(ToolOutput::ok(results.join("\n")))
    }
}
