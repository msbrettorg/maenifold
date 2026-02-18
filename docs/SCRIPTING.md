# Maenifold Search & Scripting

Maenifold combines a concept graph, embeddings, and agent workflows to support:

- **Search & Retrieval**: hybrid semantic/text search over concept-linked memories
- **Graph-Based Reasoning**: multi-hop context building on a graph of `[[WikiLinks]]`
- **Test-Time Adaptation**: roles, colors, and workflows that change how the model reasons
- **Programmable Orchestration**: CLI- and MCP-exposed tools that can be scripted and composed

This document is the primary reference for Maenifold search and scripting: the conceptual model, the RAG techniques it supports, and concrete CLI patterns for composing the tools.

---

## 1. Conceptual Model

### 1.1. From Storage to Test-Time Adaptive Reasoning

Maenifold isn't just knowledge storage. It is **test-time adaptive reasoning infrastructure**:

1. **RAG on Graph of Thought**
	 - Retrieval is augmented by **concept relationships**, not just vector similarity.
	 - Concept graphs encode how ideas relate and co-occur across files.

2. **Test-Time Adaptation**
	 - Roles, colors, perspectives, and workflows are **cognitive mode switches**.
	 - They change *how* the model reasons over the same graph (not just tone).

3. **Agent Bootstrapping**
	 - Agents/subagents are spawned with **preloaded graph context**.
	 - Domain-specific roles + curated context yield domain-specific reasoning.

4. **Composable Intelligence**
	 - CLI + MCP tools are composable primitives.
	 - Workflows and scripts layer higher-order patterns on top.

### 1.2. Primitive Layers

Maenifold exposes a layered set of primitives you can call from agents, workflows, or the CLI.

| Layer | Primitives | Purpose |
|-------|------------|---------|
| **Memory** | `WriteMemory`, `ReadMemory`, `EditMemory`, `DeleteMemory`, `MoveMemory`, `Sync` | Persist knowledge with `[[WikiLinks]]` |
| **Graph/Search** | `SearchMemories`, `BuildContext`, `FindSimilarConcepts`, `Visualize` | Query and traverse knowledge |
| **Session** | `RecentActivity`, `AssumptionLedger`, `ListMemories` | Track work, state, and uncertainty |
| **Persona** | `Adopt` (roles, colors, perspectives) | Condition LLM behavior |
| **Reasoning** | `SequentialThinking` | Multi-step thought with revision and reflection |
| **Orchestration** | `Workflow` | State machines over steps and tools |
| **System** | `GetConfig`, `GetHelp`, `MemoryStatus` | Introspection and diagnostics |

These primitives are intentionally small. Complexity emerges from **composition** (workflows, sequential sessions, and scripts).

---

## 2. Asset Types: Roles, Colors, Workflows

### 2.1. Roles

Roles encode domain expertise and perspective (e.g. `ai-researcher`, `architect`, `mcp-specialist`, `product-manager`, `red-team`, `blue-team`, etc.).

- Adopt via `Adopt` or workflow configuration.
- Influence:
	- What evidence is considered important
	- What risks or opportunities are prioritized
	- How recommendations are framed

### 2.2. Colors (Six Thinking Hats + Gray)

Colors are cognitive modes:

- **White** – Facts & information
- **Red** – Emotions & intuition
- **Black** – Critical thinking & risks
- **Yellow** – Positive benefits & value
- **Green** – Creative alternatives and ideas
- **Blue** – Orchestrator & process control
- **Gray** – Skeptical inquiry & challenge assumptions

Experiments show that colors applied to the same concept graph yield **different cognitive strategies**:

- Green hat on `maenifold` – generative "what if" scenarios
- Yellow hat – benefit/value narratives
- Black hat – risk and integrity analysis

### 2.3. Workflows

Workflows are declarative JSON state machines stored under `assets/workflows/` and hot-reloaded via MCP.

Examples:

- `agentic-research` – single-agent deep research with HyDE, reflexion, and information gain checks.
- `think-tank` – multi-agent orchestration in waves (scoping, deep dives, synthesis, review).
- `sixhat` – cycles an agent through color modes.
- Domain-specific flows (FinOps, game theory, etc.).

Composition rules:

