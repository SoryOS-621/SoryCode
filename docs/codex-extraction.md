# Codex feature extraction — status

Reference: vendored Codex source at `soryos-assistant/crates/codex-main/codex-rs`
(~150 crates). Only the *patterns* were extracted; no Codex code is vendored here.

## How Codex works (what we learned)

- History is a list of typed `ResponseItem`s persisted as rollout JSONL — not
  plain chat messages. Enables reliable session resume, compaction, per-turn diffs.
- A `StepContext` per model call carries permissions, environments, hooks.
- `ToolRegistry` + `ToolOrchestrator`: typed `ToolName` (namespaced), dynamic
  exposure (immediate vs deferred via `tool_search`), per-call sandbox selection,
  approval routing (hooks → guardian review → exec policy), cached approvals for
  canonicalized commands, and sandbox-denial retry with escalation.
- Compaction is automatic by token budget with structured summary + reinjection
  of recent items; context is assembled from ~60 instruction fragments
  (AGENTS.md, skills, plugins, world state) rather than a fixed system prompt.
- Built-in tools: `unified_exec` (persistent PTY sessions), `apply_patch`
  (freeform), `update_plan`, `view_image`, `web.run`, `spawn_agent` /
  `send_message` / `followup_task` (multi-agent), `request_user_input`,
  `new_context_window`, `get_context_remaining`, MCP tools.

## Extracted so far

| Codex feature | Where | Status |
|---|---|---|
| Approval caching (canonical cmd approved once → auto-allow repeats) | `sorycode-agent` `approval_cache` keyed by `tool:args-json` | done |
| Persistent shell sessions (`unified_exec`-like) | `sorycode-tools` `terminal` `session_id`/`close`, marker-based completion, timeout kills session | done |
| Provider retry on transient errors (429/5xx/network) | `sorycode-agent` `call_provider_with_retry` (3 attempts, backoff) | done |
| Tool-call validation (empty name / non-object args → error to model) | `sorycode-agent` tool loop | done |
| Context compaction (summary + tail, token-budget trigger) | `sorycode-agent` `compact_context` at >40 messages | done |
| Confined writes / `.gitignore`-aware list+search / read pagination / `edit_file` | `sorycode-tools` | done |
| Plan-then-execute UX | CLI `/plan <task>` | done (simple) |
| World state injection (cwd, git branch, dirty count, last commit) | `sorycode-context::world_state` appended to system prompt once per task | done |
| Native `update_plan` tool (validated todo list, max 1 in_progress) | `sorycode-tools::plan::UpdatePlanTool` + agent schema/permission | done |
| Exec policy: allow/deny/prompt rules on shell commands | `sorycode-security::exec_policy` — compound-command splitting, globset matching, built-in catastrophic denies (`rm -rf /`, `mkfs`, `dd if=/dev`, fork bomb, `git push -f`), user rules from `.sorycode/execpolicy.json` (checked after built-ins so allow can't override deny) | done |
| Cancellation tokens (`CancelTask` actually stops the loop) | `sorycode-application` + agent/orchestrator | done |
| Interactive approvals + strict policy + `--yolo` | `sorycode-cli` `CliApproval` | done |
| Tool registry typé + deferred tools (`tool_search`) | `sorycode-tools` `ToolExposure` (Immediate/Deferred), `ToolSearchTool` substring search, agent exposes only Immediate tools in `tool_defs` | done |

## Not yet extracted (backlog, by ROI)

1. **Rollout/items typés** — replace `Vec<Message>` history with typed items
   persisted per session (enables real resume + per-turn diffs). Big refactor of
   `sorycode-core`/`storage`.
2. **Richer world state** — add recently-modified files and env hints; today it
   covers cwd/git branch/dirty count/last commit only.
3. ~~Tool registry typé + deferred tools~~ — done (`ToolExposure` +
   `tool_search`; git_status/git_diff deferred). Remaining gap: tool names
   are not namespaced and MCP tools don't exist yet.
4. ~~Exec policy engine~~ — done (`exec_policy.rs`). Remaining gap: approved
   commands are not persisted across sessions (cache is per-task).
5. **Guardian/second-model review** for auto-approval mode.
6. ~~Native `update_plan` tool~~ — done (`sorycode-tools::plan`).
7. **Multi-agent** — `spawn_agent` with isolated context (orchestrator roles
   already stubbed).
8. **Sandboxing** — workspace-write confinement is partial (write tools
   confined; `terminal` still runs unsandboxed commands).
