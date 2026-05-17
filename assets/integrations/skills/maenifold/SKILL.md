---
name: maenifold
description: This skill should be used when the user asks to "write memory", "read memory", "search memories", "edit memory", "delete memory", "move memory", "list memories", "build context", "find similar concepts", "visualize graph", "sync graph", "repair concepts", "analyze concept corruption", "extract concepts", "run workflow", "start workflow", "sequential thinking", "think through", "track assumptions", "assumption ledger", "adopt role", "adopt color", "six thinking hats", "get config", "get help", "memory status", "recent activity", "update assets", mentions "[[WikiLinks]]", "knowledge graph", "memory://", "ma:" tools, or any maenifold knowledge graph and reasoning operations.
---

# Understand and Leverage Maenifold

> **Gateway-first rule:** the public MCP surface is only seven tools. Most real work (`WriteMemory`, `BuildContext`, `SequentialThinking`, `Workflow`, `GetConfig`, etc.) is **not** a top-level MCP tool. It is a gateway command you invoke through `GatewayExecute`. On cold start, do **not** assume you know which Gateway-backed domains are live — call `GatewayIndex` first and treat its response as the source of truth.

## Start Here in a Fresh Session

If you only remember one thing, remember this sequence:

1. **Discover the live domain surface** with `GatewayIndex` **first**. On cold start, you do **not** know which additional Gateway-backed server domains are currently configured.
2. **Load usage details** with `GetHelp` for the specific public tool or gateway command.
3. **Inspect packaged assets/resources** with `ReadMcpResource` when you need `asset://...` content.
4. **Inspect live downstream MCP servers** with `DiscoverMcpTools` when the question is about configured MCP servers/tool inventories rather than live gateway domains.
5. **Run the actual operation** with `GatewayExecute`.

Minimal startup flow:

```text
GatewayIndex                                       # see domains
GatewayIndex domain="maenifold.memory" includeSchemas=true
GetHelp toolName="WriteMemory"                     # or GatewayExecute, BuildContext, etc.
GatewayExecute domain="maenifold.memory" command="WriteMemory" parameters={...}
```

`GatewayIndex` is the live source of truth for **all currently exposed domains** — both native `maenifold.*` domains and any additional Gateway-backed server domains. Runtime-visible extras can include names like `ai-infrastructure`, `azure-mcp-server`, `fetch`, `msx-mcp`, `shell-mcp`, `srectl`, and `workiq`, but that set is runtime-dependent, so discover it live instead of assuming a fixed list.

## Public MCP Tools vs Gateway Commands

### Direct public entry points

These are the only top-level MCP tools you should expect to see:

| Tool | Use it for | Do not expect |
|------|------------|---------------|
| **GatewayIndex** | Start here. Discover the live domains, commands, schemas, and safety class. It is the source of truth for both native `maenifold.*` domains and any additional Gateway-backed server domains currently exposed. | It does not execute commands. |
| **GatewayExecute** | Run one exact domain+command call. | It does not infer intent. |
| **GatewayBatch** | Submit several exact gateway calls in order. | It is not discovery. |
| **GatewayIntent** | Translate natural language into gateway candidates when you do not yet know the exact command. | It is less deterministic than `GatewayIndex`. |
| **DiscoverMcpTools** | See the live MCP tools exposed by configured downstream servers, grouped by server/tool inventory. | It does **not** replace `GatewayIndex` for live domain discovery. |
| **ReadMcpResource** | Read `asset://...` and other MCP resource URIs. | It does **not** read `memory://` notes. |
| **GetHelp** | Load canonical help for either a public tool or a gateway command name. | It does not inspect runtime state. |

### Gateway commands

These are the operations you usually want, but they are **not** separate top-level MCP tools:

- `WriteMemory`, `ReadMemory`, `EditMemory`, `SearchMemories`
- `BuildContext`, `FindSimilarConcepts`, `Visualize`, `Sync`
- `SequentialThinking`, `AssumptionLedger`
- `Workflow`, `Pipeline`
- `GetConfig`, `MemoryStatus`, `RecentActivity`, `Adopt`, `ListAssets`, `AcpExec`

**Hard boundary:** `AcpExec` is an ACP runtime tool for Maenifold UI / ACP agent-session execution and ACP-specific smoke tests. It is **not** a worker-dispatch mechanism for Gate tasks, `task-pm`, outer/inner-loop coding work, or `GatewayIntent` resolution. PM/task execution must use the host agent/task dispatch surfaces and Gate transitions must use `GatewayExecute maenifold.gate`.

You call them like this:

```text
GatewayExecute
  domain: "maenifold.memory"
  command: "WriteMemory"
  parameters: { "title": "...", "content": "...[[Concept]]...", "folder": "..." }
```

When this skill says "use `WriteMemory`" or "run `SequentialThinking`", read that as shorthand for:

- identify the command with `GatewayIndex`
- confirm usage with `GetHelp` or `includeSchemas=true`
- invoke it through `GatewayExecute`

## The Discovery Failures This Skill Must Prevent

Agents commonly lose time in the same ways:

1. **They search for `WriteMemory` or `BuildContext` as if those were public MCP tools.** They are usually gateway commands instead.
2. **They open maenifold on the theory section first.** The practical startup path must come before the abstract model.
3. **They use `DiscoverMcpTools` when they really need `GatewayIndex`.** `DiscoverMcpTools` inventories tools grouped by configured downstream server; `GatewayIndex` discovers the live gateway domains you can actually call.
4. **They assume only built-in `maenifold.*` domains matter.** `GatewayIndex` can also surface additional Gateway-backed domains from configured servers, so cold-start discovery must cover the whole live domain set.
5. **They use `ReadMcpResource` for the wrong data plane.** `ReadMcpResource` is for MCP resources like `asset://catalog`; `memory://` content is handled by memory commands over the gateway. If you try `gateway://...` URIs here, expect a resource-format mismatch rather than domain discovery.

## Default Discovery Sequence

Use this default path unless you have a specific reason not to:

1. **`GatewayIndex` with no filters** — learn the available live domains first. This is the cold-start discovery step for native maenifold domains **and** any additional Gateway-backed server domains currently exposed.
2. **`GatewayIndex domain="..." includeSchemas=true`** — inspect exact command names and parameter shapes.
3. **`GetHelp toolName="..."`** — load canonical instructions when the command/tool semantics matter.
4. **`GatewayExecute`** — run the command with an exact `domain`, `command`, and `parameters` object.

Only branch from that default when:

- you need packaged asset/resource content → `ReadMcpResource`
- you need the live list of configured downstream MCP tools → `DiscoverMcpTools`
- you only have a natural-language request and need candidates first → `GatewayIntent`
- you need several exact calls in sequence → `GatewayBatch`

## Installation

This skill requires the maenifold MCP server. See [README.md](./README.md) for binary install and MCP setup. If tools aren't responding, verify `which maenifold` resolves, check your client's MCP config, and restart the client. When the server is unavailable, you can still read this skill and [references/](./references/) to guide the user to https://github.com/msbrettorg/maenifold/releases/latest.

## The Pattern

Maenifold is a compounding wiki maintained by LLMs. Unlike retrieval-augmented generation — where documents are fetched, consumed, and forgotten — the LLM writes knowledge back into the corpus after every meaningful interaction. Retrieval improves as the corpus grows. Each session builds on every prior session. The loop closes on itself, and institutional memory accumulates across sessions, agents, and time.

The substrate is three layers. First, plain markdown files containing `[[WikiLinks]]` — symbolic pointers the LLM embeds in prose as it writes. Second, a co-occurrence routing graph built mechanically from those WikiLinks: when two concepts appear in the same file, they share an edge. This graph is a routing index, not an ontology — it encodes which concepts appear together, not what they mean. Normalization is mechanical and deliberately non-semantic; interpretation is deferred to query time, where the LLM decides what each co-occurrence signifies. Third, hybrid search combining semantic vectors, full-text BM25, and graph traversal, so knowledge can be found by meaning, by keyword, or by structural proximity.

Five operations compose on this substrate. The LLM **writes** — creating and editing memory files whose WikiLinks grow the graph. It **searches** — hybrid retrieval across the full corpus. It **traverses** — hopping graph edges to build context around a concept. It **reasons** — sequential thinking sessions that persist across invocations, support branching and revision, and themselves emit WikiLinks that feed the graph. It **orchestrates** — workflows compose these operations into structured methodologies; product pipelines can compose workflow nodes into DAGs. Do not read this as permission to launch PM/Gate coding workers through ACP or `AcpExec`; coding-task dispatch belongs to the host agent/task system.

What prevents the wiki from decaying into noise? Conditioning and convention, not enforcement. Roles, colors, and perspectives shape how the LLM frames a problem. Workflows provide procedural scaffolds. Constitutional principles — graph sovereignty, signal discipline, epistemic honesty — act as soft schema: they do not constrain what can be represented, but strongly shape what is worth writing. The system provides mechanism; the LLM provides judgment.