- Workflows can **call other workflows**.
- Many steps are marked `requiresEnhancedThinking` and embed `SequentialThinking` sessions.
- Sequential sessions themselves can decide to dispatch a workflow when systematic analysis is needed.

Assets are writable from agents, so **new workflows are agent-generated skills**:

> Agent writes JSON → new workflow appears under `assets/workflows/` → becomes immediately runnable via MCP and CLI.

---

## 3. Search & RAG Capabilities

### 3.1. RAG Technique Support Matrix

| Technique | Support | Implementation | Notes |
|-----------|---------|----------------|-------|
| **Classic RAG** | ✓ Native | `SearchMemories (Semantic)` | 384-dim embeddings |
| **Knowledge Graph RAG** | ✓ Native | `BuildContext` + `SearchMemories` | Core differentiator |
| **Multi-Hop Traversal** | ✓ Native | `BuildContext (depth=N)` | Saturation at depth 2–3 |
| **Reranking (RRF)** | ✓ Native | `SearchMemories (Hybrid)` | RRF k=60 built-in |
| **Query Expansion** | ✓ Native | `FindSimilarConcepts` + `BuildContext` | Semantic + graph |
| **RAG with Memory** | ✓ Native | Persistent graph + `RecentActivity` | Architecture IS memory |
| **Adaptive/Self-RAG** | ✓ Native | MCP architecture | Agent decides when/what to retrieve |
| **Agentic Retrieval** | ✓ Native | Full tool suite | Multi-tool orchestration |
| **Self-RAG/Reflective** | ✓ Native | `SequentialThinking` + `AssumptionLedger` | Revision + verification |
| **Routing** | ✓ Native | Agent + `Workflow` | Query classification |
| **DSP (Few-shot)** | ✓ Native | Examples as memories | Retrieve similar patterns |
| **FLARE (Proactive)** | ✓ Native | `session_start.sh` pattern | Forward-looking retrieval |
| **Hierarchical Chunks** | ✓ Native | Concepts → Files | Two-level implicit |
| **Multi-step RAG** | ✓ Composable | Search → extract → refine | Script loop |
| **HYDE** | ✓ Scriptable | LLM → extract `[[WikiLinks]]` → search | Generate hypothetical doc |
| **RAG-Fusion** | ⚠️ Scriptable | Parallel queries + dedupe | Multi-query strategy |
| **Iterative (ITER-RETGEN)** | ⚠️ Scriptable | Loop until convergence | Needs termination logic |
| **CRAG (Corrective)** | ⚠️ Scriptable | Score filter + retry | Correction loop |
| **Cross-encoder Rerank** | ⚠️ Expensive | LLM per doc | RRF sufficient for most |
| **FiD** | ⚠️ Partial | Retrieval only | Fusion is LLM concern |
| **Multi-modal** | ❌ No | Text-only | Design decision |
| **Sub-file Chunking** | ❌ No | File-level retrieval | Trade-off: context preservation |
| **Contextual Embeddings** | ❌ No | Fixed embedding pipeline | Would need model change |
| **Recency Weighting** | ⚠️ Partial | `RecentActivity` exists | Not in search ranking |

Patterns with loops and complex stopping conditions (RAG-Fusion, CRAG, iterative RAG) are **intentionally left to workflows and scripts**, not core engine logic.

### 3.2. Graph-Based RAG (RAG on Graph of Thought)

Maenifold's core retrieval pattern:

1. **Search** – use `SearchMemories` in Hybrid mode for rich scores.
2. **Expand** – use `BuildContext` on key concepts to traverse the concept graph.
3. **Curate** – filter by scores, relationship strength, and recency (where relevant).
4. **Adapt** – run reasoning with a chosen role/color/workflow over the resulting context.

The graph isn't just storage – it's the **reasoning substrate** that shapes what context is retrieved and how it can be extended.

---

## 4. Integration Patterns

Maenifold's search and scripting patterns are embedded into each integration under `integrations/`. They all implement the same core ideas (Graph-RAG, HYDE-style hypothetical retrieval, FLARE-style proactive context loading), but at different layers:

- **Claude Code** (`integrations/claude-code/`)
	- Shell hook (`session_start.sh`) runs at **session start**.
	- Pattern: FLARE-style proactive retrieval.
		- Query `RecentActivity` → extract top `[[WikiLinks]]` → `BuildContext` → inject ~5K tokens of graph-derived context into the new Claude session.
	- Result: Claude never starts "cold"; it always sees a curated slice of the graph and recent work as preamble.

