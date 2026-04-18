---
name: maenifold
description: This skill should be used when the user asks to "write memory", "read memory", "search memories", "edit memory", "delete memory", "move memory", "list memories", "build context", "find similar concepts", "visualize graph", "sync graph", "repair concepts", "analyze concept corruption", "extract concepts", "run workflow", "start workflow", "sequential thinking", "think through", "track assumptions", "assumption ledger", "adopt role", "adopt color", "six thinking hats", "get config", "get help", "memory status", "recent activity", "update assets", mentions "[[WikiLinks]]", "knowledge graph", "memory://", "ma:" tools, or any maenifold knowledge graph and reasoning operations.
---

# Understand and Leverage Maenifold

## The Pattern

Maenifold is a compounding wiki maintained by LLMs. Unlike retrieval-augmented generation — where documents are fetched, consumed, and forgotten — the LLM writes knowledge back into the corpus after every meaningful interaction. Retrieval improves as the corpus grows. Each session builds on every prior session. The loop closes on itself, and institutional memory accumulates across sessions, agents, and time.

The substrate is three layers. First, plain markdown files containing `[[WikiLinks]]` — symbolic pointers the LLM embeds in prose as it writes. Second, a co-occurrence routing graph built mechanically from those WikiLinks: when two concepts appear in the same file, they share an edge. This graph is a routing index, not an ontology — it encodes which concepts appear together, not what they mean. Normalization is mechanical and deliberately non-semantic; interpretation is deferred to query time, where the LLM decides what each co-occurrence signifies. Third, hybrid search combining semantic vectors, full-text BM25, and graph traversal, so knowledge can be found by meaning, by keyword, or by structural proximity.

Five operations compose on this substrate. The LLM **writes** — creating and editing memory files whose WikiLinks grow the graph. It **searches** — hybrid retrieval across the full corpus. It **traverses** — hopping graph edges to build context around a concept. It **reasons** — sequential thinking sessions that persist across invocations, support branching and revision, and themselves emit WikiLinks that feed the graph. It **orchestrates** — workflows compose these operations into structured methodologies; pipelines compose workflows into DAGs with concurrent agent dispatch via ACP.

What prevents the wiki from decaying into noise? Conditioning and convention, not enforcement. Roles, colors, and perspectives shape how the LLM frames a problem. Workflows provide procedural scaffolds. Constitutional principles — graph sovereignty, signal discipline, epistemic honesty — act as soft schema: they do not constrain what can be represented, but strongly shape what is worth writing. The system provides mechanism; the LLM provides judgment.

This works because of what maenifold deliberately omits. The graph carries no built-in semantics, no type system, no inference engine — it is dumb infrastructure the LLM reads through. An LLM with full context interprets a co-occurrence edge more usefully than any schema we could predefine. Maintenance is radically asymmetric: writing a WikiLink costs one bracketed phrase; the routing value it creates compounds across every future query by every future agent. The system tracks freshness; the LLM decides what matters. Every feature not built into the substrate is room for intelligence to emerge.

This is the abstract pattern. What follows teaches the specific tools, conventions, and protocols that instantiate it.

## Installation

This skill requires the maenifold MCP server. See [README.md](./README.md) for binary install and MCP setup. If tools aren't responding, verify `which maenifold` resolves, check your client's MCP config, and restart the client. When the server is unavailable, you can still read this skill and [references/](./references/) to guide the user to https://github.com/msbrettorg/maenifold/releases/latest.

## Operating Stance

Your context resets between sessions. Treat the graph and `memory://` corpus as your real memory — internal model knowledge is untrusted, especially for anything that postdates training or concerns this repo's specifics. Before answering, search the graph. Before writing, search for what already exists. Before inventing a concept, look for a near-duplicate. The cost of a duplicate note is fragmentation of the routing index; the cost of a missing search is hallucination.

Your context will be auto-compacted as it approaches the limit. Do not stop work early to conserve tokens. Save progress to `memory://` as you approach the ceiling and rehydrate from those notes after compaction.

## Persistence of Thought

Sessions are ephemeral; the graph is not. Two mechanisms turn ephemeral compute into durable infrastructure:

**`sequentialthinking` for reasoning.** Every thought with `[[WikiLinks]]` becomes a node. Start a session by passing `initialThoughts: 0` and no session ID — the tool returns one. Use that session ID across multiple turns and across days. Use revision and branching when you want to explore alternatives without losing the original line of thought.

**Session sharing with subagents.** You are the decision maker; subagents execute. When you spawn a subagent, pass it your `sequentialthinking` session ID so its thoughts feed the same graph. Embed `[[WikiLinks]]` in the Task prompt — the PreToolUse hook injects matching graph context automatically. The hook does retrieval; the graph grows through the thinking sessions themselves.

