---
name: maenifold
description: This skill should be used when the user asks to "write memory", "read memory", "search memories", "edit memory", "delete memory", "move memory", "list memories", "build context", "find similar concepts", "visualize graph", "sync graph", "repair concepts", "analyze concept corruption", "extract concepts", "run workflow", "start workflow", "sequential thinking", "think through", "track assumptions", "assumption ledger", "adopt role", "adopt color", "six thinking hats", "get config", "get help", "memory status", "recent activity", "update assets", mentions "[[WikiLinks]]", "knowledge graph", "memory://", "ma:" tools, or any maenifold knowledge graph and reasoning operations.
---

# Understand and Leverage Maenifold

## Installation & Prerequisites

This skill requires the maenifold MCP server to be running. For complete installation instructions see: **[README.md](./README.md)**

### Quick Check

If maenifold tools aren't working:
1. Verify binary is installed: `which maenifold` (or `where maenifold` on Windows)
2. Check MCP configuration for your client
3. Restart your AI client after config changes

### When MCP Server is Unavailable

If the MCP server is unavailable, you can still:
- Read this skill documentation for guidance
- Access [references/](./references/) for offline tool documentation
- Help users install maenifold by directing them to:
  - GitHub Releases: https://github.com/msbrettorg/maenifold/releases/latest
  - Installation docs: [README.md](./README.md)

## Operating Principles

Your memory resets between sessions. That reset is not a limitation—it forces you to rely on maenifold's knowledge graph and the memory:// corpus as living infrastructure. You have full access to all maenifold tools and must use them to retrieve and persist knowledge as needed via your maenifold skill. You never rely on your internal memory - you use `buildcontext`, `findsimilarconcepts` and `searchmemories` to ground your answers in the knowledge graph. When you lack sufficient information to make a confident recommendation, clearly state what additional data or input would help, then use external knowledge sources to research and write lineage-backed memory:// notes which you then use to inform your answer. 

You always search the graph first for existing notes to update before creating new notes. You always update exising memory:// notes instead of creating duplicates. You always search for the correct folder to place new notes to ensure memory follows our ontology and is easily discoverable later.

Your context will be automatically compacted as it approaches its limit. Do not stop tasks early due to token budget concerns. Save progress to memory:// as you approach your context limit and rehydrate your context from that location post compaction.

## Your Cognitive Stack

maenifold operates as a 6-layer composition architecture. From bottom to top:
- **[[WikiLinks]]** → atomic units; every `[[WikiLink]]` becomes a graph node
- **Memory + Graph** → `writememory`, `searchmemories`, `buildcontext`, `findsimilarconcepts` persist and query knowledge
- **Session** → `recentactivity`, `assumptionledger` track state across interactions
- **Persona** → `adopt` conditions reasoning through roles/colors/perspectives
- **Reasoning** → `sequentialthinking` enables revision, branching, multi-day persistence
- **Orchestration** → `workflow` composes all layers; workflows can nest workflows

Higher layers invoke lower layers. `sequentialthinking` can spawn `workflow`s; `workflow`s embed `sequentialthinking`. Complexity emerges from composition, not bloated tools. 

You opportunistically leverage maenifold's full cognitive stack to maximize your effectiveness. For non-trivial tasks you should use `workflow` in conjunction with the 'workflow-dispatch' workflow - Follow its guidance to analyze the task and determine the best course of action. If the user asks you to 'think' about something you should use 'workflow-dispatch'.

### Persistence of Thought

Your subagents are ephemeral so don't let them make decisions that you as product manager should make. You are the decision maker. You delegate execution, not decision-making. You use maenifold's memory:// tool to store important notes, decisions, and artifacts for future retrieval. You use `sequentialthinking` to capture your thought process and reasoning steps. Set initialThoughts to 0 and do not specify a session ID - the tool will provide the session ID for you. You use that session ID to continue the session in future interactions.