These integrations are examples of **where to plug the patterns in**:

- Session hooks (Claude) → FLARE-style proactive context.

When you add new integrations, align them with these patterns instead of inventing bespoke behavior.

---

## 5. Scripting with the CLI

The CLI treats Maenifold as a black-box API and exposes all the primitives above. Unix tools (grep/awk/sed/sort/uniq/comm) provide post-processing.

**Quick setup for scripts**

```bash
# Set an isolated root for experiments
export MAENIFOLD_ROOT=/tmp/maenifold-cli-demo
mkdir -p "$MAENIFOLD_ROOT"

# Helper for readable JSON payloads
run() { maenifold --tool "$1" --payload "$2"; }
```

### 5.1. Pattern: Graph-Augmented Search

Combine search with concept-level context expansion:

```bash
# Search + context expansion
RESULTS=$(maenifold --tool SearchMemories --payload '{"query":"authentication","mode":"Hybrid","pageSize":20}')

# Extract memory URIs from the search results
FILES=$(echo "$RESULTS" | grep -oE "memory://[^)]*")

# Extract concepts from those files and build context around them
for URI in $FILES; do
	CONCEPTS=$(maenifold --tool ExtractConceptsFromFile --payload "{\"identifier\":\"$URI\"}" | \
		grep -oE "\[\[[^]]+\]\]" | sed 's/\[\[\(.*\)\]\]/\1/')

	for CONCEPT in $CONCEPTS; do
		maenifold --tool BuildContext --payload "{\"conceptName\":\"$CONCEPT\",\"depth\":1}"
	done
done
```

Use cases:

- Explore a topic beyond keyword matches.
- Discover bridge concepts connecting different clusters.

### 5.2. Pattern: Concept Network Visualization

Build a concept neighborhood and visualize it via graph tools:

```bash
# Build context + visualize
maenifold --tool BuildContext --payload '{"conceptName":"maenifold","depth":2}' | \
	maenifold --tool Visualize --payload '{"conceptName":"maenifold"}'
```

### 5.3. Pattern: Batch Processing with Filtering

Process recent work and filter locally:

```bash
maenifold --tool RecentActivity --payload '{"limit":50}' | \
	grep '\[\[authentication\]\]' | \
	head -10
```

### 5.4. Pattern: Score-Aware Search Filtering

Hybrid search output includes a breakdown of scores that can be filtered with `awk`:

```bash
# Filter hybrid results by semantic score > 0.7
maenifold --tool SearchMemories --payload '{"query":"auth","mode":"Hybrid","pageSize":50}' | \
	awk '
		/^📄 / { title = $0; next }
		/Scores - Fused:/ {
			sem_start = index($0, "Semantic: ") + 10
			semantic = substr($0, sem_start)
			if (semantic+0 > 0.70) {
				print title
				print $0
				print ""
			}
		}
	'
```

Observations:

- **Hybrid mode** is key: provides Fused, Text, and Semantic scores.
- **Semantic score** is most useful for relevance; Text score for literal keyword matches.
- Filtering in the CLI reduces token load for downstream agents.

### 5.5. Pattern: Concept Co-Occurrence Analysis

Discover conceptual neighborhoods and architecture layers:

```bash
# 1. Search for files
FILES=$(maenifold --tool SearchMemories --payload '{"query":"maenifold","pageSize":20}' | \
	grep -oE "memory://[^)]*")

# 2. Extract concepts from each file and count frequency
for file in $FILES; do
	maenifold --tool ExtractConceptsFromFile --payload "{\"identifier\":\"$file\"}" | \
		grep -oE "\[\[[^]]+\]\]" | sed 's/\[\[\(.*\)\]\]/\1/'
done | \
	sort | uniq -c | sort -rn
```

This surfaces:

- Dominant concepts in a domain.
- Co-occurrence patterns (when extended to pairs) that reveal architecture layers:
	- Core system concepts
	- Orchestration (sequential thinking, workflows)
	- Knowledge infrastructure (graph, embeddings)

### 5.6. Pattern: Multi-Hop Discovery

Use `BuildContext` depth parameter for multi-hop graph exploration:

```bash
# Breadth-first multi-hop discovery
maenifold --tool BuildContext --payload '{"conceptName":"authentication","depth":2}'
```

