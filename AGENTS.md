# AGENTS.md — SoryCode Core

## Project identity
SoryCode Core is the independent agentic programming engine of the SoryOS ecosystem.
Language: Rust. Async runtime: Tokio. It is independent of Tauri, React and the CLI.

## Repository layout
- `crates/` — Rust workspace crates, one responsibility each.
- `apps/desktop/` — native Tauri shell (Rust only): windows, OS, IPC, permissions.
- `apps/ui/` — React design system: primitives, components, layouts, themes, styles.
- `apps/app/` — React application: chat, code, work, projects, workspaces, sessions, agents, settings.
- `packages/` — shared TypeScript packages: `types` (domain), `contracts` (wire IPC/events), `config` (tsconfig base).
- `docs/` — architecture, modules, protocols, security, development.
- `tests/` — integration tests.

## Crate dependency direction (acyclic)
```
sorycode-core
  <- sorycode-events
  <- sorycode-protocol
  <- sorycode-storage
  <- sorycode-security
  <- sorycode-context
  <- sorycode-execution
  <- sorycode-tools (depends on core, security)
  <- sorycode-providers (depends on core)
  <- sorycode-agent (depends on core, events, protocol, providers, tools, context, security, storage)
  <- sorycode-orchestrator (depends on core, agent, providers, events)
  <- sorycode-application (depends on core, events, protocol, storage, agent, orchestrator, context, providers)
  <- sorycode-cli (depends on application, storage, agent, orchestrator, providers, events)
```
No crate may depend on a crate that depends on it.

## Application dependency direction
```
packages/types <- packages/contracts <- apps/ui <- apps/app <- apps/desktop
```
`apps/app` imports `@sorycode/ui` for visuals and `@sorycode/contracts` for
wire types. `apps/desktop` exposes native capabilities through IPC and loads
the `apps/app` build; it contains no React. The Core never depends on any app.

## Coding conventions
- Rust edition 2024, MSRV 1.85.
- Prefer `thiserror` for library errors, `anyhow` only in binaries.
- All public APIs documented. Types derive `Debug`, and `Clone`/`Serialize` when sensible.
- Async traits use `async_trait`.
- Errors are never silently swallowed; log via `tracing`.
- No `unwrap()` in library code except on infallible operations; tests may unwrap.

## Agent loop contract
See `docs/architecture/`. The runtime implements:
User Request -> Task Init -> Context Build -> Model Request -> Interpret -> (Final | Tool Calls -> Permission -> Execute -> Collect -> Update -> Model Request).

## Tool contract
Every tool implements `Tool`:
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    async fn execute(&self, input: serde_json::Value, context: ToolContext) -> Result<ToolOutput, ToolError>;
}
```

## Security
- Default policy allows reads; writes/exec require approval in strict mode.
- Secrets never stored in plaintext in SQLite.
- All actions auditable.

## Build constraints
Development machine: 2 cores, 4 GB RAM. Use `CARGO_BUILD_JOBS=1`. Prefer debug builds. Do not run all tests at once. Measure RAM/CPU.

## Phase status
Phase 0 (audit) done: repository was empty. Phase 1 (foundations) in progress.
EOF