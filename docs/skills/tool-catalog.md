# Tool catalog — SoryCode agent

All tools the agent can call. Every call is a JSON object validated against the
schema; results return as `tool` role messages.

## Files

| Tool | Params | Notes |
|---|---|---|
| `read_file` | `path*`, `offset?`, `limit?` | 1-based line range, numbered output, shows `[lines a-b of n]` when paginated. Read before editing — never guess file contents. |
| `write_file` | `path*`, `content*` | Creates parent dirs. **Confined to working_dir** — `..` and absolute paths outside are rejected. Prefer `edit_file` for changes to existing files. |
| `edit_file` | `path*`, `old_string*`, `new_string?`, `replace_all?` | Exact string replacement. Fails if `old_string` missing or ambiguous (then add context or `replace_all`). |
| `list_files` | `path?`, `max_depth?=4`, `limit?=500` | Recursive, respects `.gitignore`, skips hidden. |
| `apply_patch` | `patch*` | Unified diff via `git apply`. Use for multi-file changes. |

## Search

| Tool | Params | Notes |
|---|---|---|
| `search` | `pattern*` (regex), `path?`, `glob?` (`*.rs`), `limit?=200` | ripgrep-style `path:line: content`. Skips binary files and `.gitignore`d paths. |
| `web_search` | `query*`, `limit?=5` | DuckDuckGo, no API key. Titles/URLs/snippets. Needs network approval. |

## Execution

| Tool | Params | Notes |
|---|---|---|
| `terminal` | `command*`, `timeout_ms?=60000`, `cwd?`, `session_id?`, `close?` | `sh -c`. With `session_id` → persistent shell (cwd/env persist); `close=true` kills it. Timeout kills the session. Output truncated at 30KB, `[exit N]` appended, `exit_code` in metadata. |
| `run_tests` | `command?="cargo test"`, `timeout_ms?=300000` | Same runner, test-oriented default. |

## Git & planning

| Tool | Params | Notes |
|---|---|---|
| `git_status` | — | Deferred: discover via `tool_search` first. |
| `git_diff` | — | Deferred: discover via `tool_search` first. |
| `tool_search` | `query*`, `limit?=10` | Finds deferred tools (git_*) by name/description. |
| `update_plan` | `plan*` = `[{step, status}]` | Visible todo list. Statuses: `pending`, `in_progress`, `completed`. Max 30 steps, **only one `in_progress`**. Call again to revise the whole plan — it replaces, not appends. |

## Rules of use

1. **Plan first** on multi-step tasks: `update_plan` with 3–8 steps, keep exactly
   one `in_progress`, update as you go.
2. **Read before write**: `read_file` the target (and neighbors) before
   `edit_file`/`write_file`/`apply_patch`.
3. **Search before creating**: `search`/`list_files` to find existing code and
   conventions; don't duplicate utilities.
4. Prefer `edit_file` over `write_file` for existing files; `apply_patch` for
   coordinated multi-file edits.
5. `terminal` for builds/tests/git; use `session_id` (e.g. `"build"`) when a
   sequence of commands shares state.
6. Verify: after edits, run the relevant check (`cargo check`, tests, lint)
   before declaring done.