Empirical behavior:

- `depth=1` – local neighborhood (~20 neighbors).
- `depth=2` – extended network (~40+ concepts), captures most of the domain.
- `depth>=3` – saturation: few new concepts; more repetition/noise.

### 5.7. Pattern: Subagent Bootstrapping

Preload graph context before spawning a subagent:

```bash
CONCEPT="mcp-protocol"
ROLE="mcp-specialist"
TASK="analyze implementation patterns"

# 1. Build graph context
CONTEXT=$(maenifold --tool BuildContext --payload "{\"conceptName\":\"$CONCEPT\",\"depth\":2,\"maxEntities\":10}")

# 2. Extract related concepts
RELATED=$(echo "$CONTEXT" | grep -oE "\[\[[^]]+\]\]" | sed 's/\[\[\(.*\)\]\]/\1/' | sort -u)

# 3. Format bootstrap prompt
BOOTSTRAP_PROMPT="Concept: $CONCEPT
Related concepts: $RELATED
Task: $TASK
Role: $ROLE
"

# 4. Spawn subagent with this prompt (pseudo-call)
# maenifold --tool Task --payload "{\"prompt\":\"$BOOTSTRAP_PROMPT\"}"
```

Result:

- Subagents start with ~3–5K curated tokens of domain context instead of raw, unfiltered text.
- This pattern mirrors Anthropic's RAG + code-execution pattern, but used for **cognitive bootstrapping**.

### 5.8. Pattern: End-to-End Retrieval + Synthesis (happy path)

```bash
# 1) Search
SEARCH=$(run SearchMemories '{"query":"authentication","mode":"Hybrid","pageSize":10}')

# 2) Pull a top URI and build context
TOP_URI=$(echo "$SEARCH" | grep -oE "memory://[^)]*" | head -1)
CONCEPTS=$(run ExtractConceptsFromFile "{\"identifier\":\"$TOP_URI\"}" | grep -oE "\[\[[^]]+\]\]" | sed 's/\[\[\(.*\)\]\]/\1/' | head -5)
for C in $CONCEPTS; do
  run BuildContext "{\"conceptName\":\"$C\",\"depth\":1}"
done

# 3) Write a synthesized note (include [[WikiLinks]]), then sync
run WriteMemory "{\"title\":\"Auth synthesis\",\"content\":\"Linking [[authentication]] and [[session-management]]\"}"
run Sync "{}"
```

### 5.9. Pattern: Failure modes worth testing

These are common tripping points (exact errors come from the tools):

- Missing `[[WikiLinks]]` in write/edit/sequentialthinking content → `ERROR: Must include [[WikiLinks]]...`
- `SequentialThinking` with `nextThoughtNeeded=false` but no `conclusion` → conclusion required
- `SequentialThinking` branching without `branchId` → branchId required
- `MoveMemory` that drops extension (regression guard) → verify extension preserved after move
- `BuildContext` on unknown concept → returns empty neighborhood (not an error); follow with `FindSimilarConcepts`

Script these checks up front in CI-like smoke tests when integrating Maenifold into automation.

## 11. Tested CLI flows (Dec 22, 2025, using ~/maenifold/bin/maenifold)

Environment:

```bash
export MAENIFOLD_ROOT=/tmp/maenifold-cli-demo-test
rm -rf \"$MAENIFOLD_ROOT\" && mkdir -p \"$MAENIFOLD_ROOT\"
BIN=~/maenifold/bin/maenifold
```

Happy path:

- WriteMemory (with [[WikiLinks]]):
  - `Auth Note` content: `Testing [[authentication]] flow with [[session-management]]`
  - `RAG Patterns` content: `Hybrid search for [[authentication]] plus [[graph-rag]] expansion.`
- Sync:
  - 2 files, 3 concepts, 2 relations; embeddings generated.
- SearchMemories (Hybrid, query=authentication):
  - Returned both memories; semantic score 1.0 on `Auth Note`.