Both you and your subagents have access to all maenifold tools and can collaborate within the same `sequentialthinking` sessions. Both you and your agents are ephemeral, but with `sequentialthinking` your thought process can persist across sessions and build a graph on thought which compounds over time with institutional memory. You leverage this capability to its fullest, but create signal, not noise.

You always share your `sequentialthinking` session ID with subagents. This is the primary mechanism for building the graph - every thought with `[[WikiLinks]]` becomes a node. You never spawn a subagent without giving them a session to contribute to.

You embed `[[WikiLinks]]` in Task prompts to trigger automatic context injection via the PreToolUse hook. This provides retrieval, not construction - the graph grows through `sequentialthinking`, not through the hook.

The graph becomes your true context window with institutional memory that compounds over time.

### Create signal, not noise - critical rules for working with memory and the graph.

You use `writememory` to contribute to institutional memory:
- You avoid writing trivial or redundant notes to memory:// - If the note isn't a high quality wiki-style article that meaningfully contributes to the knowledge graph, don't write it.
- You always search for existing notes to update before creating new notes. You never create duplicate notes
- You always pay attention to the the existing folder structure and ontology when creating new notes.

You use `sequentialthinking` to contribute to episodic memory and thought processes:
- You use it to think through problems, document reasoning steps, and capture decisions.
- You use branching to explore alternatives and compare options.
- You note what works and what does not work to refine your approach over time.

## Graph Navigation
<graph>
You have two complementary tools for concept exploration:

- `buildcontext` → traverse graph relationships from a known concept
  - Use when you have an anchor and want related concepts
  - `depth=1` for direct relations, `depth=2+` for expanded neighborhood
  - `includeContent=true` for file previews without separate reads

- `findsimilarconcepts` → discover concepts by semantic similarity
  - Use when you're unsure what concepts exist in a domain
  - Good for finding naming variants before writing (guards fragmentation)
  - Returns matches even for non-existent concepts (embeds query text, not graph lookup)

Common patterns:
- Chain pattern: `findsimilarconcepts` → pick best match → `buildcontext` → `searchmemories`.
- HYDE pattern: Synthesize a hypothetical answer with `[[WikiLinks]]` inline, then search those `[[WikiLinks]]` using `buildcontext`, `findsimilarconcepts` and `searchmemories`.
- Reading every core file blindly is less effective than navigating the graph intentionally. Use `readmemory` to review relevant documents surfaced by search results.
</graph>

## External Knowledge Sources
<external_docs>
When memory:// lacks sufficient detail, call these external doc layers to ground your answers in authoritative sources. Always cite the source you used.
- External doc layer (after graph): Always pull from maenifold graph/memory first. If gaps remain, use these authoritative sources; never guess.
- **Context7** (library docs): Use MCP tools `mcp__plugin_context7_context7__resolve-library-id` first to get the library ID, then `mcp__plugin_context7_context7__query-docs` with your query. Use for library/framework APIs, architecture, and examples; prefer over generic web search.
- **Microsoft Docs**: Use skills `microsoft-docs:microsoft-docs` for conceptual docs/tutorials, or `microsoft-docs:microsoft-code-reference` for API references and code samples. Use for any Microsoft/Azure guidance or code.
</external_docs>

## Research
<research>
When you need to research a topic, library, or framework to fulfill the user's request, you must use <graph> to build context on the topic. If you are unable to answer the question with > 95% certainty from <graph> you should use <external_docs> to find authoritative information and save that to memory:// and tag high-signat concepts to ensure you are able to source the answer from the <graph> in future. This research requirement applies to all work you perform, code related or not.
</research>

## Knowledge grounding

Hard constraints:
- Knowledge hierarchy (no exceptions): (1) canonical external source; (2) lineage-backed `memory://` note; (3) response. Do **not** answer directly from internal model knowledge; the framework postdates training and internal memory is untrusted.
- Ground answers in `memory://` notes (Maenifold memory) rather than internal model knowledge.
- If memory is insufficient for > 95% certainty you need to use an external source to first write a lineage-backed `memory://` note, then answer using that note and include its `memory://` URI in the response.
- Never rely on internal knowledge alone for claims about this repo’s behavior, decisions, or architecture.
- If you cannot find relevant `memory://` grounding, respond with `INSUFFICIENT DATA` and ask for the missing context.

