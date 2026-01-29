# SPEC-2: Session Start Handler Technical Specification

## Document Control

| Version | Date | Author | Description |
|---------|------|--------|-------------|
| 1.0 | 2026-01-27 | SWE Agent | Initial specification |

## References

- PRD.md FR-2.x (Session Start Hook requirements)
- RTM.md FR-2.x entries
- `memory://tech/integrations/opencode-plugin-api-reference`

---

## 1. Overview

This specification defines the technical design for the Session Start Handler, which automatically injects knowledge graph context when a new OpenCode session begins.

### Traced Requirements

| FR ID | Requirement |
|-------|-------------|
| FR-2.1 | Plugin SHALL subscribe to `session.created` event |
| FR-2.2 | Plugin SHALL search memories for current repo/project name |
| FR-2.3 | Plugin SHALL retrieve recent activity concepts (last 24h) |
| FR-2.4 | Plugin SHALL build context for merged concept list |
| FR-2.5 | Plugin SHALL inject context into session via `client.session.prompt({ noReply: true })` |
| FR-2.6 | Plugin SHALL respect TOKEN_LIMIT config (default 4000) |

---

## 2. Event Handler Signature

```typescript
// T-2.1: RTM FR-2.1
event: async ({ event }) => {
  if (event.type === "session.created") {
    const session = event.properties.info as Session;
    await handleSessionStart(session);
  }
}
```

### Session Type Reference

```typescript
type Session = {
  id: string;              // Unique session identifier
  // Additional properties from OpenCode SDK
}
```

### Handler Function Signature

```typescript
// T-2.1: RTM FR-2.1
async function handleSessionStart(session: Session): Promise<void> {
  // 1. Get project name from context
  // 2. Search memories for project
  // 3. Get recent activity concepts
  // 4. Merge and dedupe concepts
  // 5. Build context for concepts
  // 6. Format and inject context
}
```

---

## 3. Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         SESSION START FLOW                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────┐                                                            │
│  │  OpenCode   │                                                            │
│  │   Runtime   │                                                            │
│  └──────┬──────┘                                                            │
│         │                                                                    │
│         │ session.created event                                             │
│         ▼                                                                    │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                    Session Start Handler                              │   │
│  │  ┌─────────────────────────────────────────────────────────────────┐ │   │
│  │  │ Step 1: Get Project Name                                        │ │   │
│  │  │   - Extract from project.worktree (basename)                    │ │   │
│  │  │   - Fallback: use directory basename                            │ │   │
│  │  └─────────────────────────────────────────────────────────────────┘ │   │
│  │                              │                                        │   │
│  │                              ▼                                        │   │
│  │  ┌─────────────────────────────────────────────────────────────────┐ │   │
│  │  │ Step 2: Search Memories (FR-2.2)                                │ │   │
│  │  │   maenifold --tool SearchMemories                               │ │   │
│  │  │   --payload '{"query": "<projectName>"}'                        │ │   │
│  │  │                                                                  │ │   │
│  │  │   Output: List of memory files with concepts                    │ │   │
│  │  │   Extract: [[concepts]] from search results                     │ │   │
│  │  └─────────────────────────────────────────────────────────────────┘ │   │
│  │                              │                                        │   │
│  │                              ▼                                        │   │
│  │  ┌─────────────────────────────────────────────────────────────────┐ │   │
│  │  │ Step 3: Get Recent Activity (FR-2.3)                            │ │   │
│  │  │   maenifold --tool RecentActivity                               │ │   │
│  │  │   --payload '{"timespan": "1.00:00:00", "limit": 10}'           │ │   │
│  │  │                                                                  │ │   │
│  │  │   Output: Recent files modified in last 24h                     │ │   │
│  │  │   Extract: [[concepts]] from recent activity                    │ │   │
│  │  └─────────────────────────────────────────────────────────────────┘ │   │
│  │                              │                                        │   │
│  │                              ▼                                        │   │
│  │  ┌─────────────────────────────────────────────────────────────────┐ │   │
│  │  │ Step 4: Merge & Dedupe Concepts                                 │ │   │
│  │  │   - Combine concepts from Steps 2 + 3                           │ │   │
│  │  │   - Normalize: lowercase, hyphenated                            │ │   │
│  │  │   - Deduplicate using Set                                       │ │   │
│  │  │   - Rank by frequency                                           │ │   │
│  │  └─────────────────────────────────────────────────────────────────┘ │   │
│  │                              │                                        │   │
│  │                              ▼                                        │   │
│  │  ┌─────────────────────────────────────────────────────────────────┐ │   │
│  │  │ Step 5: Build Context (FR-2.4)                                  │ │   │
│  │  │   For top N concepts (by frequency):                            │ │   │
│  │  │     maenifold --tool BuildContext                               │ │   │
│  │  │     --payload '{"conceptName": "<concept>", "depth": 1}'        │ │   │
│  │  │                                                                  │ │   │
│  │  │   Output: Related concepts and file references                  │ │   │
│  │  │   Accumulate: until TOKEN_LIMIT approached                      │ │   │
│  │  └─────────────────────────────────────────────────────────────────┘ │   │
│  │                              │                                        │   │
│  │                              ▼                                        │   │
│  │  ┌─────────────────────────────────────────────────────────────────┐ │   │
│  │  │ Step 6: Format Context                                          │ │   │
│  │  │   - Create markdown section with header                         │ │   │
│  │  │   - Include concept summaries                                   │ │   │
│  │  │   - List related memory:// URIs                                 │ │   │
│  │  │   - Truncate if > TOKEN_LIMIT (FR-2.6)                          │ │   │
│  │  └─────────────────────────────────────────────────────────────────┘ │   │
│  │                              │                                        │   │
│  └──────────────────────────────┼───────────────────────────────────────┘   │
│                                 │                                            │
│                                 ▼                                            │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ Step 7: Inject Context (FR-2.5)                                      │   │
│  │   await client.session.prompt({                                      │   │
│  │     path: { id: session.id },                                        │   │
│  │     body: {                                                          │   │
│  │       noReply: true,  // CRITICAL: No AI response                    │   │
│  │       parts: [{                                                      │   │
│  │         type: "text",                                                │   │
│  │         text: formattedContext                                       │   │
│  │       }]                                                             │   │
│  │     }                                                                │   │
│  │   })                                                                 │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                 │                                            │
│                                 ▼                                            │
│                          Session Ready                                       │
│                     (User sees injected context)                             │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. CLI Tool Invocations

