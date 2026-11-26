use super::Tool;
use async_trait::async_trait;
use serde_json::Value;
use sorycode_core::tool::{ToolContext, ToolError, ToolOutput};
use std::sync::OnceLock;

/// `github_search` — search GitHub repos/issues without auth via the public
/// REST API (rate-limited but keyless). Code search requires a token and is
/// intentionally not used.
pub struct GitHubSearchTool;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .user_agent("sorycode/0.1")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-_.~".contains(c) {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect()
}

#[async_trait]
impl Tool for GitHubSearchTool {
    fn name(&self) -> &'static str {
        "github_search"
    }
    fn description(&self) -> &'static str {
        "Search GitHub (no API key). Params: query* (e.g. \"tokio async runtime\" or \"repo:serde-rs/serde derive\"), kind (\"repos\"|\"issues\", default repos), limit (default 5, max 10). Returns names, URLs, descriptions/stars."
    }
    async fn execute(&self, input: Value, _context: ToolContext) -> Result<ToolOutput, ToolError> {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("query required".into()))?;
        let kind = input
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("repos");
        let limit = input
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| (n as usize).min(10))
            .unwrap_or(5);

        let url = match kind {
            "issues" => format!(
                "https://api.github.com/search/issues?q={}&per_page={}",
                urlencode(query),
                limit
            ),
            _ => format!(
                "https://api.github.com/search/repositories?q={}&per_page={}&sort=stars",
                urlencode(query),
                limit
            ),
        };

        let resp = client()
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        if resp.status() == reqwest::StatusCode::FORBIDDEN {
            return Ok(ToolOutput::err(
                "github rate limit hit (10 req/min unauthenticated) — try again shortly",
            ));
        }
        let json: Value = resp
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let items = json
            .get("items")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if items.is_empty() {
            return Ok(ToolOutput::err("no results found"));
        }

        let mut out = String::new();
        for (i, it) in items.iter().enumerate() {
            match kind {
                "issues" => {
                    let title = it.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let u = it.get("html_url").and_then(|v| v.as_str()).unwrap_or("");
                    let state = it.get("state").and_then(|v| v.as_str()).unwrap_or("");
                    let repo = it
                        .get("repository_url")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.rsplit('/').next())
                        .unwrap_or("");
                    out.push_str(&format!(
                        "{}. [{}] {} ({})\n   {}\n\n",
                        i + 1,
                        state,
                        title,
                        repo,
                        u
                    ));
                }
                _ => {
                    let name = it.get("full_name").and_then(|v| v.as_str()).unwrap_or("");
                    let u = it.get("html_url").and_then(|v| v.as_str()).unwrap_or("");
                    let desc = it.get("description").and_then(|v| v.as_str()).unwrap_or("");
                    let stars = it
                        .get("stargazers_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let lang = it.get("language").and_then(|v| v.as_str()).unwrap_or("");
                    out.push_str(&format!(
                        "{}. {} ⭐{} {}\n   {}\n   {}\n\n",
                        i + 1,
                        name,
                        stars,
                        lang,
                        u,
                        desc.chars().take(160).collect::<String>()
                    ));
                }
            }
        }
        Ok(ToolOutput::ok(out.trim_end().to_string()))
    }
}