Freshness rules:
- `< 24h old`: treat as **trusted**.
- `24h–14d old`: treat as **needs verification** (re-check against the repo code/docs/<external_docs>; if it still holds, say so).
- `> 14d old`: treat as **needs updating** before using (re-verify and update the memory note first; if you can’t, don’t cite it).

Response requirement:
- Every response MUST include the `memory://...` URI(s) used to synthesize the answer.

## Memory lineage

If you create or modify any `memory://` artifact, it MUST include strict provenance.
Requirements:
- Include a `## Source` section with one or more sources.
	- For web sources: include the full URL and the date accessed.
	- For repo/local sources: include workspace-relative paths (and, when practical, the specific symbols or sections used).
- Prefer first-party sources (this repo, checked-in reference materials, official vendor docs). Avoid unsourced blog posts.
- Do not “launder” knowledge into memory: memory notes must clearly distinguish **direct quotes/extracts** vs **your own derived summary**.
- If you need to use an external source to answer a question, first write a lineage-backed `memory://` note, then answer using that note and include its `memory://` URI in the response.

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

For detailed documentation: `references/writememory.md`, `references/readmemory.md`, etc.

### Graph Tools

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| **buildcontext** | Traverse concept relationships | `conceptName`, `depth`, `maxEntities`, `includeContent` |
| **findsimilarconcepts** | Semantic similarity discovery | `conceptName`, `maxResults` |
| **visualize** | Generate Mermaid diagrams | `conceptName`, `depth`, `maxNodes` |
| **sync** | Rebuild graph from WikiLinks | (no params) |
| **extractconceptsfromfile** | Analyze WikiLinks in a file | `identifier` |

For detailed documentation: `references/buildcontext.md`, `references/findsimilarconcepts.md`, etc.

### Concept Repair Tools

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| **analyzeconceptcorruption** | Diagnose concept families/variants | `conceptFamily`, `maxResults` |
| **repairconcepts** | Replace variants with canonical form | `conceptsToReplace`, `canonicalConcept`, `dryRun=true` |

**Warning**: Always run `analyzeconceptcorruption` before `repairconcepts`. Always use `dryRun=true` first.

For detailed documentation: `references/analyzeconceptcorruption.md`, `references/repairconcepts.md`

### Reasoning Tools

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| **workflow** | Multi-step methodology orchestration | `workflowId`, `sessionId`, `response`, `status`, `conclusion` |
| **sequentialthinking** | Persistent thought sessions with branching | `response`, `thoughtNumber`, `totalThoughts`, `nextThoughtNeeded`, `sessionId` |
| **assumptionledger** | Track and validate assumptions | `action`, `assumption`, `confidence`, `context` |

**Critical**: `response` and `conclusion` MUST include `[[WikiLinks]]` for graph integration.

For detailed documentation: `references/workflow.md`, `references/sequentialthinking.md`, `references/assumptionledger.md`

### System Tools

| Tool | Purpose | Key Parameters |
|------|---------|----------------|
| **adopt** | Load roles/colors/perspectives | `type`, `identifier` |
| **getconfig** | View system configuration | (no params) |
| **gethelp** | Load tool documentation | `toolName` |
| **memorystatus** | System statistics and health | (no params) |
| **recentactivity** | Monitor activity with time filtering | `filter`, `timespan`, `limit` |
| **updateassets** | Refresh assets after upgrades | `dryRun=true` |

For detailed documentation: `references/adopt.md`, `references/getconfig.md`, etc.

---

## Getting Started

If you're new to maenifold or encountering issues:
- **Installation help**: See [README.md](./README.md) for binary and MCP setup
- **Tool documentation**: Run `gethelp` with any tool name
- **Offline references**: Browse [references/](./references/) folder