This works because of what maenifold deliberately omits. The graph carries no built-in semantics, no type system, no inference engine — it is dumb infrastructure the LLM reads through. An LLM with full context interprets a co-occurrence edge more usefully than any schema we could predefine. Maintenance is radically asymmetric: writing a WikiLink costs one bracketed phrase; the routing value it creates compounds across every future query by every future agent. The system tracks freshness; the LLM decides what matters. Every feature not built into the substrate is room for intelligence to emerge.

## Domains and Commands

`GatewayIndex` may expose more than the native maenifold domains because configured Gateway-backed servers can add their own live domains at runtime. The list below documents the native maenifold domains; use `GatewayIndex` to discover the full live set first. (`maenifold.gate` is documented in the separate `gate` skill.)

### maenifold.memory — knowledge files

| Command | What it does |
|---------|--------------|
| `WriteMemory` | Create a knowledge file. `content` MUST contain at least one `[[WikiLink]]`. |
| `ReadMemory` | Retrieve a file by `identifier` (URI or title). Returns checksum by default for safe edits. |
| `EditMemory` | Modify a file. Operations: `append`, `prepend`, `find_replace`, `replace_section`. Pass last-read `checksum` for stale-edit protection. |
| `DeleteMemory` | Delete a file. Requires `confirm=true`. |
| `MoveMemory` | Rename or relocate a file; WikiLinks are preserved. |
| `ListMemories` | List files or folders relative to the memory root. The folder structure is the ontology. |
| `SearchMemories` | Hybrid / Semantic / FullText ranking with optional weight knobs (`semanticWeight`, `textWeight`, `rrfK`, `semanticMinScore`). |
| `ExtractConceptsFromFile` | Return the WikiLinks in one file. |

### maenifold.graph — co-occurrence routing graph

| Command | What it does |
|---------|--------------|
| `BuildContext` | Traverse from a known concept anchor. `depth=1` for neighbors, higher to expand. `includeContent=true` returns previews. |
| `FindSimilarConcepts` | Semantic discovery, including concepts that don't exist yet (the query text is embedded). Use BEFORE writing to find existing variants. |
| `Visualize` | Mermaid or graph JSON output for a concept neighborhood. |
| `Sync` | Rebuild graph from WikiLinks on disk. |
| `AnalyzeConceptCorruption` | Diagnose a concept family for variants. Run before `RepairConcepts`. |
| `RepairConcepts` | Replace variants with a canonical form. **Always** call with `dryRun=true` first. |

### maenifold.thinking — persistent reasoning

| Command | What it does |
|---------|--------------|
| `SequentialThinking` | Persistent thought sessions. To start, omit `sessionId` and pass `thoughtNumber=0` — the server returns the new `sessionId`. Reuse it across turns and days. Supports `isRevision`, `branchId`, `submachineSessionId`. `response` MUST contain `[[WikiLinks]]`. |
| `AssumptionLedger` | Track assumptions tied to a workflow or thinking context. Actions: `append`, `update`, `read`. |

### maenifold.workflow — orchestration

| Command | What it does |
|---------|--------------|
| `Workflow` | Run a structured workflow with optional nested submachines. |
| `Pipeline` | Create / advance / cancel a DAG of workflow nodes with edges (`success`, `always`, `conditional`). Validates the graph and returns ready nodes. |

### maenifold.system — config, assets, ACP

| Command | What it does |
|---------|--------------|
| `GetConfig` | View resolved configuration and paths. |
| `MemoryStatus` | Corpus statistics and health. |
| `RecentActivity` | Recently indexed files, optionally with content. Filters: `thinking`, `memory`, `assumptions`, `all`. |
| `ListAssets` | List asset types or items: `workflow`, `role`, `color`, `perspective`. |
| `Adopt` | Load a role / color / perspective to condition reasoning. |
| `ListSessions` | List resumable ACP sessions across configured agents. |
| `AcpExec` | ACP runtime invocation for Maenifold UI / ACP sessions and ACP-specific smoke tests. Never use it to run Gate tasks, `task-pm` work, outer/inner-loop coding workers, or `GatewayIntent` fallback. |
| `ReadTextFile` | Read a UTF-8 text file (absolute or cwd-relative). |
| `ReadMcpResource` | Read MCP resource content by URI. (Also a top-level convenience tool.) |
| `EmbeddingModels` | Manage embedding models: `status`, `list`, `download`, `activate`, `delete`, `regenerate`. |
| `UpdateAssets` | Refresh runtime assets from packaged assets. Defaults `dryRun=true`. |
| `RunFullBenchmark` | Run search / sync / traversal benchmarks. |
| `Schedule` | Scheduled command execution. Use `learn=true` for details. |