### 4.1 SearchMemories (FR-2.2)

```typescript
// T-2.2: RTM FR-2.2
const searchResult = await runMaenifoldTool("SearchMemories", {
  query: projectName,
  mode: "Hybrid",      // Best of text + semantic search
  pageSize: 5          // Limit results to top 5
});
```

**CLI Command:**
```bash
maenifold --tool SearchMemories --payload '{"query":"maenifold","mode":"Hybrid","pageSize":5}'
```

**Expected Output (JSON):**
```json
{
  "results": [
    {
      "uri": "memory://tech/integrations/opencode-plugin-for-maenifold",
      "title": "OpenCode Plugin for Maenifold",
      "score": 0.85,
      "concepts": ["OpenCode", "maenifold", "plugin"]
    }
  ]
}
```

### 4.2 RecentActivity (FR-2.3)

```typescript
// T-2.3: RTM FR-2.3
const recentResult = await runMaenifoldTool("RecentActivity", {
  timespan: "1.00:00:00",   // Last 24 hours (.NET TimeSpan format)
  limit: 10,                 // Up to 10 recent items
  filter: "memory"           // Only memory files, not thinking sessions
});
```

**CLI Command:**
```bash
maenifold --tool RecentActivity --payload '{"timespan":"1.00:00:00","limit":10,"filter":"memory"}'
```

**Expected Output (JSON):**
```json
{
  "items": [
    {
      "uri": "memory://tech/integrations/opencode-plugin-api-reference",
      "title": "OpenCode Plugin API Reference",
      "modified": "2026-01-27T21:36:01-08:00",
      "concepts": ["OpenCode", "SDK", "plugin", "API"]
    }
  ]
}
```

### 4.3 BuildContext (FR-2.4)

```typescript
// T-2.4: RTM FR-2.4
const contextResult = await runMaenifoldTool("BuildContext", {
  conceptName: concept,      // e.g., "maenifold"
  depth: 1,                  // Direct relations only
  maxEntities: 10,           // Limit graph traversal
  includeContent: false      // Just relationships, not full content
});
```

**CLI Command:**
```bash
maenifold --tool BuildContext --payload '{"conceptName":"maenifold","depth":1,"maxEntities":10}'
```

**Expected Output (JSON):**
```json
{
  "concept": "maenifold",
  "relatedConcepts": [
    { "name": "OpenCode", "relationship": "integrated-with", "files": 3 },
    { "name": "knowledge-graph", "relationship": "provides", "files": 5 }
  ],
  "fileReferences": [
    "memory://tech/integrations/opencode-plugin-for-maenifold",
    "memory://tech/integrations/opencode-plugin-api-reference"
  ]
}
```

---

## 5. Context Injection Pattern

### 5.1 The `noReply: true` Constraint (FR-2.5)

The OpenCode SDK's `client.session.prompt()` method supports a `noReply` option that injects text into the session WITHOUT triggering an AI response. This is critical for context injection.

```typescript
// T-2.5: RTM FR-2.5
async function injectContext(
  client: OpencodeClient,
  sessionId: string,
  context: string
): Promise<void> {
  await client.session.prompt({
    path: { id: sessionId },
    body: {
      noReply: true,  // CRITICAL: Prevents AI response
      parts: [{
        type: "text",
        text: context
      }]
    }
  });
}
```

### 5.2 Context Format Template