For non-trivial tasks, use `workflow` with the `workflow-dispatch` workflow. It analyzes the task and routes to an appropriate methodology. If the user says "think about X," reach for `workflow-dispatch`.

## Knowledge Grounding

The hierarchy of trust, in order:

1. **Canonical external source** (this repo's code/docs, official vendor docs).
2. **Lineage-backed `memory://` note** that cites such a source.
3. **Your response**, which cites the note(s) it drew from.

Rules that follow from this:

- Never answer this repo's behavior, decisions, or architecture from internal model knowledge alone.
- If `memory://` lacks the grounding for >95% certainty, fetch from an authoritative external source, write a lineage-backed note, then answer from the note.
- Every response includes the `memory://...` URI(s) used to synthesize it.
- If no relevant grounding exists and you cannot acquire it, respond `INSUFFICIENT DATA` and ask for the missing context.

**Freshness.** `< 24h` trusted. `24h–14d` needs verification against current code/docs. `> 14d` needs updating before reuse — re-verify and update the note first, or don't cite it.

**Lineage.** Any `memory://` note you create or modify must carry a `## Source` section. Web sources include full URL and access date. Repo sources include workspace-relative paths and, when practical, specific symbols/sections. Prefer first-party sources; avoid unsourced blogs. Distinguish direct extracts from your derived summary — never launder external text into the graph as if it were native knowledge.

**External sources, in order of preference when the graph is insufficient:**
- **Context7** for library/framework APIs: `mcp__plugin_context7_context7__resolve-library-id` then `query-docs`.
- **Microsoft Docs**: `microsoft-docs:microsoft-docs` for concepts, `microsoft-docs:microsoft-code-reference` for APIs and samples.
- Generic web search only when neither covers the topic.

This grounding requirement applies to all work, code or otherwise.

## Graph Navigation

Two complementary tools for concept exploration:

- **`buildcontext`** traverses graph relationships from a known anchor. Use `depth=1` for direct neighbors, `depth=2+` to expand. `includeContent=true` returns previews so you skip a separate `readmemory` round-trip.
- **`findsimilarconcepts`** finds concepts by semantic similarity, including ones that don't exist yet (it embeds the query text). Use it to discover naming variants *before* writing — this is your primary defense against fragmenting the graph with near-duplicates.

Two patterns worth memorizing:

- **Chain pattern**: `findsimilarconcepts` → pick the best anchor → `buildcontext` → `searchmemories` for relevant files → `readmemory` on the highest-signal hits. E.g., when researching authentication, you might land on `[[JWT]]` as the anchor, traverse to `[[token-rotation]]` and `[[refresh-tokens]]`, then read the surfaced notes.
- **HYDE pattern**: Draft a hypothetical answer with `[[WikiLinks]]` inline — `[[vector-embeddings]]`, `[[BM25]]`, `[[reciprocal-rank-fusion]]` — then feed those concepts into `buildcontext`/`findsimilarconcepts`/`searchmemories`. The hypothesis seeds retrieval; the corpus corrects you.

Reading every plausible file blindly is worse than navigating the graph intentionally.

## Writing Signal, Not Noise

`writememory` is how the wiki compounds. Most notes you consider writing should not be written. Write a note only when it is a high-quality, wiki-style article that future agents will benefit from retrieving. The test: would another agent searching this domain in three months be glad this note exists?

Before writing, always:

1. `searchmemories` and `findsimilarconcepts` to find an existing note to update instead.
2. `listmemories` to find the right folder. **The folder structure is the ontology** — `azure/`, `finops/`, `tech/`, with sub-domains nested (`azure/billing/`, `tech/ml/`). Place notes where they will be discovered. Extend the structure only when a genuinely new domain emerges.
3. Choose `[[WikiLinks]]` that route. A good note about a deadlock fix might link `[[deadlock]]`, `[[mutex]]`, `[[tokio-runtime]]`, `[[async-rust]]` — substantive concepts an agent in the same domain would search for.

For thinking, `sequentialthinking` is your episodic record. Use it to capture reasoning, document decisions, branch through alternatives, and note what worked and what didn't. The graph absorbs both kinds of writing — articles and thoughts — and the routing improves either way.

## WikiLink Anti-Patterns

`[[WikiLinks]]` are graph nodes. Bad nodes corrupt the routing index for every future agent. The system filters the worst offenders via `.wikilink-filter.json`, but you should not rely on that — write good concepts in the first place.

Real failure modes observed in the corpus:

- **Prompt leakage**: `[[load-the-gate-skill-and-sync-the-gate-cache-in-users-brett-src-ma-collective-ma]]`, `[[sequential-thinking-required-the-blue-hat-opens-every-six-thinking-hats-session]]`. Sentences and instructions wrapped in brackets. Concepts are nouns or short noun phrases, not directives.
- **God nodes**: `[[software-tool]]` (7,438 edges), `[[testing-tool]]` (6,820), `[[rust]]` (601 files). When a concept connects to everything, it routes to nothing — `buildcontext` returns noise. Prefer specifics: `[[cargo-workspace]]`, `[[clippy-lint]]`, `[[tokio-runtime]]`.
- **Code symbols**: `[[AgentPool]]`, `[[PipelineComposer]]`, `[[WorkflowStepTimeline]]`. Implementation details, not domain concepts. Tag the *behavior* (`[[agent-pooling]]`, `[[workflow-orchestration]]`), not the struct name.
- **Trivial tags**: `[[tool]]`, `[[memory]]` (531 files), `[[concept]]`, `[[file]]`, `[[the]]`. Too generic to be useful retrieval anchors.
- **Cosmetic violations** (silently normalized — still avoid): underscores `[[my_concept]]`, slashes `[[foo/bar]]`, double hyphens, leading/trailing hyphens, `[single-bracket]`.

Good concept conventions: double brackets, singular for the general concept (`[[agent]]`), plural only for the collection meaning (`[[tools]]`), primary noun not qualifier compound (`[[MCP]]` over `[[MCP-server]]`), `findsimilarconcepts` first to reuse what exists.

Example of a well-tagged sentence: `Fixed [[null-reference-exception]] in [[authentication]] flow by hardening [[JWT]] validation.`

## Quick Reference

Per-tool docs are not duplicated here. Call `gethelp` with the PascalCase tool name (e.g., `WriteMemory`, `BuildContext`, `Workflow`) to load canonical help.

### Memory

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| **writememory** | Create knowledge files with graph integration | `title`, `content` (must contain `[[WikiLinks]]`), `folder`, `tags` |
| **readmemory** | Retrieve files by URI or title | `identifier`, `includeChecksum` |
| **searchmemories** | Hybrid/Semantic/FullText search | `query`, `mode`, `folder`, `tags` |
| **editmemory** | Modify with checksum-based safety | `identifier`, `operation`, `content`, `checksum` |
| **deletememory** | Remove with confirmation | `identifier`, `confirm=true` |
| **movememory** | Relocate/rename, preserving WikiLinks | `source`, `destination` |
| **listmemories** | Explore folder structure (the ontology) | `path` |

### Graph

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| **buildcontext** | Traverse from a concept anchor | `conceptName`, `depth`, `maxEntities`, `includeContent` |
| **findsimilarconcepts** | Semantic discovery, including non-existent concepts | `conceptName`, `maxResults` |
| **visualize** | Mermaid diagram of a neighborhood | `conceptName`, `depth`, `maxNodes` |
| **sync** | Rebuild graph from WikiLinks on disk | (none) |
| **extractconceptsfromfile** | List WikiLinks in a file | `identifier` |

### Concept Repair

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| **analyzeconceptcorruption** | Diagnose families/variants (e.g., `[[tool]]` vs `[[tools]]` vs `[[software-tool]]`) | `conceptFamily`, `maxResults` |
| **repairconcepts** | Replace variants with a canonical form | `conceptsToReplace`, `canonicalConcept`, `dryRun=true` |

Always run `analyzeconceptcorruption` first, and always use `dryRun=true` before applying.

### Reasoning

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| **workflow** | Multi-step methodology orchestration | `workflowId`, `sessionId`, `response`, `status`, `conclusion` |
| **sequentialthinking** | Persistent thought sessions; branching & revision | `response`, `thoughtNumber`, `totalThoughts`, `nextThoughtNeeded`, `sessionId` |
| **assumptionledger** | Track and validate assumptions | `action`, `assumption`, `confidence`, `context` |

`response` and `conclusion` MUST contain `[[WikiLinks]]` — that's how reasoning feeds the graph.

### Pipeline

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| **pipeline** | Compose workflows into a validated DAG | `definition` (nodes + edges), `maxConcurrency` |

Each node references a `workflowId` and optional `acpClient` for agent dispatch. Edges define flow control (e.g. `success`). The tool validates the graph (no cycles, no duplicates, no dangling refs), creates a persistent session, and returns ready nodes for scheduling.

### System

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| **adopt** | Load a role / color / perspective to condition reasoning | `type`, `identifier` |
| **getconfig** | View resolved configuration and paths | (none) |
| **gethelp** | Load canonical tool documentation | `toolName` |
| **memorystatus** | Corpus statistics and health | (none) |
| **recentactivity** | Recently indexed files, optionally with content | `filter`, `timespan`, `limit` |
| **updateassets** | Refresh packaged assets after upgrade | `dryRun=true` |
