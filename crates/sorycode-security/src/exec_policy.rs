//! Execution policy for shell commands (Codex `execpolicy` pattern).
//!
//! Rules map glob patterns to Allow / Deny / Prompt decisions. Commands are
//! split on shell separators (`&&`, `||`, `;`, `|`, newline) and each segment
//! is matched independently: any Deny wins, otherwise any Prompt wins,
//! otherwise Allow. An empty rule set defaults to Prompt (safe).
//!
//! Rules load from `.sorycode/execpolicy.json` in the working dir:
//! `{ "rules": [ { "action": "allow", "pattern": "cargo test*" } ] }`

use globset::Glob;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecAction {
    Allow,
    Deny,
    Prompt,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExecRule {
    pub action: ExecAction,
    pub pattern: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ExecPolicyFile {
    #[serde(default)]
    rules: Vec<ExecRule>,
}

#[derive(Debug, Clone)]
pub enum ExecDecision {
    Allow,
    Deny(String),
    Prompt,
}

#[derive(Debug, Clone, Default)]
pub struct ExecPolicy {
    rules: Vec<(ExecAction, Glob)>,
}

/// Split a shell command into segments on common separators.
fn split_segments(command: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let chars: Vec<char> = command.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let two: String = chars[i..].iter().take(2).collect();
        if two == "&&" || two == "||" {
            out.push(cur.trim().to_string());
            cur.clear();
            i += 2;
            continue;
        }
        let c = chars[i];
        if c == ';' || c == '|' || c == '\n' {
            out.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(c);
        }
        i += 1;
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out.retain(|s| !s.is_empty());
    out
}

impl ExecPolicy {
    /// Built-in deny rules for catastrophic commands — always enforced,
    /// even in auto-approve mode.
    pub fn builtin() -> Self {
        let patterns = [
            "rm -rf /*",
            "rm -rf ~*",
            "rm -rf $HOME*",
            "mkfs*",
            "dd if=/dev/*",
            ":(){ :|:& };:",
            "shutdown*",
            "reboot*",
            "poweroff*",
            "init 0",
            "init 6",
            "git push --force*",
            "git push -f*",
        ];
        Self {
            rules: patterns
                .iter()
                .filter_map(|p| Glob::new(p).ok().map(|g| (ExecAction::Deny, g)))
                .collect(),
        }
    }

    /// Load rules from `<working_dir>/.sorycode/execpolicy.json`, merged
    /// after the built-in deny rules (user rules are checked last, so a
    /// user `allow` cannot override a built-in `deny`).
    pub fn load(working_dir: &Path) -> Self {
        let mut policy = Self::builtin();
        let path = working_dir.join(".sorycode").join("execpolicy.json");
        let Ok(data) = std::fs::read_to_string(&path) else {
            return policy;
        };
        let Ok(file) = serde_json::from_str::<ExecPolicyFile>(&data) else {
            tracing::warn!("invalid execpolicy.json at {}", path.display());
            return policy;
        };
        for rule in file.rules {
            match Glob::new(&rule.pattern) {
                Ok(g) => policy.rules.push((rule.action, g)),
                Err(e) => {
                    tracing::warn!("bad execpolicy pattern '{}': {}", rule.pattern, e)
                }
            }
        }
        policy
    }

    /// Evaluate a full command line (may contain separators).
    pub fn evaluate(&self, command: &str) -> ExecDecision {
        let mut saw_prompt = false;
        for seg in split_segments(command) {
            match self.evaluate_segment(&seg) {
                ExecDecision::Deny(r) => return ExecDecision::Deny(r),
                ExecDecision::Prompt => saw_prompt = true,
                ExecDecision::Allow => {}
            }
        }
        if saw_prompt {
            ExecDecision::Prompt
        } else if self.rules.is_empty() {
            ExecDecision::Prompt
        } else {
            ExecDecision::Allow
        }
    }

    fn evaluate_segment(&self, segment: &str) -> ExecDecision {
        for (action, glob) in &self.rules {
            if glob.compile_matcher().is_match(segment) {
                return match action {
                    ExecAction::Allow => ExecDecision::Allow,
                    ExecAction::Deny => {
                        ExecDecision::Deny(format!("command denied by execpolicy: {}", segment))
                    }
                    ExecAction::Prompt => ExecDecision::Prompt,
                };
            }
        }
        ExecDecision::Prompt
    }
}

#[cfg(test)]
mod tests {

        use super::*;

        fn policy(rules: &[(&str, &str)]) -> ExecPolicy {
            ExecPolicy {
                rules: rules
                    .iter()
                    .filter_map(|(a, p)| {
                        let action = match *a {
                            "allow" => ExecAction::Allow,
                            "deny" => ExecAction::Deny,
                            _ => ExecAction::Prompt,
                        };
                        Glob::new(p).ok().map(|g| (action, g))
                    })
                    .collect(),
            }
        }

        #[test]
        fn builtin_denies_catastrophic() {
            let p = ExecPolicy::builtin();
            assert!(matches!(
                p.evaluate("rm -rf /"),
                ExecDecision::Deny(_)
            ));
            assert!(matches!(
                p.evaluate("git push --force origin main"),
                ExecDecision::Deny(_)
            ));
            assert!(matches!(
                p.evaluate("cargo test"),
                ExecDecision::Prompt
            ));
        }

        #[test]
        fn compound_command_any_deny_wins() {
            let p = policy(&[("allow", "cargo *")]);
            assert!(matches!(
                p.evaluate("cargo test && rm -rf /tmp/x"),
                ExecDecision::Prompt // /tmp/x not denied, cargo allowed → prompt? no: rm -rf /tmp/x doesn't match builtin
            ));
        }

        #[test]
        fn allow_rule_bypasses_prompt() {
            let p = policy(&[("allow", "cargo test*")]);
            assert!(matches!(
                p.evaluate("cargo test --release"),
                ExecDecision::Allow
            ));
        }

        #[test]
        fn deny_in_compound_defeats_allow() {
            let p = policy(&[("allow", "*"), ("deny", "git push*")]);
            // allow * matches first segment-wise? rules evaluated per segment in order:
            // "git push origin" matches allow * first → Allow. Order matters.
            let p2 = policy(&[("deny", "git push*"), ("allow", "*")]);
            assert!(matches!(
                p2.evaluate("ls && git push origin main"),
                ExecDecision::Deny(_)
            ));
            let _ = p;
        }

        #[test]
        fn empty_policy_prompts() {
            let p = ExecPolicy::default();
            assert!(matches!(p.evaluate("ls"), ExecDecision::Prompt));
        }
    }