### Help and discovery

Use `GetHelp` with a tool name (e.g. `GatewayExecute`, `WriteMemory`) for canonical per-tool documentation. Use `GatewayIndex` with `includeSchemas=true` to inspect exact parameter shapes for any command. Most commands also accept `learn=true` to return their help text instead of executing.

## Operating Stance

Your context resets between sessions. Treat the graph and `memory://` corpus as your real memory — internal model knowledge is untrusted, especially for anything that postdates training or concerns this repo's specifics. Before answering, search the graph. Before writing, search for what already exists. Before inventing a concept, look for a near-duplicate. The cost of a duplicate note is fragmentation of the routing index; the cost of a missing search is hallucination.

Your context will be auto-compacted as it approaches the limit. Do not stop work early to conserve tokens. Save progress to `memory://` as you approach the ceiling and rehydrate from those notes after compaction.

## Persistence of Thought

Sessions are ephemeral; the graph is not. Two mechanisms turn ephemeral compute into durable infrastructure:

**`SequentialThinking` for reasoning.** Every thought with `[[WikiLinks]]` becomes a node. Start a session by passing `thoughtNumber=0` and no session ID — the tool returns one. Use that session ID across multiple turns and across days. Use `isRevision` and `branchId` when you want to explore alternatives without losing the original line of thought.

**Session sharing with subagents.** You are the decision maker; subagents execute. When you spawn a subagent, pass it your thinking session ID so its thoughts feed the same graph. Embed `[[WikiLinks]]` in the dispatch prompt — the PreToolUse hook injects matching graph context automatically. The hook does retrieval; the graph grows through the thinking sessions themselves.

For non-trivial **knowledge-graph reasoning** tasks, use `Workflow` with the `workflow-dispatch` workflow. It analyzes the task and routes to an appropriate methodology. If the user says "think about X," reach for `workflow-dispatch`. For Gate-tracked coding tasks, do **not** use `Workflow`, `Pipeline`, or `AcpExec` as a worker launcher; use the host PM/task dispatch process for visible worktree lanes.

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
- **Context7** for library/framework APIs.
- **Microsoft Docs** for Microsoft / Azure concepts and code samples.
- Generic web search only when neither covers the topic.

This grounding requirement applies to all work, code or otherwise.

## Graph Navigation

Two complementary commands for concept exploration:

- **`BuildContext`** traverses graph relationships from a known anchor. Use `depth=1` for direct neighbors, `depth=2+` to expand. `includeContent=true` returns previews so you skip a separate `ReadMemory` round-trip.
- **`FindSimilarConcepts`** finds concepts by semantic similarity, including ones that don't exist yet (it embeds the query text). Use it to discover naming variants *before* writing — this is your primary defense against fragmenting the graph with near-duplicates.

Two patterns worth memorizing:

- **Chain pattern**: `FindSimilarConcepts` → pick the best anchor → `BuildContext` → `SearchMemories` for relevant files → `ReadMemory` on the highest-signal hits. E.g., when researching authentication, you might land on `[[JWT]]` as the anchor, traverse to `[[token-rotation]]` and `[[refresh-tokens]]`, then read the surfaced notes.
- **HYDE pattern**: Draft a hypothetical answer with `[[WikiLinks]]` inline — `[[vector-embeddings]]`, `[[BM25]]`, `[[reciprocal-rank-fusion]]` — then feed those concepts into `BuildContext` / `FindSimilarConcepts` / `SearchMemories`. The hypothesis seeds retrieval; the corpus corrects you.

Reading every plausible file blindly is worse than navigating the graph intentionally.

## Writing Signal, Not Noise

`WriteMemory` is how the wiki compounds. Most notes you consider writing should not be written. Write a note only when it is a high-quality, wiki-style article that future agents will benefit from retrieving. The test: would another agent searching this domain in three months be glad this note exists?

Before writing, always:

1. `SearchMemories` and `FindSimilarConcepts` to find an existing note to update instead.
2. `ListMemories` to find the right folder. **The folder structure is the ontology** — `azure/`, `finops/`, `tech/`, with sub-domains nested (`azure/billing/`, `tech/ml/`). Place notes where they will be discovered. Extend the structure only when a genuinely new domain emerges.
3. Choose `[[WikiLinks]]` that route. A good note about a deadlock fix might link `[[deadlock]]`, `[[mutex]]`, `[[tokio-runtime]]`, `[[async-rust]]` — substantive concepts an agent in the same domain would search for.

