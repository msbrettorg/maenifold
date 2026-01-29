# maenifold

Knowledge graph and reasoning infrastructure for Claude Code.

## Prerequisites

The `maenifold` binary must be in your PATH.

## Installation

```bash
claude plugin add /path/to/plugin-maenifold
```

## What's Included

### MCP Server

Provides maenifold tools for knowledge graph operations:
- Memory read/write/search
- Concept extraction and graph traversal
- Sequential thinking and workflows
- Assumption ledger

### Hooks

| Event | Purpose |
|-------|---------|
| `SessionStart` | Builds context from knowledge graph on session resume |
| `PreCompact` | Preserves knowledge before context compaction |

### Skills

- **maenifold** - Reference documentation for all maenifold tools

## Usage

Once installed, maenifold tools are available via MCP. Use `[[WikiLinks]]` in your memories to build concept relationships.

```
Write a memory about the authentication system
Search memories for API design patterns
Build context around the user-auth concept
```
