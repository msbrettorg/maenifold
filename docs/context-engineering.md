# Context Engineering with Maenifold

Maenifold implements context engineering principles as described in Anthropic's research on building effective AI agents. This document maps those principles to maenifold's architecture.

## Background

Anthropic defines context engineering as:

> "The set of strategies for curating and maintaining the optimal set of tokens during LLM inference, including all the other information that may land there outside of the prompts."

The core challenge: LLMs have a finite "attention budget" due to transformer architecture constraints (n² pairwise relationships for n tokens). As context grows, recall accuracy degrades—a phenomenon called **context rot**.

The goal: **Find the smallest possible set of high-signal tokens that maximize the likelihood of your desired outcome.**

Source: [Effective context engineering for AI agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) (Anthropic, 2025)

## Principle Mapping

| Anthropic Principle | Maenifold Implementation |
|---------------------|-------------------------|
| Just-in-time context | `BuildContext` retrieves graph neighbors on demand |
| Lightweight identifiers | `memory://` URIs and `[[WikiLinks]]` as pointers, not payloads |
| Progressive disclosure | Graph traversal with `depth` parameter; `FindSimilarConcepts` for discovery |
| Hybrid retrieval | CLAUDE.md up front + PreToolUse hook for on-demand injection |
| Compaction | PreCompact hook extracts concepts/decisions → persists to `memory://` |
| Structured note-taking | `SequentialThinking` sessions, `memory://` files, `AssumptionLedger` |
| Sub-agent architectures | PM coordinates; subagents return condensed ConfessionReport |

## Just-in-Time Context

Anthropic describes the shift from pre-computed retrieval to runtime exploration:

> "Rather than pre-processing all relevant data up front, agents built with the 'just in time' approach maintain lightweight identifiers (file paths, stored queries, web links, etc.) and use these references to dynamically load data into context at runtime using tools."

### Maenifold's Approach

**WikiLinks as lightweight identifiers:**
```
[[authentication]] [[OAuth2]] [[session-management]]
```

These are pointers into the graph, not the content itself. When needed, `BuildContext` retrieves the relevant neighborhood:

```bash
maenifold --tool BuildContext --payload '{"conceptName":"authentication","depth":2,"maxEntities":10}'
```

**PreToolUse hook for automatic injection:**

When a Task prompt contains `[[WikiLinks]]`, the hook:
1. Extracts concepts from the prompt
2. Calls `BuildContext` for each concept
3. Injects graph context into the subagent's starting state

This is the "Concept-as-Protocol" pattern—WikiLinks trigger just-in-time retrieval without manual context building.

## Progressive Disclosure

Anthropic notes:

> "Progressive disclosure allows agents to incrementally discover relevant context through exploration. Each interaction yields context that informs the next decision."

### Maenifold's Approach

**Graph traversal with depth control:**
- `depth=1`: Direct relations only (focused)
- `depth=2`: Two hops (broader context)
- `depth=3+`: Extended neighborhood (exploration)

**Semantic discovery:**
```bash
maenifold --tool FindSimilarConcepts --payload '{"conceptName":"authentication","maxResults":10}'
```

Returns concepts by embedding similarity, enabling discovery of related concepts the agent didn't know to ask for.

**Chain pattern:**
```
FindSimilarConcepts → pick best match → BuildContext → SearchMemories → ReadMemory
```

Each step narrows focus while building understanding.

## Hybrid Retrieval Strategy

Anthropic describes Claude Code's approach:

> "CLAUDE.md files are naively dropped into context up front, while primitives like glob and grep allow it to navigate its environment and retrieve files just-in-time, effectively bypassing the issues of stale indexing and complex syntax trees."

### Maenifold's Approach

**Up front (via CLAUDE.md):**
- Graph navigation instructions
- Tool quick reference
- Concept tagging rules

**Just-in-time (via hooks and tools):**
- SessionStart hook: Injects project + recency context from graph
- PreToolUse hook: Injects concept context for subagent tasks
- Runtime tools: `BuildContext`, `SearchMemories`, `ReadMemory`

