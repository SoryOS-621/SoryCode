use super::Tool;
use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;
use sorycode_core::tool::{ToolContext, ToolError, ToolOutput};
use std::sync::OnceLock;

/// `web_search` — keyless web search via DuckDuckGo (Instant Answer API +
/// lite HTML fallback). No API key required, like Codex's `web.run` which
/// rides on the ChatGPT backend; we approximate it with a public endpoint.
pub struct WebSearchTool;

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .user_agent("sorycode/0.1 (+https://sorycode.local)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

fn decode_html(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ")")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "web_search"
    }
    fn description(&self) -> &'static str {
        "Search the web (no API key needed). Param: query (required), limit (default 5, max 10). Returns titles, URLs and snippets."
    }
    async fn execute(&self, input: Value, _context: ToolContext) -> Result<ToolOutput, ToolError> {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("query required".into()))?;
        let limit = input
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| (n as usize).min(10))
            .unwrap_or(5);

        // 1) DuckDuckGo Instant Answer API (JSON, no key)
        let mut results: Vec<(String, String, String)> = Vec::new(); // (title, url, snippet)
        let api_url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
            urlencoding(query)
        );
        if let Ok(resp) = client().get(&api_url).send().await {
            if let Ok(json) = resp.json::<Value>().await {
                if let Some(abstract_text) = json.get("AbstractText").and_then(|v| v.as_str()) {
                    if !abstract_text.is_empty() {
                        let heading = json
                            .get("Heading")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Result");
                        let url = json
                            .get("AbstractURL")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        results.push((
                            heading.to_string(),
                            url.to_string(),
                            abstract_text.to_string(),
                        ));
                    }
                }
                // related topics (first level only)
                if let Some(topics) = json.get("RelatedTopics").and_then(|v| v.as_array()) {
                    for t in topics.iter().take(limit.saturating_sub(results.len())) {
                        let text = t.get("Text").and_then(|v| v.as_str());
                        let first_url = t.get("FirstURL").and_then(|v| v.as_str()).or_else(|| {
                            t.get("Topics")
                                .and_then(|v| v.as_array())
                                .and_then(|a| a.first())
                                .and_then(|x| x.get("FirstURL"))
                                .and_then(|v| v.as_str())
                        });
                        if let (Some(text), Some(url)) = (text, first_url) {
                            results.push((
                                text.split(" - ").next().unwrap_or(text).to_string(),
                                url.to_string(),
                                text.to_string(),
                            ));
                        }
                    }
                }
            }
        }

        // 2) Fallback: lite HTML endpoint if API gave nothing useful
        if results.len() < 2 {
            let lite = async {
                let resp = client()
                    .get(format!(
                        "https://lite.duckduckgo.com/lite/?q={}",
                        urlencoding(query)
                    ))
                    .send()
                    .await
                    .ok()?;
                resp.text().await.ok()
            }
            .await;
            if let Some(html) = lite {
                results.extend(parse_lite_results(&html, limit));
            }
        }

        results.truncate(limit);
        if results.is_empty() {
            return Ok(ToolOutput::err("no results found"));
        }
        let mut out = String::new();
        for (i, (title, url, snippet)) in results.iter().enumerate() {
            out.push_str(&format!(
                "{}. {}\n   {}\n   {}\n\n",
                i + 1,
                title,
                url,
                snippet
            ));
        }
        Ok(ToolOutput::ok(out.trim_end().to_string()))
    }
}

fn urlencoding(s: &str) -> String {
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

/// Parse duckduckgo lite HTML: links in <a rel="nofollow" href="..."> and
/// snippets in following <td class='result-snippet'>.
fn parse_lite_results(html: &str, limit: usize) -> Vec<(String, String, String)> {
    let link_re = Regex::new(r#"<a[^>]+class='result-link'[^>]+href='([^']+)'[^>]*>(.*?)</a>"#)
        .unwrap_or_else(|_| Regex::new("a").unwrap());
    let snip_re = Regex::new(r#"class='result-snippet'[^>]*>(.*?)</td>"#)
        .unwrap_or_else(|_| Regex::new("b").unwrap());
    let mut out = Vec::new();
    let links: Vec<(String, String)> = link_re
        .captures_iter(html)
        .take(limit)
        .map(|c| {
            let url = c.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
            let title = decode_html(
                &Regex::new("<[^>]+>")
                    .map(|r| r.replace_all(c.get(2).map(|m| m.as_str()).unwrap_or(""), ""))
                    .unwrap_or_default()
                    .to_string(),
            );
            (title, url)
        })
        .collect();
    let snippets: Vec<String> = snip_re
        .captures_iter(html)
        .take(limit)
        .map(|c| {
            decode_html(
                &Regex::new("<[^>]+>")
                    .map(|r| r.replace_all(c.get(1).map(|m| m.as_str()).unwrap_or(""), ""))
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            )
        })
        .collect();
    for (i, (title, url)) in links.into_iter().enumerate() {
        out.push((title, url, snippets.get(i).cloned().unwrap_or_default()));
    }
    out
}
