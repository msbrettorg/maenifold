# Maenifold UI & Agent Architecture

> Status: Requirements captured

## Three things

### 1. Non-web UI for maenifold

A native interface — TUI (terminal), desktop app, or both. Not a browser tab. Shows the endophasia stream (thinking sessions, memory writes, concept connections) and lets you interact with the knowledge graph directly.

Candidates: Ratatui (Rust TUI), Tauri (desktop), or integrate with an existing tool (Obsidian plugin was considered but too opinionated).

### 2. Pluggable ACP clients

Maenifold doesn't build its own chat/agent system. Instead, it exposes an interface that any ACP client can plug into:

- Claude Code, Copilot, Codex, OpenCode — whatever the user already has
- Maenifold doesn't care which agent is talking to it
- Configuration says "these ACP clients are available" and maenifold can route to them

### 3. Shared MCP server instance

Today, every agent spawns its own maenifold process. That means separate DBs, separate indexes, separate state. Instead:

- One maenifold instance runs as the MCP server
- All ACP clients connect to that same instance
- Shared knowledge graph, shared thinking sessions, shared memory
- Agents can see each other's work in real time through the graph

```
                    ┌─────────────┐
                    │  maenifold  │ (single instance, MCP server)
                    │  SQLite DB  │
                    │  memory://  │
                    └──────┬──────┘
                           │ MCP (stdio or SSE)
              ┌────────────┼────────────┐
              │            │            │
        ┌─────┴────┐ ┌────┴─────┐ ┌────┴─────┐
        │ Claude   │ │ Copilot  │ │ Codex    │
        │ Code     │ │          │ │          │
        └──────────┘ └──────────┘ └──────────┘
              ACP clients (pluggable)
```

## What this means for Dispatch

Dispatch as originally designed is dead. But the concept of a PM agent coordinating work across multiple ACP clients through a shared maenifold instance — that's exactly this architecture. The PM just uses the existing tools (todos, thinking sessions, workflows) through the shared MCP server.

## Open questions

1. How do multiple ACP clients connect to one maenifold MCP server simultaneously? (stdio is 1:1 — need SSE/HTTP transport)
2. TUI vs desktop app vs both?
3. How does the UI connect to maenifold — as another MCP client, or direct DB access?
