---
name: maenifold
description: This skill should be used when the user asks to "write memory", "read memory", "search memories", "edit memory", "delete memory", "move memory", "list memories", "build context", "find similar concepts", "visualize graph", "sync graph", "repair concepts", "analyze concept corruption", "extract concepts", "run workflow", "start workflow", "sequential thinking", "think through", "track assumptions", "assumption ledger", "adopt role", "adopt color", "six thinking hats", "get config", "get help", "memory status", "recent activity", "update assets", "list assets", "read resource", mentions "[[WikiLinks]]", "knowledge graph", "memory://", or any maenifold knowledge graph and reasoning operations.
---

# Understand and Leverage Maenifold

## Installation & Prerequisites

This skill requires the maenifold binary to be installed. For complete installation instructions see: **[references/README.md](./references/README.md)**

### Quick Check

If maenifold tools are not working:
1. Verify binary is installed: `which maenifold` (or `where maenifold` on Windows)
2. Check MCP configuration for the active client
3. Restart the AI client after config changes

### CLI-First, MCP as Fallback

Prefer CLI execution (`maenifold --tool <ToolName> --payload '{...}'`) over MCP tool calls. Code execution reduces token consumption by avoiding tool-definition overhead and intermediate-result bloat ([rationale](https://www.anthropic.com/engineering/code-execution-with-mcp)). Use MCP tools as fallback when CLI execution is unavailable or impractical.

For CLI scripting patterns, search techniques, and RAG composition recipes, see: **[references/SCRIPTING.md](./references/SCRIPTING.md)**

### When MCP Server is Unavailable

If the MCP server is unavailable:
- Fall back to CLI: `maenifold --tool <ToolName> --payload '{...}'`
- Read this skill documentation for guidance
- Browse [usage/](./usage/) for per-tool API documentation
- Browse [references/](./references/) for architecture and operational guides
- Direct users to install maenifold:
  - GitHub Releases: https://github.com/msbrettorg/maenifold/releases/latest
  - Installation docs: [references/README.md](./references/README.md)

## Operating Principles

Memory resets between sessions. That reset is not a limitation—it forces reliance on maenifold's knowledge graph and the memory:// corpus as living infrastructure. Use all maenifold tools to retrieve and persist knowledge as needed. Never rely on internal memory—use `buildcontext`, `findsimilarconcepts` and `searchmemories` to ground answers in the knowledge graph. When insufficient information exists for a confident recommendation, clearly state what additional data or input would help, then use external knowledge sources to research and write lineage-backed memory:// notes to inform the answer.

Always search the graph first for existing notes to update before creating new ones. Always update existing memory:// notes instead of creating duplicates. Always search for the correct folder to place new notes to ensure memory follows the ontology and is easily discoverable later.

Context will be automatically compacted as it approaches its limit. Do not stop tasks early due to token budget concerns. Save progress to memory:// when approaching the context limit and rehydrate from that location post compaction.

## Cognitive Stack

maenifold operates as a 6-layer composition architecture. From bottom to top:
- **[[WikiLinks]]** → atomic units; every `[[WikiLink]]` becomes a graph node
- **Memory + Graph** → `writememory`, `searchmemories`, `buildcontext`, `findsimilarconcepts` persist and query knowledge
- **Session** → `recentactivity`, `assumptionledger` track state across interactions
- **Persona** → `adopt` conditions reasoning through roles/colors/perspectives
- **Reasoning** → `sequentialthinking` enables revision, branching, multi-day persistence
- **Orchestration** → `workflow` composes all layers; workflows can nest workflows

Higher layers invoke lower layers. `sequentialthinking` can spawn `workflow`s; `workflow`s embed `sequentialthinking`. Complexity emerges from composition, not bloated tools. 

Opportunistically leverage maenifold's full cognitive stack to maximize effectiveness. For non-trivial tasks, use `workflow` in conjunction with the 'workflow-dispatch' workflow—follow its guidance to analyze the task and determine the best course of action. If the user asks to 'think' about something, use 'workflow-dispatch'.

### Persistence of Thought

Subagents are ephemeral. Use maenifold's memory:// tool to store important notes, decisions, and artifacts for future retrieval. Use `sequentialthinking` to capture thought processes and reasoning steps. Set `totalThoughts` to the initial estimate of thoughts needed and do not specify a session ID—the tool will provide the session ID automatically. Use that session ID to continue the session in future interactions.

All agents have access to all maenifold tools and can collaborate within the same `sequentialthinking` sessions. All agents are ephemeral, but with `sequentialthinking` thought processes persist across sessions and build a graph on thought which compounds over time with institutional memory. Leverage this capability fully, but create signal, not noise.

Always share the `sequentialthinking` session ID with subagents. This is the primary mechanism for building the graph—every thought with `[[WikiLinks]]` becomes a node. Never spawn a subagent without giving them a session to contribute to.

Embed `[[WikiLinks]]` in Task prompts to trigger automatic context injection via the PreToolUse hook. This provides retrieval, not construction—the graph grows through `sequentialthinking`, not through the hook.

The graph becomes the true context window with institutional memory that compounds over time.

### Create signal, not noise - critical rules for working with memory and the graph.

Use `writememory` to contribute to institutional memory:
- Avoid writing trivial or redundant notes to memory://—if the note is not a high quality wiki-style article that meaningfully contributes to the knowledge graph, do not write it.
- Always search for existing notes to update before creating new ones. Never create duplicate notes.
- Always pay attention to the existing folder structure and ontology when creating new notes.

Use `sequentialthinking` to contribute to episodic memory and thought processes:
- Think through problems, document reasoning steps, and capture decisions.
- Use branching to explore alternatives and compare options.
- Note what works and what does not work to refine the approach over time.

## Graph Navigation
<graph>
Two complementary tools support concept exploration:

- `buildcontext` → traverse graph relationships from a known concept
  - Use with a known anchor to find related concepts
  - `depth=1` for direct relations, `depth=2+` for expanded neighborhood
  - `includeContent=true` for file previews without separate reads

- `findsimilarconcepts` → discover concepts by semantic similarity
  - Use when unsure what concepts exist in a domain
  - Good for finding naming variants before writing (guards fragmentation)
  - Returns matches even for non-existent concepts (embeds query text, not graph lookup)

Common patterns:
- Chain pattern: `findsimilarconcepts` → pick best match → `buildcontext` → `searchmemories`.
- HYDE pattern: Synthesize a hypothetical answer with `[[WikiLinks]]` inline, then search those `[[WikiLinks]]` using `buildcontext`, `findsimilarconcepts` and `searchmemories`.
- Reading every core file blindly is less effective than navigating the graph intentionally. Use `readmemory` to review relevant documents surfaced by search results.
</graph>

## External Knowledge Sources
<external_docs>
When memory:// lacks sufficient detail, call these external doc layers to ground answers in authoritative sources. Always cite the source used.
- External doc layer (after graph): Always pull from maenifold graph/memory first. If gaps remain, use these authoritative sources; never guess.
- **Context7** (library docs): Use MCP tools `mcp__plugin_context7_context7__resolve-library-id` first to get the library ID, then `mcp__plugin_context7_context7__query-docs` with your query. Use for library/framework APIs, architecture, and examples; prefer over generic web search.
- **Microsoft Docs**: Use skills `microsoft-docs:microsoft-docs` for conceptual docs/tutorials, or `microsoft-docs:microsoft-code-reference` for API references and code samples. Use for any Microsoft/Azure guidance or code.
</external_docs>

## Research
<research>
When researching a topic, library, or framework to fulfill the user's request, use <graph> to build context on the topic. If unable to answer the question with > 95% certainty from <graph>, use <external_docs> to find authoritative information and save that to memory:// and tag high-signal concepts to ensure the answer can be sourced from <graph> in future. This research requirement applies to all work performed, code related or not.
</research>

## Knowledge grounding

Hard constraints:
- Knowledge hierarchy (no exceptions): (1) canonical external source; (2) lineage-backed `memory://` note; (3) response. Do **not** answer directly from internal model knowledge; the framework postdates training and internal memory is untrusted.
- Ground answers in `memory://` notes (Maenifold memory) rather than internal model knowledge.
- If memory is insufficient for > 95% certainty, use an external source to first write a lineage-backed `memory://` note, then answer using that note and include its `memory://` URI in the response.
- Never rely on internal knowledge alone for claims about this repo’s behavior, decisions, or architecture.
- If no relevant `memory://` grounding exists, respond with `INSUFFICIENT DATA` and ask for the missing context.

Freshness rules:
- `< 24h old`: treat as **trusted**.
- `24h–14d old`: treat as **needs verification** (re-check against the repo code/docs/<external_docs>; if it still holds, say so).
- `> 14d old`: treat as **needs updating** before using (re-verify and update the memory note first; if unable to verify, do not cite it).

Response requirement:
- Every response MUST include the `memory://...` URI(s) used to synthesize the answer.

## Memory lineage

When creating or modifying any `memory://` artifact, include strict provenance.
Requirements:
- Include a `## Source` section with one or more sources.
	- For web sources: include the full URL and the date accessed.
	- For repo/local sources: include workspace-relative paths (and, when practical, the specific symbols or sections used).
- Prefer first-party sources (this repo, checked-in reference materials, official vendor docs). Avoid unsourced blog posts.
- Do not “launder” knowledge into memory: memory notes must clearly distinguish **direct quotes/extracts** vs **your own derived summary**.
- When using an external source to answer a question, first write a lineage-backed `memory://` note, then answer using that note and include its `memory://` URI in the response.

## Concept Tagging

WikiLinks are graph nodes. Bad tagging = graph corruption = broken context recovery.

**Ontology**: Folder structure is the ontology. Run `listmemories` to see current domains (e.g., `azure/`, `finops/`, `tech/`). Nest for sub-domains (e.g., `azure/billing/`, `tech/ml/`). Align new concepts with existing folders; extend structure when a new domain emerges.

- Double brackets: `[[authentication]]` never `[authentication]`
- Normalized to lowercase-with-hyphens internally
- SINGULAR for general: `[[tool]]`, `[[agent]]`, `[[test]]`
- PLURAL only for collections: `[[tools]]` when meaning "all tools"
- PRIMARY concept only: `[[MCP]]` not `[[MCP-server]]`
- GENERAL terms: `[[authentication]]` not `[[auth-system]]`
- NO file paths, code elements, or trivial words (`[[the]]`, `[[a]]`, `[[file]]`)
- TAG substance: `[[machine-learning]]`, `[[GraphRAG]]`, `[[vector-embeddings]]`
- REUSE existing concepts before inventing near-duplicates (guard fragmentation)
- HYPHENATE multiword: `[[null-reference-exception]]` not `[[Null Reference Exception]]`

Anti-patterns (silently normalized but avoid):
- Underscores: `[[my_concept]]` → use `[[my-concept]]`
- Slashes: `[[foo/bar]]` → use `[[foo-bar]]` or separate concepts
- Double hyphens: `[[foo--bar]]` → use `[[foo-bar]]`
- Leading/trailing hyphens: `[[-database-]]` → use `[[database]]`

Example: `Fixed [[null-reference-exception]] in [[authentication]] using [[JWT]]`

## Quick Reference

### Memory Tools

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| **writememory** | Create knowledge files with graph integration | `title`, `content` (must have `[[WikiLinks]]`), `folder`, `tags` |
| **readmemory** | Retrieve files by URI or title | `identifier`, `includeChecksum` |
| **searchmemories** | Find files via Hybrid/Semantic/FullText search | `query`, `mode`, `folder`, `tags` |
| **editmemory** | Modify existing files with checksum safety | `identifier`, `operation`, `content`, `checksum` |
| **deletememory** | Remove files with confirmation | `identifier`, `confirm=true` |
| **movememory** | Relocate/rename preserving WikiLinks | `source`, `destination` |
| **listmemories** | Explore folder structure | `path` |

For detailed documentation: [writememory](./usage/writememory.md), [readmemory](./usage/readmemory.md), [searchmemories](./usage/searchmemories.md), [editmemory](./usage/editmemory.md), [deletememory](./usage/deletememory.md), [movememory](./usage/movememory.md), [listmemories](./usage/listmemories.md)

### Graph Tools

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| **buildcontext** | Traverse concept relationships | `conceptName`, `depth`, `maxEntities`, `includeContent` |
| **findsimilarconcepts** | Semantic similarity discovery | `conceptName`, `maxResults` |
| **visualize** | Generate Mermaid diagrams | `conceptName`, `depth`, `maxNodes` |
| **sync** | Rebuild graph from WikiLinks | (no params) |
| **extractconceptsfromfile** | Analyze WikiLinks in a file | `identifier` |

For detailed documentation: [buildcontext](./usage/buildcontext.md), [findsimilarconcepts](./usage/findsimilarconcepts.md), [visualize](./usage/visualize.md), [sync](./usage/sync.md), [extractconceptsfromfile](./usage/extractconceptsfromfile.md)

### Concept Repair Tools

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| **analyzeconceptcorruption** | Diagnose concept families/variants | `conceptFamily`, `maxResults` |
| **repairconcepts** | Replace variants with canonical form | `conceptsToReplace`, `canonicalConcept`, `dryRun=true` |

**Warning**: Always run `analyzeconceptcorruption` before `repairconcepts`. Always use `dryRun=true` first.

For detailed documentation: [analyzeconceptcorruption](./usage/analyzeconceptcorruption.md), [repairconcepts](./usage/repairconcepts.md)

### Reasoning Tools

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| **workflow** | Multi-step methodology orchestration | `workflowId`, `sessionId`, `response`, `status`, `conclusion` |
| **sequentialthinking** | Persistent thought sessions with branching | `response`, `thoughtNumber`, `totalThoughts`, `nextThoughtNeeded`, `sessionId` |
| **assumptionledger** | Track and validate assumptions | `action`, `assumption`, `confidence`, `context` |

**Critical**: `response` and `conclusion` MUST include `[[WikiLinks]]` for graph integration.

For detailed documentation: [workflow](./usage/workflow.md), [sequentialthinking](./usage/sequentialthinking.md), [assumptionledger](./usage/assumptionledger.md)

### System Tools

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| **adopt** | Load roles/colors/perspectives | `type`, `identifier` |
| **listassets** | Discover available assets by type | `type` (optional) |
| **readmcpresource** | Read asset content by URI | `uri` |
| **getconfig** | View system configuration | (no params) |
| **gethelp** | Load tool documentation | `toolName` |
| **memorystatus** | System statistics and health | (no params) |
| **recentactivity** | Monitor activity with time filtering | `filter`, `timespan`, `limit` |
| **updateassets** | Refresh assets after upgrades | `dryRun=true` |

For detailed documentation: [adopt](./usage/adopt.md), [listassets](./usage/listassets.md), [readmcpresource](./usage/readmcpresource.md), [getconfig](./usage/getconfig.md), [gethelp](./usage/gethelp.md), [memorystatus](./usage/memorystatus.md), [recentactivity](./usage/recentactivity.md), [updateassets](./usage/updateassets.md)

---

## References

### Operational Guides

- **[references/BOOTSTRAP.md](./references/BOOTSTRAP.md)** — Build domain expertise from empty graph to institutional memory. Covers the full journey: seed, research, specialize, systematize, operate, maintain. Start here when onboarding a new domain.
- **[references/SCRIPTING.md](./references/SCRIPTING.md)** — CLI scripting patterns, RAG technique support matrix, search composition recipes, and validated CLI flows. The canonical reference for composing maenifold tools via code execution.
- **[references/SECURITY_MODEL.md](./references/SECURITY_MODEL.md)** — STRIDE threat analysis, data flow diagram, trust boundaries, and security controls reference.
- **[references/README.md](./references/README.md)** — Binary installation (macOS/Linux/Windows), MCP configuration for all clients, environment variables, troubleshooting.

### Tool Usage (per-tool API docs)

| Category | Tools |
|----------|-------|
| Memory | [writememory](./usage/writememory.md), [readmemory](./usage/readmemory.md), [searchmemories](./usage/searchmemories.md), [editmemory](./usage/editmemory.md), [deletememory](./usage/deletememory.md), [movememory](./usage/movememory.md), [listmemories](./usage/listmemories.md) |
| Graph | [buildcontext](./usage/buildcontext.md), [findsimilarconcepts](./usage/findsimilarconcepts.md), [visualize](./usage/visualize.md), [sync](./usage/sync.md), [extractconceptsfromfile](./usage/extractconceptsfromfile.md) |
| Repair | [analyzeconceptcorruption](./usage/analyzeconceptcorruption.md), [repairconcepts](./usage/repairconcepts.md) |
| Reasoning | [workflow](./usage/workflow.md), [sequentialthinking](./usage/sequentialthinking.md), [assumptionledger](./usage/assumptionledger.md) |
| System | [adopt](./usage/adopt.md), [listassets](./usage/listassets.md), [readmcpresource](./usage/readmcpresource.md), [getconfig](./usage/getconfig.md), [gethelp](./usage/gethelp.md), [memorystatus](./usage/memorystatus.md), [recentactivity](./usage/recentactivity.md), [updateassets](./usage/updateassets.md) |

---

## Getting Started

If new to maenifold or encountering issues:
- **Installation help**: See [references/README.md](./references/README.md) for binary and MCP setup
- **Domain bootstrapping**: See [references/BOOTSTRAP.md](./references/BOOTSTRAP.md) for the seed-to-operate journey
- **CLI patterns**: See [references/SCRIPTING.md](./references/SCRIPTING.md) for scripting and RAG composition
- **Tool documentation**: Run `gethelp` with any tool name, or browse [usage/](./usage/)
