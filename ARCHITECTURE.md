# ARCHITECTURE.md — SoryCode Core

## Layers
1. Application Layer: bootstrap, command handler, task/session coordinators, event dispatcher.
2. Agent Domain Layer: runtime, orchestrator, manager, planner, tool-call manager, conversation, context manager.
3. Capability Layer: provider, tool, context, execution, git, search, security engines.
4. Infrastructure Layer: HTTP client, SQLite repository, filesystem adapter, PTY adapter, git adapter, sandbox adapters, config, secrets.
5. Interface Layer: Tauri IPC, CLI, event protocol, command protocol, serialization.

## Module communication
Modules communicate through explicit traits. No circular dependencies. Events are published through `EventSink`.

## Agent runtime loop
```
User Request
  -> Task Initialization
  -> Context Construction
  -> Model Request
  -> Response Interpretation
       -> Final Answer -> Task Complete
       -> Tool Calls -> Permission Check -> Tool Execution -> Result Collection -> Context Update -> Model Request
```

## Protocol
Commands: CreateProject, OpenProject, CreateSession, SendMessage, CancelTask, ApproveAction, RejectAction, GetSession, ListSessions, GetTaskStatus, ExecuteTool.
Events: TaskStarted, TaskProgress, MessageDelta, ToolStarted, ToolOutput, ToolCompleted, ApprovalRequired, TaskCompleted, TaskFailed, TaskCancelled.
Every event carries project/session/task/tool identifiers.

## Interfaces
- CLI (`crates/sorycode-cli`): REPL, provider/model selection, slash-command menu.
- Desktop shell (`apps/desktop`): Tauri IPC, native capabilities, event bridge.
- Design system (`apps/ui`): React primitives, components, layouts, themes.
- Application (`apps/app`): chat, code, work, projects, workspaces, sessions, agents, settings.
See `docs/architecture/apps.md` and `docs/architecture/cli.md`.

## Storage
SQLite via SQLx. Entities: Projects, Workspaces, Sessions, Messages, Tasks, ToolCalls, Approvals, Providers, Settings, Migrations.
Secrets are never stored in plaintext.

## Execution environments
Common trait `ExecutionEnvironment`: exec, read_file, write_file, list_dir. Implementations: Local, E2B, Vercel Sandbox.
EOF