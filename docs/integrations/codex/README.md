# Codex CLI + Maenifold Integration

A Maenifold-backed SWE agent running inside the Codex CLI, using the knowledge graph and memory corpus instead of ephemeral prompts.

## Overview

This integration configures a Codex CLI software engineering agent to:

- Use Maenifold's `memory://` corpus as its source of truth
- Rebuild context at the start of every session using graph queries
- Implement HYDE-style hypothetical retrieval and Self-RAG correction
- Persist important reasoning and results back into the graph

The core behavior is defined in `docs/integrations/codex/swe.md`.

## SWE Agent Behavior

The SWE profile in `swe.md` instructs the agent to:

- **Rebuild context at session start**:
  - `#sync` – ensure the graph and vectors reflect the latest markdown
  - `#Recent_activity` – inspect the most recently touched sessions and memory files
  - `#build_context` / `#find_similar_concepts` / `#search_memories` – surface related knowledge beyond a single file
  - `#read_memory` – load relevant `memory://` files

- **Navigate the graph for retrieval**:
  - For exploratory queries, synthesize a hypothetical answer with `[[concepts]]` inline
  - Use those concepts with `build_context` + `search_memories` to retrieve supporting material
  - Use `find_similar_concepts` when you don't know which concepts exist yet

- **Tag concepts correctly**:
  - Follow the concept tagging rules (double-bracket `[[concept]]`, hyphenated, reuse existing concepts, avoid trivial tags)
  - Treat folder structure as the ontology (`azure/`, `finops/`, `tech/`, etc.)

- **Use SequentialThinking as the loop primitive**:
  - Break down complex work into steps
  - Revise and branch when needed
  - Use it before git operations, major edits, or when requirements are unclear

## RAG Patterns Implemented

The Codex SWE agent is a concrete instance of the patterns described in `docs/search-and-scripting.md`:

- **Graph-RAG**
  - All retrieval goes through `SearchMemories`, `BuildContext`, and `FindSimilarConcepts`.
  - Uses both text and semantic scores, plus graph relationships.

- **HYDE (Hypothetical Retrieval)**
  - For open-ended questions, the agent drafts a plausible answer with `[[concepts]]`.
  - It then searches those concepts to retrieve real supporting files.

- **FLARE-style Startup**
  - Every session starts by proactively querying the graph (sync + recent activity + context building) before answering.

- **Self-RAG / CRAG / Iterative Patterns**
  - `SequentialThinking` provides the loop for:
    - checking answers against acceptance criteria,
    - requesting additional retrieval when gaps are found,
    - tracking assumptions via `AssumptionLedger`.

## Wiring Into Codex CLI

The exact wiring depends on your Codex CLI harness configuration, but the general pattern is:

1. Point the SWE agent's system prompt or profile at `docs/integrations/codex/swe.md`.
2. Ensure the Maenifold MCP server is available in the tool list so that `maenifold/*` tools can be called.
3. Make sure the workspace points at the Maenifold repository and `memory/` tree so the agent can read and update knowledge.

Refer to your Codex CLI harness documentation for how to:

- Register a custom SWE profile
- Attach MCP servers (including Maenifold)
- Expose tools like `maenifold/*` to the agent

## See Also

- `docs/integrations/codex/swe.md` – full SWE profile
- `docs/integrations/vscode/README.md` – VS Code agents using the same patterns
- `docs/integrations/claude-code/` – Claude Code session-start graph integration
- `docs/search-and-scripting.md` – RAG and scripting patterns underpinning all integrations