```markdown
## Knowledge Graph Context

The following context was automatically retrieved from maenifold based on this project and recent activity.

### Project: {projectName}

**Related Concepts:**
- [[concept-1]] - {brief description or file count}
- [[concept-2]] - {brief description or file count}

**Recent Activity (last 24h):**
- {memory title 1} (memory://path/to/file)
- {memory title 2} (memory://path/to/file)

**Graph Connections:**
- [[concept-1]] → [[related-concept-a]], [[related-concept-b]]
- [[concept-2]] → [[related-concept-c]]

---
*Use `ma:readmemory` to explore specific files or `ma:buildcontext` for deeper graph traversal.*
```

### 5.3 Token Limit Enforcement (FR-2.6)

```typescript
// T-2.6: RTM FR-2.6
const TOKEN_LIMIT = parseInt(process.env.TOKEN_LIMIT || "4000", 10);

function enforceTokenLimit(context: string): string {
  // Rough estimate: ~4 chars per token
  const estimatedTokens = Math.ceil(context.length / 4);
  
  if (estimatedTokens <= TOKEN_LIMIT) {
    return context;
  }
  
  // Truncate with indicator
  const targetLength = TOKEN_LIMIT * 4 - 100; // Leave room for truncation message
  return context.substring(0, targetLength) + 
    "\n\n*[Context truncated to fit token limit. Use ma:searchmemories for more.]*";
}
```

---

## 6. Error Handling

### 6.1 CLI Unavailable (graceful degradation)

```typescript
if (!maenifoldCli) {
  await client.app.log({
    body: {
      service: "maenifold-plugin",
      level: "warn",
      message: "maenifold CLI not found, skipping context injection"
    }
  });
  return; // No error, just skip
}
```

### 6.2 CLI Timeout

```typescript
const CLI_TIMEOUT = parseInt(process.env.CLI_TIMEOUT || "5000", 10);

async function runMaenifoldTool(tool: string, payload: object): Promise<any> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), CLI_TIMEOUT);
  
  try {
    const proc = Bun.spawn([maenifoldCli, "--tool", tool, "--payload", JSON.stringify(payload)], {
      signal: controller.signal
    });
    const output = await new Response(proc.stdout).text();
    return JSON.parse(output);
  } catch (e) {
    if (e.name === "AbortError") {
      console.warn(`maenifold ${tool} timed out after ${CLI_TIMEOUT}ms`);
      return null;
    }
    throw e;
  } finally {
    clearTimeout(timeout);
  }
}
```

### 6.3 Empty Results

If any CLI tool returns empty/null results, the handler continues with available data rather than failing entirely.

---

## 7. Performance Considerations

| Metric | Target | Strategy |
|--------|--------|----------|
| Total handler latency | < 2s | Parallel CLI calls where possible |
| SearchMemories | < 500ms | Limit pageSize to 5 |
| RecentActivity | < 200ms | Already fast, limit 10 |
| BuildContext (per concept) | < 300ms | depth=1, maxEntities=10 |
| Number of BuildContext calls | Max 5 | Only top-frequency concepts |

### 7.1 Parallel Execution

Steps 2 and 3 (SearchMemories and RecentActivity) can run in parallel:

```typescript
const [searchResult, recentResult] = await Promise.all([
  runMaenifoldTool("SearchMemories", { query: projectName, pageSize: 5 }),
  runMaenifoldTool("RecentActivity", { timespan: "1.00:00:00", limit: 10 })
]);
```

---

## 8. Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `TOKEN_LIMIT` | 4000 | Maximum tokens for injected context |
| `CLI_TIMEOUT` | 5000 | CLI command timeout in milliseconds |
| `MAENIFOLD_ROOT` | (none) | Override path to maenifold binary |
| `SESSION_START_ENABLED` | true | Enable/disable session start hook |

---

## 9. Testing Strategy

| Test ID | Description | Verifies |
|---------|-------------|----------|
| T-2.1.1 | Event hook receives session.created | FR-2.1 |
| T-2.2.1 | SearchMemories called with project name | FR-2.2 |
| T-2.3.1 | RecentActivity called with 24h timespan | FR-2.3 |
| T-2.4.1 | BuildContext called for merged concepts | FR-2.4 |
| T-2.5.1 | Context injected with noReply: true | FR-2.5 |
| T-2.6.1 | Context truncated at TOKEN_LIMIT | FR-2.6 |
| T-2.E.1 | Graceful degradation when CLI unavailable | FR-1.3 |
| T-2.E.2 | Timeout handling for slow CLI | NFR-1.1 |

---

## 10. Implementation Checklist

- [ ] Implement `handleSessionStart()` function
- [ ] Add event hook subscription for `session.created`
- [ ] Implement `runMaenifoldTool()` with timeout
- [ ] Implement concept extraction from search results
- [ ] Implement concept merging and deduplication
- [ ] Implement context formatting
- [ ] Implement `injectContext()` with `noReply: true`
- [ ] Add token limit enforcement
- [ ] Add error handling for all failure modes
- [ ] Add logging for debugging
- [ ] Write tests for T-2.x.x test cases
