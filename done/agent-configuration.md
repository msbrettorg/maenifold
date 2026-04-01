---
id: T-80.8
title: Agent Configuration
status: done
pipeline: sdd
requirement: FR-32
---

# agent-configuration

**Requirement**: FR-32
**Test Cases**: TC-32.12
**Component**: crates/maenifold-core/src/acp/config.rs (new)

## Acceptance Criteria

1. `agents.json` located at `$MAENIFOLD_ROOT/agents.json` (same directory as `memory.db`), consistent with existing config resolution via `Config::current()`
2. Format matches ACP registry schema: `{ "agents": { "<name>": { "command": "...", "args": [...], "env": { ... } } } }`
3. Agent `command` field in `agents.json` SHALL be validated at startup: must be an absolute path, must exist, and must be executable. Invalid entries are logged as warnings and excluded from the registry.
4. Parsed at startup into an `AgentRegistry` struct listing available agents
5. If `agents.json` is missing, maenifold starts with an empty agent registry (no error)
6. If `agents.json` is malformed, maenifold logs a warning and starts with an empty registry (fail-open)
7. `GET /api/agents` returns the list of configured agents as JSON: `[{ "name": "...", "status": "available" }]`
8. Agent names from the registry are available in the SPA's agent selector dropdown

## Verdicts

- TC-32.12 (agents.json parsed at startup): ADD — test does not exist yet
