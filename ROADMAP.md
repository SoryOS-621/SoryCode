# ROADMAP.md — SoryCode Core

## Phase 0 — Audit & design (done)
Empty repository audited. Architecture, contracts, documents created.

## Phase 1 — Rust foundations (in progress)
Workspace, common types, errors, config, logs, protocols, events. Goal: first compilable engine.

## Phase 2 — Provider engine
OpenAI, Gemini, Mistral, OpenRouter, streaming, tool calling. Goal: real model request/response.

## Phase 3 — Tool engine
Filesystem, search, terminal, git, patch. Goal: real operations on test project.

## Phase 4 — Agent runtime
Agentic loop, tool calls, results, errors, cancellation. Goal: agent exploring/modifying project.

## Phase 5 — Context engine
Project discovery, AGENTS.md, context management, history, compaction. Goal: real repo without loading all files.

## Phase 6 — Orchestrator
Main/planner/coder/researcher/tester/reviewer/fixer. Goal: delegation and verification.

## Phase 7 — Execution engine
Local, E2B, Vercel. Goal: multi-environment execution.

## Phase 8 — Storage & interfaces
SQLite, CLI, Tauri IPC, desktop React. Goal: usable engine from multiple interfaces.
Desktop is split into three independent apps: `apps/desktop` (native shell),
`apps/ui` (design system) and `apps/app` (business application).

## Phase 9 — Validation & optimization
Integration tests, security, performance, RAM/CPU, packaging, CI/CD.
EOF