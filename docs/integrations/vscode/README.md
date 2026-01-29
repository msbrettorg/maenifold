# VS Code + Maenifold Integration

Graph-aware software engineering agents inside VS Code, backed by Maenifold's knowledge graph.

## Overview

This integration provides two VS Code chat agents that are deeply integrated with Maenifold:

- `maenifold` – a knowledge-graph enhanced SWE agent with persistent memory
- `agent-boss` – an orchestration agent that coordinates subagents and verifies their work

Both agents rely on Maenifold's `memory://` corpus, concept graph, and MCP tools instead of local, ephemeral prompts.

## Agents

### `maenifold` Agent

Defined in `docs/integrations/vscode/maenifold.agent.md`.

Behavior:

- At the start of every session:
  - `sync` – ensure the graph and embeddings reflect the latest markdown
  - `recent_activity` – find the most recent sessions and memory files
  - `build_context` / `find_similar_concepts` / `search_memories` – surface related knowledge
  - `read_memory` – pull in relevant `memory://` files
- For exploratory questions:
  - Synthesizes a hypothetical answer with `[[concepts]]` inline
  - Uses those concepts to drive retrieval via `build_context` + `search_memories`
- For complex work:
  - Uses `sequential_thinking` as the default loop primitive
  - Stores key thoughts and results back into memory with `write_memory`

This is a HYDE + Self-RAG agent: it hypotheses first, then uses the graph to validate and refine.

### `agent-boss` Agent

Defined in `docs/integrations/vscode/agent-boss.agent.md`.

Behavior:

- Decomposes user goals into atomic, testable tasks
- Launches parallel subagents via `runSubagent` (always using the `maenifold` agent)
- Uses Maenifold tools to coordinate:
  - `sequential_thinking` – shared reasoning sessions
  - `write_memory` – persist subagent outputs
  - `build_context` / `search_memories` – aggregate results
  - `assumption_ledger` – track and challenge assumptions
- Dispatches verification agents to confirm outcomes instead of trusting self-reports

This is a multi-agent Graph-RAG orchestrator: it uses the graph to both bootstrap and verify work across agents.

## How It Uses RAG Patterns

Both agents implement the patterns described in `docs/search-and-scripting.md`:

- **Graph-RAG** – All retrieval goes through `SearchMemories`, `BuildContext`, and `FindSimilarConcepts`, not ad-hoc string search.
- **HYDE** – For exploratory work, the agents synthesize a hypothetical answer with `[[concepts]]`, then search those concepts.
- **FLARE-like startup** – Each session begins by proactively querying the graph (`sync`, `recent_activity`, `build_context`) before answering.
- **Self-RAG / CRAG / iterative** – `SequentialThinking` is the loop primitive for revision, corrective retrieval, and multi-step reasoning.

## Installation & Configuration

How you wire these agents into VS Code depends on your chat/agent extension (e.g., GitHub Copilot, Continue, etc.). The general pattern is:

1. Place the agent definition files:
   - `docs/integrations/vscode/maenifold.agent.md`
   - `docs/integrations/vscode/agent-boss.agent.md`

2. In your agent-capable extension, configure custom agents to point at these files.

3. Ensure the Maenifold MCP server is available to the extension so tools like `maenifold/*` are usable.

Refer to your specific VS Code extension's documentation for how to register custom agents or MCP servers.

## See Also

- `docs/search-and-scripting.md` – full description of Maenifold's RAG and scripting patterns
- `docs/integrations/codex/swe.md` – Codex CLI SWE profile using the same patterns
- `docs/integrations/claude-code/` – Claude Code integration with session-start graph context