For thinking, `SequentialThinking` is your episodic record. Use it to capture reasoning, document decisions, branch through alternatives, and note what worked and what didn't. The graph absorbs both kinds of writing — articles and thoughts — and the routing improves either way.

## WikiLink Anti-Patterns

`[[WikiLinks]]` are graph nodes. Bad nodes corrupt the routing index for every future agent. The system filters the worst offenders via `.wikilink-filter.json`, but you should not rely on that — write good concepts in the first place.

Real failure modes observed in the corpus:

- **Prompt leakage**: `[[load-the-gate-skill-and-sync-the-gate-cache-in-users-brett-src-ma-collective-ma]]`, `[[sequential-thinking-required-the-blue-hat-opens-every-six-thinking-hats-session]]`. Sentences and instructions wrapped in brackets. Concepts are nouns or short noun phrases, not directives.
- **God nodes**: `[[software-tool]]` (7,438 edges), `[[testing-tool]]` (6,820), `[[rust]]` (601 files). When a concept connects to everything, it routes to nothing — `BuildContext` returns noise. Prefer specifics: `[[cargo-workspace]]`, `[[clippy-lint]]`, `[[tokio-runtime]]`.
- **Code symbols**: `[[AgentPool]]`, `[[PipelineComposer]]`, `[[WorkflowStepTimeline]]`. Implementation details, not domain concepts. Tag the *behavior* (`[[agent-pooling]]`, `[[workflow-orchestration]]`), not the struct name.
- **Trivial tags**: `[[tool]]`, `[[memory]]` (531 files), `[[concept]]`, `[[file]]`, `[[the]]`. Too generic to be useful retrieval anchors.
- **Cosmetic violations** (silently normalized — still avoid): underscores `[[my_concept]]`, slashes `[[foo/bar]]`, double hyphens, leading/trailing hyphens, `[single-bracket]`.

Good concept conventions: double brackets, singular for the general concept (`[[agent]]`), plural only for the collection meaning (`[[tools]]`), primary noun not qualifier compound (`[[MCP]]` over `[[MCP-server]]`), `FindSimilarConcepts` first to reuse what exists.

Example of a well-tagged sentence: `Fixed [[null-reference-exception]] in [[authentication]] flow by hardening [[JWT]] validation.`

## Invocation Cheat Sheet

Conceptual call, then the literal MCP request. Parameter spelling matters — when in doubt run `GatewayIndex` with `includeSchemas=true` for the target domain.

**Search the corpus**

```text
GatewayExecute
  domain="maenifold.memory"
  command="SearchMemories"
  parameters={"query": "azure savings plan", "mode": "Hybrid", "pageSize": 5}
```

**Find concept variants before writing**

```text
GatewayExecute
  domain="maenifold.graph"
  command="FindSimilarConcepts"
  parameters={"conceptName": "savings-plan", "maxResults": 10}
```

**Traverse the graph from an anchor**

```text
GatewayExecute
  domain="maenifold.graph"
  command="BuildContext"
  parameters={"conceptName": "JWT", "depth": 2, "includeContent": true}
```

**Write a memory file**

```text
GatewayExecute
  domain="maenifold.memory"
  command="WriteMemory"
  parameters={
    "title": "Azure Savings Plan refresh policy",
    "folder": "azure/billing",
    "content": "Decision to use [[savings-plan]] over [[reservation]] for [[compute-fleet]]..."
  }
```

**Start (or continue) a thinking session**

```text
# start: omit sessionId, pass thoughtNumber=0, get sessionId back
GatewayExecute
  domain="maenifold.thinking"
  command="SequentialThinking"
  parameters={
    "thoughtNumber": 0,
    "totalThoughts": 5,
    "nextThoughtNeeded": true,
    "response": "First thought touching [[concept-A]] and [[concept-B]]..."
  }

# continue: reuse sessionId, increment thoughtNumber
GatewayExecute
  domain="maenifold.thinking"
  command="SequentialThinking"
  parameters={
    "sessionId": "<returned-id>",
    "thoughtNumber": 1,
    "nextThoughtNeeded": true,
    "response": "Building on [[concept-A]] with [[concept-C]]..."
  }
```

**Inspect what's available**

```text
GatewayIndex                                           # all domains
GatewayIndex parameters={"domain": "maenifold.graph"}  # one domain's commands
GatewayIndex parameters={"domain": "maenifold.memory", "includeSchemas": true}  # exact schemas
GetHelp parameters={"toolName": "WriteMemory"}        # canonical per-tool help
```

---

## Verification

Verified against commit `aa7df44a` on `2026-04-28` via `make audit-parity` (see `docs/parity-uat.md`).
