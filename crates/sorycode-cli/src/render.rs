//! Pretty output rendering for the CLI: boxed cards for assistant messages,
//! styled tool events, approvals and errors (Codex/OpenCode-style).

use std::io::{self, Write};

pub const CYAN: &str = "\x1b[36m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const GRAY: &str = "\x1b[90m";
pub const RESET: &str = "\x1b[0m";

fn width() -> usize {
    // keep cards readable; cap at 96, min 50
    std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(100)
        .saturating_sub(4)
        .clamp(50, 96)
}

fn wrap_line(line: &str, w: usize) -> Vec<String> {
    if line.chars().count() <= w {
        return vec![line.to_string()];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in line.split(' ') {
        if cur.is_empty() {
            cur = word.to_string();
        } else if cur.chars().count() + 1 + word.chars().count() <= w {
            cur.push(' ');
            cur.push_str(word);
        } else {
            out.push(cur);
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Render markdown-lite: headers, bullets, code fences stay readable inside
/// the card. Long content is truncated with a marker.
pub fn assistant_card(title: &str, body: &str, max_lines: usize) -> String {
    let w = width();
    let inner = w.saturating_sub(4); // "│ " + content + " │"
    let mut out = String::new();
    out.push_str(&format!("{}╭{}╮{}\n", CYAN, "─".repeat(w - 2), RESET));
    out.push_str(&format!(
        "{}│ {}{} {}{} │{}\n",
        CYAN, BOLD, title, RESET, DIM, RESET
    ));
    out.push_str(&format!("{}├{}┤{}\n", CYAN, "─".repeat(w - 2), RESET));

    let mut lines: Vec<String> = Vec::new();
    for raw in body.lines() {
        for wl in wrap_line(raw, inner) {
            lines.push(wl);
        }
    }
    let total = lines.len();
    let show = lines.len().min(max_lines);
    for l in &lines[..show] {
        let pad = inner.saturating_sub(l.chars().count());
        out.push_str(&format!("{}│ {}{} │{}\n", CYAN, l, " ".repeat(pad), RESET));
    }
    if total > show {
        let note = format!("… {} more lines", total - show);
        let pad = inner.saturating_sub(note.chars().count());
        out.push_str(&format!(
            "{}│ {}{} │{}\n",
            DIM,
            note,
            " ".repeat(pad),
            RESET
        ));
    }
    out.push_str(&format!("{}╰{}╯{}", CYAN, "─".repeat(w - 2), RESET));
    out
}

/// Compact one-line status for tool lifecycle events.
pub fn tool_line(kind: &str, name: &str, detail: &str, ok: Option<bool>) -> String {
    let (icon, color) = match ok {
        Some(true) => ("✓", GREEN),
        Some(false) => ("✗", RED),
        None => ("▸", CYAN),
    };
    let _ = kind;
    format!("{} {} {} {}{}", GRAY, icon, color, name, RESET)
        + &format!("{} — {}{}", DIM, detail, RESET)
}

/// Highlighted approval prompt line.
pub fn approval_line(action: &str, reason: &str) -> String {
    let a = format!("{}{}{}", BOLD, action, RESET);
    let r = format!("{}{}{}", DIM, reason, RESET);
    format!("{}⚠ approval {} — {} [y/N]{}", YELLOW, a, r, RESET)
}

/// Error card (small, red border).
pub fn error_card(title: &str, body: &str) -> String {
    let w = width().min(80);
    let inner = w.saturating_sub(4);
    let mut out = format!("{}╭{}╮\n", RED, "─".repeat(w - 2));
    out.push_str(&format!("{}│ {}{} │\n", RED, BOLD, title));
    out.push_str(&format!("{}├{}┤\n", RED, "─".repeat(w - 2)));
    for raw in body.lines().take(12) {
        for wl in wrap_line(raw, inner) {
            let pad = inner.saturating_sub(wl.chars().count());
            out.push_str(&format!("{}│ {}{} │\n", RED, wl, " ".repeat(pad)));
        }
    }
    out.push_str(&format!("{}╰{}╯{}", RED, "─".repeat(w - 2), RESET));
    out
}

/// Print a rendered block to stdout.
pub fn print_block(s: &str) {
    println!("{}", s);
    let _ = io::stdout().flush();
}