- ExtractConceptsFromFile (memory://auth-note):
  - [[authentication]], [[session-management]]
- BuildContext (authentication, depth=1, maxEntities=10):
  - Related concepts: graph-rag (memory://rag-patterns), session-management (memory://auth-note)
- Visualize (authentication):
  - Mermaid graph with edges to graph_rag and session_management.
- RecentActivity (limit=5):
  - Shows both memories with timestamps.

Notes:
- All commands executed without build since an existing binary was used; root cleanliness check was avoided by not invoking `dotnet run`.
- Errors to expect in this flow if misused:
  - Missing [[WikiLinks]] in WriteMemory/SequentialThinking → concept validation error.
  - Missing conclusion when nextThoughtNeeded=false in SequentialThinking → conclusion required.
  - Branching in SequentialThinking without branchId → branchId required.

## 12. Prod CLI validation (Dec 22, 2025, default MAENIFOLD_ROOT)

Binary: `~/maenifold/bin/maenifold` (no env override; memory path `~/maenifold/memory`).

- GetConfig: confirmed prod paths (memory/db under `~/maenifold/`).
- SearchMemories Hybrid (`"maenifold"`, pageSize 3): returned 49 matches; sample entries under `memory://research/context/*`.
- BuildContext (`maenifold`, depth 1, maxEntities 5): neighbors `rag-fusion`, `information-gain`, `context`, `persona-conditioning`, `recency` with file lists.
- Visualize (`maenifold`): mermaid edges among those concepts (high co-occurrence counts).
- SequentialThinking validation errors (no files written):
  - Missing [[WikiLinks]] → `ERROR: Must include [[WikiLinks]]...`
  - Branching without branchId → `ERROR: branchId required when branchFromThought is specified...`
  - Providing `sessionId` on thoughtNumber=1 for a non-existent session triggered a runtime crash (Signal 6) instead of a friendly error.
- SequentialThinking happy path (writes to prod, then cleaned):
  - Thought 1 with `[[cli-test]]` auto-created session `session-1766422076688` at `memory://thinking/sequential/2025/12/22/session-1766422076688`
  - Thought 2 with conclusion + `[[cli-test]]` completed session.
  - Deleted test session via DeleteMemory `{"identifier":"memory://thinking/sequential/2025/12/22/session-1766422076688","confirm":true}`

Notes:
- SequentialThinking storage includes year/month/day in the path.
- The Signal 6 crash on invalid sessionId (thoughtNumber=1) is a bug; expected a user-facing error string.

## 13. Scripts for the 3.1 Technique Matrix

Use `maenifold` CLI; assumes `BIN=maenifold` is on PATH (prod root).

**Classic RAG (Semantic)**
```bash
$BIN --tool SearchMemories --payload '{"query":"authentication","mode":"Semantic","pageSize":20}'
```

**Knowledge Graph RAG (Search + BuildContext)**
```bash
TOP=$( $BIN --tool SearchMemories --payload '{"query":"authentication","mode":"Hybrid","pageSize":5}' \
  | grep -oE "memory://[^)]*" | head -1 )
CONCEPTS=$( $BIN --tool ExtractConceptsFromFile --payload "{\"identifier\":\"$TOP\"}" \
  | grep -oE "\[\[[^]]+\]\]" | sed 's/\[\[\(.*\)\]\]/\1/' | head -3 )
for C in $CONCEPTS; do
  $BIN --tool BuildContext --payload "{\"conceptName\":\"$C\",\"depth\":1}"
done
```

**Multi-Hop Traversal**
```bash
$BIN --tool BuildContext --payload '{"conceptName":"maenifold","depth":2}'
```

**Reranking (Hybrid/RRF)**
```bash
$BIN --tool SearchMemories --payload '{"query":"recency decay","mode":"Hybrid","pageSize":10}'
# Use fused/text/semantic scores from output to filter locally if needed
```

**Query Expansion (FindSimilarConcepts)**
```bash
$BIN --tool FindSimilarConcepts --payload '{"conceptName":"recency","limit":10}'
```

**RAG with Memory (RecentActivity)**
```bash
$BIN --tool RecentActivity --payload '{"limit":20}' | head -50
```

**Adaptive/Self-RAG (workflow/agent-driven)**
- Pattern: embed `SequentialThinking` in your loop; call `SearchMemories` when `nextThoughtNeeded` is set. (See SequentialThinking doc.)

**Agentic Retrieval (multi-tool)**
```bash
RES=$( $BIN --tool SearchMemories --payload '{"query":"workflow","mode":"Hybrid","pageSize":5}' )
URIS=$(echo "$RES" | grep -oE "memory://[^)]*")
for U in $URIS; do $BIN --tool ReadMemory --payload "{\"identifier\":\"$U\"}" | head -40; done
```

**Self-RAG/Reflective (SequentialThinking + AssumptionLedger)**
```bash
$BIN --tool SequentialThinking --payload '{"response":"Investigating [[rag-fusion]] gaps","thoughtNumber":1,"totalThoughts":2,"nextThoughtNeeded":true}'
$BIN --tool AssumptionLedger --payload '{"assumption":"RRF k=60 is adequate for [[rag-fusion]]","action":"record"}'
```

**Routing (Workflow)**
```bash
$BIN --tool Workflow --payload '{"workflowId":"agentic-research","response":"[[rag-fusion]] parameter tuning"}'
```

**DSP / Few-shot via memories**
```bash
$BIN --tool SearchMemories --payload '{"query":"example","mode":"Hybrid","pageSize":5}'
```

**FLARE-style proactive context**
```bash
$BIN --tool RecentActivity --payload '{"limit":20}' \
  | grep -oE "\[\[[^]]+\]\]" | sed 's/\[\[\(.*\)\]\]/\1/' | sort -u | head -5 \
  | while read C; do $BIN --tool BuildContext --payload "{\"conceptName\":\"$C\",\"depth\":1}"; done
```

**Multi-step RAG loop (simple)**
```bash
QUERY="authentication retry"
RES=$($BIN --tool SearchMemories --payload "{\"query\":\"$QUERY\",\"mode\":\"Hybrid\",\"pageSize\":5}")
URIS=$(echo "$RES" | grep -oE "memory://[^)]*")
for U in $URIS; do
  $BIN --tool ExtractConceptsFromFile --payload "{\"identifier\":\"$U\"}" | grep -oE "\[\[[^]]+\]\]"
done | sort -u | head -5 | while read C; do
  $BIN --tool BuildContext --payload "{\"conceptName\":\"${C//[\[\]]/}\",\"depth\":1}"
done
```

**HYDE (hypothetical doc then search)**
```bash
HYPOTHESIS="Hypothetical [[zero-downtime]] deployment doc for [[authentication]] service."
$BIN --tool WriteMemory --payload "{\"title\":\"Hyde Note\",\"content\":\"$HYPOTHESIS\"}"
$BIN --tool Sync --payload '{}'
$BIN --tool SearchMemories --payload '{"query":"zero-downtime authentication","mode":"Hybrid","pageSize":10}'
```

**RAG-Fusion (parallel queries + dedupe)**
```bash
for Q in "authentication timeout" "authentication retry" "authentication backoff"; do
  $BIN --tool SearchMemories --payload "{\"query\":\"$Q\",\"mode\":\"Hybrid\",\"pageSize\":10}"
done | grep -oE "memory://[^)]*" | sort -u
```

**Iterative (ITER-RETGEN)**
```bash
for i in 1 2 3; do
  $BIN --tool SearchMemories --payload "{\"query\":\"authentication iteration $i\",\"mode\":\"Hybrid\",\"pageSize\":5}"
done
```

**CRAG-style corrective loop (manual)**
```bash
RAW=$($BIN --tool SearchMemories --payload '{"query":"authentication errors","mode":"Hybrid","pageSize":20}')
echo "$RAW" | awk '/Semantic:/ { sem=$NF+0; if (sem>0.5) keep=1; else keep=0 } /^📄/ {if(keep) print prev; prev=$0}'
```

**Cross-encoder rerank (LLM-as-judge placeholder)**
- Export top N URIs from Hybrid search, send to your LLM reranker outside the CLI; feed reranked order back to downstream steps.

**FiD (retrieve only)**
```bash
$BIN --tool SearchMemories --payload '{"query":"fuse context","mode":"Hybrid","pageSize":10}'
# Fusion is performed by your model; CLI retrieves.
```

For unsupported/partial techniques (multi-modal, sub-file chunking, contextual embeddings, recency weighting in ranking), the CLI does not provide direct support—compose using existing tools or external processing.

---

## 14. Tested outputs for section 3.1 scripts (Dec 22, 2025, prod MAENIFOLD_ROOT)

Binary: `~/maenifold/bin/maenifold` (no env override; memory path `~/maenifold/memory`).

- Semantic search (`authentication`, mode=Semantic): 30 matches; top `memory://finops/research/finops-framework-overview` (semantic 1.0).
- Graph RAG script (Hybrid → ExtractConcepts → BuildContext): top URI `memory://finops/research/finops-framework-overview`; concept `finops`; neighbors include `finops-toolkit`, `azure`, `cost-optimization`.
- Multi-hop (BuildContext depth=2 on `maenifold`): core neighbors plus extended concepts (`routing`, `rrf`, `subquery-decomposition`).
- Hybrid search (`recency decay`): returns context-skill memories with fused/text/semantic scores.
- FindSimilarConcepts (`recency`): returns similar concepts (similarity 1.0) such as `workflows`, `sqlite`, `wikilinks`, `toolregistry`.
- RecentActivity (limit 3): shows latest workflow/sequential/memory items.
- Workflow routing (`agentic-research`): session created and first step returned; deleted afterward.
- SequentialThinking (happy path): session created/completed with conclusion under `memory://thinking/sequential/2025/12/22/...`; deleted. Errors confirmed for missing [[WikiLinks]] and missing `branchId`. Invalid `sessionId` on thought 1 still crashes (Signal 6 bug).
- AssumptionLedger: requires `action` in {append, update, read} and `concepts` array; append succeeded, then deleted.
- HYDE example: WriteMemory `HYDE CLI Test` with `[[zero-downtime]]` + Sync + Hybrid search surfaced it at semantic 1.0; deleted.
- RAG-Fusion sample (parallel queries on authentication timeout/retry/backoff): Hybrid searches completed; URIs deduped (e.g., `memory://finops/research/finops-framework-overview`).
- All temporary artifacts (SequentialThinking session, Workflow session, HYDE note, AssumptionLedger entry) were removed via DeleteMemory after testing.

## 6. Test-Time Adaptive Reasoning

Experiments on the same concept graph show that roles and colors act as **interpretive lenses** over the same data.

### 5.1. Role- and Color-Based Reasoning

**Setup**: Three parallel subagents analyze the same isolated concept (`code-execution`, 0 relations):

- Agent 1: `ai-researcher` role
- Agent 2: `black` (critical thinking) color
- Agent 3: Baseline (no role/color)

**Results**:

| Agent | Mode | Same Data? | Different Analysis? |
|-------|------|------------|---------------------|
| 1 | AI Researcher | Yes (0 relations) | Methodological: evidence quality, validation steps, reproducibility |
| 2 | Black Hat | Yes (0 relations) | Risk-focused: vulnerabilities, operational issues, undefined constraints |
| 3 | Baseline | Yes (0 relations) | Neutral: factual observation, semantic clustering |

The raw graph data is identical, but reasoning differs:

- AI Researcher: "Evidence quality low, need validation methodology."
- Black Hat: "Security risk, undefined constraints, operational danger."
- Baseline: "Isolated concept with semantic similarities."

### 5.2. Six Hats on a Connected Concept

On the `maenifold` concept (~52 connected concepts):

- Green hat – creative opportunities: knowledge mutation engine, cross-domain pattern transfer, emergent role discovery, "what if" scenarios.
- Yellow hat – value focus: cost governance, enterprise readiness, knowledge leverage.
- Black hat – risk analysis: identity crisis & scope creep, unvalidated production claims, graph integrity risks, maintenance burden.

Same 52-concept graph, different cognitive strategies:

- Creative → generative possibilities
- Positive → value affirmation
- Critical → risk identification

**Implication**: roles/colors/workflows are not cosmetic. They change:

- What questions are asked
- What risks and opportunities are noticed
- What connections are considered salient

---

## 7. Workflow Implementations

### 6.1. `agentic-research` (Single-Agent Deep Research)

Implements a structured research process:

| Step | Technique | Implementation |
|------|-----------|----------------|
| Research Initiation | Query Expansion | Coverage vector + research questions |
| Knowledge Baseline | Graph-RAG | `SearchMemories` + `BuildContext` |
| HyDE Expansion | HYDE | Generate hypothetical docs → extract queries |
| Information Gathering | Multi-step | Web search with relevance scoring |
| Topic Discovery | Adaptive | Emergent theme detection |
| Multi-Perspective | STORM-lite | Multiple personas analyze findings |
| Synthesis & Reflexion | Self-RAG | Six Thinking Hats + gap analysis |
| Knowledge Integration | Graph-RAG | Write memories, sync graph |
| Information Gain | CRAG-loop | Additional cycles if gain < threshold |

### 6.2. `think-tank` (Multi-Agent Orchestration)

Implements a collaborative research process in waves:

| Wave | Agents | Purpose | Techniques |
|------|--------|---------|------------|
| Charter | 1 (Director) | Plan, decompose, set quality gates | Query Expansion, Routing |
| Wave 1 | 4 parallel | Domain scoping, gaps, trends, literature | RAG-Fusion, Graph-RAG |
| Wave 2 | 3–5 dynamic | Deep domain research | Nested `agentic-research` |
| Wave 3 | 4 parallel | Cross-domain synthesis, innovations | Multi-Hop, Reflexion |
| Wave 4 | 4 parallel | Peer review, validation | Self-RAG, CRAG |
| Integration | 1 (Director) | Graph sync, final synthesis | Graph-RAG, Visualize |

Composition hierarchy:

```text
think-tank
	└── agentic-research (Wave 2 deep dives)
				└── SequentialThinking (most steps require enhanced thinking)
```

---

## 8. CLI-Only Pattern Summary

### 7.1. What Works Well

| Pattern | Validated | Performance | Use Case |
|---------|-----------|-------------|----------|
| Iterative Context | ✅ | ~50ms/concept (approximate, based on local testing) | Explore relationships |
| Score Filtering | ✅ | Instant | Quality filtering |
| Co-Occurrence | ✅ | ~17s/15 files (approximate, based on local testing) | Topic clustering |
| Multi-Hop | ✅ | ~100ms (depth=2, approximate, based on local testing) | Network discovery |

### 7.2. What Doesn't Fit the CLI Boundary

- Custom graph algorithms (centrality, PageRank)
- Direct file content analysis (AST processing)
- Temporal analysis (concept evolution)
- Large-scale batch processing (>1000 files)
- Complex SQL analytics

For those, use direct access to the underlying storage or specialized analysis tools.

---

## 9. Practical Guidance

### 8.1. Modeling Knowledge

- Use `[[WikiLinks]]` consistently in all knowledge files.
- Avoid orphaned concepts with 0 relations; they don't benefit from the graph.
- Expect **depth=2** traversals to hit the core of most domains.

### 8.2. Using Search Effectively

- Prefer `SearchMemories` in **Hybrid** mode for analysis and scripting.
	- Gives access to text and semantic scores plus the fused RRF score.
- Use CLI filtering (awk/grep) to downselect to high-relevance results before passing to agents.
- Use `BuildContext` for multi-hop, concept-centric exploration.

### 8.3. Designing Workflows

- Encode repeated RAG patterns (RAG-Fusion, CRAG, HyDE loops) as workflows, not ad-hoc prompts.
- Embed `SequentialThinking` in steps where deeper reasoning or reflection is required.
- Use `Adopt` to set roles/colors explicitly per step for deliberate cognitive modes.

### 8.4. CLI vs Direct Access

- **Use CLI-only** when:
	- Integrating with external tools (MCP, shell scripts).
	- You want a stable, language-neutral API.
	- You're exploring <100 concepts or a small file set.

- **Use direct access (DB/files)** when:
	- Running large-scale analytics (hundreds/thousands of files).
	- Needing advanced graph algorithms (centrality, PageRank).
	- Doing AST-level analysis or complex SQL queries.

---

## 10. Summary

- Maenifold provides:
	- A **graph-aware RAG engine** (concepts + embeddings + RRF).
	- A **persona + workflow layer** for test-time cognitive adaptation.
	- A **CLI + MCP interface** for scripting and integration.
- Most state-of-the-art RAG techniques are either:
	- **Native** to the architecture, or
	- **Composable** via workflows and CLI scripting.
- Real power comes from:
	- Good `[[WikiLink]]` modeling.
	- Intentional use of Hybrid search and depth-limited graph traversals.
	- Deliberate use of roles/colors/workflows for different reasoning modes.
	- Capturing recurring patterns as workflows so agents can reuse them as skills.

This document is the canonical reference for Maenifold search and scripting: how to query, how to compose tools, and how to wire them into agents and workflows for RAG on a graph of thought.