This hybrid ensures agents start with orientation but retrieve specifics on demand.

## Compaction

Anthropic explains:

> "Compaction is the practice of taking a conversation nearing the context window limit, summarizing its contents, and reinitiating a new context window with the summary."

### Maenifold's Approach

**PreCompact hook** (`plugin-maenifold/scripts/hooks.sh`):

When compaction triggers, the hook:
1. Extracts first H2 section (problem statement) and last H2 section (conclusion)
2. Extracts all `[[WikiLinks]]` from those sections
3. Identifies decision patterns ("decided to", "chose", "because")
4. Persists to `memory://sessions/compaction/compaction-{timestamp}.md`

This ensures critical concepts and decisions survive compaction and can be retrieved later via graph search.

**Tool result clearing:**

Anthropic recommends clearing old tool results as lightweight compaction. Maenifold's graph approach naturally supports this—once a concept is in the graph, the raw tool output can be discarded; the knowledge persists as WikiLinks and relations.

## Structured Note-Taking

Anthropic observes:

> "Like Claude Code creating a to-do list, or your custom agent maintaining a NOTES.md file, this simple pattern allows the agent to track progress across complex tasks, maintaining critical context and dependencies."

### Maenifold's Approach

**SequentialThinking sessions:**
- Persist to `memory://thinking/sequential/`
- Support branching for alternative exploration
- Require `[[WikiLinks]]` for graph integration
- Survive across context resets via session ID

**Memory files:**
- `WriteMemory` creates persistent knowledge files
- Content with `[[WikiLinks]]` builds graph connections
- `SearchMemories` retrieves by hybrid search (semantic + full-text)

**AssumptionLedger:**
- Track beliefs and their validation status
- Link to thinking sessions via context reference
- Update status as assumptions are validated/invalidated

## Sub-Agent Architectures

Anthropic describes the pattern:

> "Each subagent might explore extensively, using tens of thousands of tokens or more, but returns only a condensed, distilled summary of its work (often 1,000-2,000 tokens)."

### Maenifold's Approach

**PM + Subagent model:**
- PM coordinates with high-level plan
- Subagents (SWE, red-team, blue-team, researcher) perform focused work
- Each subagent has clean context window

**ConfessionReport as condensed summary:**

Instead of returning full traces, subagents return structured ConfessionReport:
1. Instructions followed (with compliance status)
2. Grey areas and shortcuts
3. Files and `[[WikiLinks]]` used

This is the "1,000-2,000 token summary" Anthropic describes—the PM gets signal without the noise of the full exploration trace.

**Shared reasoning graph:**

All subagents contribute to a shared `SequentialThinking` session:
```
session-1234567890 (PM trunk)
├── T-2.1.2-swe (SWE branch)
├── T-2.1.2-blue-team (test branch)
└── T-2.1.2-red-team (attack branch)
```

Context stays isolated per-agent, but knowledge accumulates in the graph.

## The Attention Budget

Anthropic frames context as a scarce resource:

> "Context, therefore, must be treated as a finite resource with diminishing marginal returns. Like humans, who have limited working memory capacity, LLMs have an 'attention budget' that they draw on when parsing large volumes of context."

### Maenifold's Design Philosophy

Maenifold is built around this constraint:

1. **WikiLinks as compression**: `[[authentication]]` is 18 tokens; the full context might be 500+. The pointer is cheap; retrieval is on-demand.

2. **Graph as external memory**: The knowledge graph is unlimited; the context window is not. Store in graph, retrieve what's needed.

3. **Decay as relevance signal**: ACT-R decay weights prioritize recent/frequent content, naturally surfacing high-signal tokens.

4. **Hooks as context gates**: PreToolUse decides *what* context to inject; not everything, just what's relevant to the current task.

## References

- Anthropic. (2025). "Effective context engineering for AI agents." https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
- Anthropic. (2025). "Building effective AI agents." https://www.anthropic.com/research/building-effective-agents
- Anthropic. (2025). "How we built our multi-agent research system." https://www.anthropic.com/engineering/multi-agent-research-system
