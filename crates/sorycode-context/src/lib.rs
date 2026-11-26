use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct ProjectDiscovery {
    pub root: PathBuf,
}

impl ProjectDiscovery {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    pub fn list_files(&self, max_depth: usize, limit: usize) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let walker = WalkDir::new(&self.root)
            .max_depth(max_depth)
            .follow_links(false);
        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.to_string_lossy().contains("/.git/") {
                continue;
            }
            if path
                .components()
                .any(|c| c.as_os_str().to_string_lossy().starts_with('.') && c.as_os_str() != ".")
            {
                // skip hidden files dirs except root dotfiles? skip
            }
            out.push(path.to_path_buf());
            if out.len() >= limit {
                break;
            }
        }
        out
    }
    pub fn read_agents_md(&self) -> Option<String> {
        for name in ["AGENTS.md", "agents.md"] {
            let p = self.root.join(name);
            if p.is_file() {
                return std::fs::read_to_string(p).ok();
            }
        }
        None
    }
}

pub struct ContextBuilder {
    pub files: Vec<PathBuf>,
    pub agents_md: Option<String>,
    pub tree_summary: String,
}

impl ContextBuilder {
    pub fn build(root: &Path) -> Self {
        let disc = ProjectDiscovery::new(root);
        let files = disc.list_files(4, 200);
        let agents_md = disc.read_agents_md();
        let tree_summary = files
            .iter()
            .map(|p| p.strip_prefix(root).unwrap_or(p).display().to_string())
            .collect::<Vec<_>>()
            .join("\n");
        Self {
            files,
            agents_md,
            tree_summary,
        }
    }
    pub fn to_system_prompt(&self) -> String {
        let mut s = String::new();
        if let Some(a) = &self.agents_md {
            s.push_str("# Project Instructions (AGENTS.md)\n");
            s.push_str(a);
            s.push_str("\n\n");
        }
        s.push_str("# Project File Tree (partial)\n");
        s.push_str(&self.tree_summary);
        s
    }
}

/// Structured snapshot of the working environment injected into the system
/// prompt (Codex "world state" pattern): cwd, git branch, dirty file count,
/// and the most recent commit subject. Cheap: three short git calls, once
/// per task — never per turn.
pub fn world_state(root: &Path) -> String {
    let mut s = String::from("# Environment\n");
    s.push_str(&format!("working_dir: {}\n", root.display()));

    let git = |args: &[&str]| -> Option<String> {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    };

    if let Some(branch) = git(&["rev-parse", "--abbrev-ref", "HEAD"]) {
        s.push_str(&format!("git_branch: {}\n", branch));
    }
    if let Some(status) = git(&["status", "--porcelain"]) {
        let dirty = status.lines().filter(|l| !l.trim().is_empty()).count();
        s.push_str(&format!("git_dirty_files: {}\n", dirty));
    }
    if let Some(last) = git(&["log", "-1", "--pretty=%s"]) {
        s.push_str(&format!("git_last_commit: {}\n", last));
    }
    s
}
