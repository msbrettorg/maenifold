# Dispatch Tool — Design Specification

> **Status**: Draft v1 — iterating  
> **Scope**: New MCP tool for maenifold-rs that manages a dependency-aware work queue and dispatches items to ACP agents

## 1. Overview

**Dispatch** is a knowledge-work queue supervisor. It tracks work items with dependency edges, dispatches them to ACP agents with graph-assembled context, and records results. Agents get access to maenifold MCP tools and can execute Workflow or SequentialThinking submachines.

### What Dispatch is

- A dependency-aware work queue stored in SQLite
- A state machine for each work item (like Workflow and SequentialThinking are state machines for their sessions)
- An ACP client that spawns agent subprocesses, injects context, and captures results
- A bridge between the knowledge graph and autonomous agent execution

### What Dispatch is NOT

- Not a git/code/PR workflow tool
- Not a replacement for Workflow or SequentialThinking — it orchestrates them
- Not stored in `memory://` — queue state lives entirely in SQLite
- Not a daemon — it's an MCP tool called by an orchestrating LLM

## 2. Tool Name

**`Dispatch`**

Alternatives considered and rejected:
- `Task` — overloaded (Copilot CLI, todo lists, generic CS term)
- `WorkItem` — too enterprise
- `Orchestrate` — too vague
- `QueueFlow` — awkward compound

## 3. State Machine

### Item States



### State descriptions

| State | Meaning |
|-------|---------|
| `pending` | Created, dependencies not yet resolved |
| `ready` | All dependencies satisfied, eligible for dispatch |
| `running` | ACP agent actively executing |
| `waiting-sub` | Delegated to a Workflow or SequentialThinking submachine |
| `succeeded` | Completed, result recorded |
| `failed` | Run failed; may retry → `ready` or exhaust → `stuck` |
| `stuck` | Retries exhausted, needs manual intervention |
| `cancelled` | Terminal, stopped by caller |

### Transitions

| From | To | Trigger |
|------|----|---------|
| `pending` | `ready` | All `depends_on` items reached `succeeded` |
| `pending` | `cancelled` | Caller cancels |
| `ready` | `running` | `dispatch` action |
| `ready` | `cancelled` | Caller cancels |
| `running` | `succeeded` | Agent completes successfully |
| `running` | `failed` | Agent fails / timeout / crash |
| `running` | `waiting-sub` | Agent attaches a submachine |
| `running` | `cancelled` | Caller cancels |
| `waiting-sub` | `running` | Submachine reaches terminal state |
| `failed` | `ready` | Retry (attempt_count < max_attempts) |
| `failed` | `stuck` | Retry exhausted (attempt_count >= max_attempts) |

### Dependency resolution

When an item transitions to `succeeded`, Dispatch scans all `pending` items that depend on it. If ALL dependencies of a pending item are now `succeeded`, that item transitions to `ready`.

## 4. Storage — SQLite Only

All queue state lives in SQLite. No markdown files. JSON columns for prompts, results, and structured data (prompts can contain arbitrary text including multi-line content, making JSON the natural serialization).

### `dispatch_items`

The core queue table.

```sql
CREATE TABLE IF NOT EXISTS dispatch_items (
    item_id         TEXT PRIMARY KEY,
    title           TEXT NOT NULL,
    description     TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending'
        CHECK(status IN ('pending','ready','running','waiting-sub','succeeded','failed','stuck','cancelled')),
    
    -- Agent configuration
    agent           TEXT,                       -- copilot, claude-code, codex, etc.
    role            TEXT,                       -- adopt role identifier (e.g. 'researcher')
    color           TEXT,                       -- adopt color identifier (e.g. 'white')
    perspective     TEXT,                       -- adopt perspective identifier (e.g. 'en')
    
    -- Execution mode
    execution_mode  TEXT NOT NULL DEFAULT 'simple'
        CHECK(execution_mode IN ('simple','workflow','thinking')),
    workflow_id     TEXT,                       -- workflow asset ID when mode = 'workflow'
    
    -- Submachine state (populated during execution)
    submachine_type       TEXT,                 -- 'workflow' or 'sequential'
    submachine_session_id TEXT,                 -- session ID of the attached submachine
    
    -- Context assembly
    concept         TEXT,                       -- concept name for graph context assembly
    context_json    TEXT,                       -- cached assembled context (JSON)
    
    -- Retry tracking
    attempt_count   INTEGER NOT NULL DEFAULT 0,
    max_attempts    INTEGER NOT NULL DEFAULT 3,
    
    -- Result
    result_json     TEXT,                       -- JSON: { outcome, summary, artifacts[], citations[] }
    error_text      TEXT,                       -- last failure reason
    
    -- Timestamps
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    started_at      TEXT,
    finished_at     TEXT
);
```

### `dispatch_deps`

Dependency edges between items.

```sql
CREATE TABLE IF NOT EXISTS dispatch_deps (
    item_id         TEXT NOT NULL,
    depends_on      TEXT NOT NULL,
    PRIMARY KEY (item_id, depends_on),
    FOREIGN KEY (item_id) REFERENCES dispatch_items(item_id),
    FOREIGN KEY (depends_on) REFERENCES dispatch_items(item_id)
);
```

### `dispatch_runs`

One row per ACP execution attempt. An item may have multiple runs (retries).

```sql
CREATE TABLE IF NOT EXISTS dispatch_runs (
    run_id          TEXT PRIMARY KEY,
    item_id         TEXT NOT NULL,
    attempt         INTEGER NOT NULL,
    
    -- Agent snapshot (captures what was actually used)
    agent           TEXT NOT NULL,
    role            TEXT,
    color           TEXT,
    perspective     TEXT,
    
    -- ACP session
    acp_session_id  TEXT,
    prompt_json     TEXT NOT NULL,              -- the full prompt envelope sent to agent
    
    -- Result
    status          TEXT NOT NULL DEFAULT 'running'
        CHECK(status IN ('running','succeeded','failed','cancelled')),
    result_json     TEXT,                       -- agent's structured response
    error_text      TEXT,
    
    -- Timestamps
    started_at      TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at        TEXT,
    
    FOREIGN KEY (item_id) REFERENCES dispatch_items(item_id)
);
```

### Key queries

```sql
-- Dashboard: queue state at a glance
SELECT status, count(*) FROM dispatch_items GROUP BY status;

-- Ready items (all deps satisfied)
SELECT i.* FROM dispatch_items i
WHERE i.status = 'ready'
ORDER BY i.created_at;

-- Pending items that might be ready (check deps)
SELECT i.item_id FROM dispatch_items i
WHERE i.status = 'pending'
AND NOT EXISTS (
    SELECT 1 FROM dispatch_deps d
    JOIN dispatch_items dep ON d.depends_on = dep.item_id
    WHERE d.item_id = i.item_id
    AND dep.status != 'succeeded'
);

-- Items blocked by a specific item
SELECT i.* FROM dispatch_items i
JOIN dispatch_deps d ON i.item_id = d.item_id
WHERE d.depends_on = ?;

-- Run history for an item
SELECT * FROM dispatch_runs WHERE item_id = ? ORDER BY attempt;
```

## 5. Tool Interface

Single tool with `action` discriminator (matches `AssumptionLedger` pattern).

### Actions

| Action | Purpose |
|--------|---------|
| `create` | Add a work item to the queue |
| `list` | Query queue status, show ready/blocked items |
| `dispatch` | Trigger ACP execution of a ready item |
| `attach` | Attach a submachine to a running item |
| `status` | Check status of a specific item or run |
| `complete` | Record result when an agent finishes |
| `cancel` | Cancel an item or running dispatch |
| `learn` | Return help documentation |

### Parameters

```
Dispatch(
    action:       enum "create|list|dispatch|attach|status|complete|cancel"  [required]
    
    -- Item identification
    itemId:       string    -- target item (required for most actions)
    
    -- Create parameters
    title:        string    -- short name for the work item
    description:  string    -- what the agent should do
    agent:        string    -- ACP agent to use (copilot, claude-code, codex)
    role:         string    -- role asset ID to adopt (e.g. "researcher")
    color:        string    -- color asset ID to adopt (e.g. "white")
    perspective:  string    -- perspective asset ID to adopt (e.g. "en")
    executionMode: enum "simple|workflow|thinking"  -- default: simple
    workflowId:   string    -- workflow asset ID (when mode=workflow)
    concept:      string    -- concept name for graph context assembly
    dependsOn:    string[]  -- item IDs this depends on
    maxAttempts:  integer   -- retry limit (default: 3)
    
    -- List parameters  
    filterStatus: string    -- filter by status
    showReady:    boolean   -- include dependency resolution
    
    -- Attach parameters
    submachineType:      enum "workflow|sequential"
    submachineSessionId: string
    
    -- Complete parameters
    outcome:      enum "succeeded|failed"
    result:       string    -- summary with [[WikiLinks]]
    artifacts:    string[]  -- URIs of produced artifacts (memory://, session IDs, etc.)
    
    -- Cancel parameters
    reason:       string
    
    learn:        boolean
)
```

### Create example

```json
{
    "action": "create",
    "title": "Research pricing dimension of rate-optimization",
    "description": "Investigate pricing structure, discount mechanics, and optimization levers. Write findings to a memory note.",
    "agent": "copilot",
    "role": "researcher",
    "color": "white",
    "concept": "rate-optimization",
    "executionMode": "simple"
}
```

### Dispatch example

```json
{
    "action": "dispatch",
    "itemId": "di-1773500000000-42017"
}
```

### Complete example

```json
{
    "action": "complete",
    "itemId": "di-1773500000000-42017",
    "outcome": "succeeded",
    "result": "Completed pricing-dimension research for [[rate-optimization]]. Captured findings on [[pricing-models]], [[discounting]], and [[cost-drivers]].",
    "artifacts": ["memory://research/rate-optimization/pricing-dimension"]
}
```

## 6. Response Format

Every response follows this envelope:

```json
{
    "ok": true,
    "action": "create",
    "message": "Created di-1773500000000-42017. Status: ready (no dependencies).",
    "data": { ... },
    "nextActions": [
        { "action": "dispatch", "itemId": "di-1773500000000-42017", "reason": "Item is ready" }
    ]
}
```

`nextActions` is the key UX element — it tells the orchestrating LLM what to do next without requiring it to figure out the queue logic.

### List response example

```json
{
    "ok": true,
    "action": "list",
    "message": "3 ready, 1 blocked, 2 running.",
    "data": {
        "counts": { "ready": 3, "blocked": 1, "running": 2, "succeeded": 5 },
        "items": [
            {
                "itemId": "di-...-42017",
                "title": "Research pricing dimension",
                "status": "ready",
                "agent": "copilot",
                "role": "researcher"
            }
        ]
    },
    "nextActions": [
        { "action": "dispatch", "itemId": "di-...-42017", "reason": "Ready, no running slots occupied" }
    ]
}
```

### Complete response example

```json
{
    "ok": true,
    "action": "complete",
    "message": "Completed di-...-42017. 1 downstream item now ready.",
    "data": {
        "itemId": "di-...-42017",
        "outcome": "succeeded",
        "artifacts": ["memory://research/rate-optimization/pricing-dimension"],
        "unblocked": ["di-...-99031"]
    },
    "nextActions": [
        { "action": "dispatch", "itemId": "di-...-99031", "reason": "All dependencies satisfied" }
    ]
}
```

## 7. Graph Context Assembly

When dispatching, the tool assembles a prompt envelope:

1. **Concept retrieval**: If `concept` is set, call `BuildContext(concept, depth=2)` + `FindSimilarConcepts(concept)` internally
2. **Memory search**: `SearchMemories(description)` to find relevant notes
3. **Dependency context**: Read `result_json` from all completed upstream items
4. **Role/color/perspective**: Load the Adopt asset JSON for each
5. **Compose prompt envelope** (stored as `prompt_json` on the run):

```json
{
    "goal": "item description",
    "acceptance": "what success looks like",
    "upstream": [
        { "title": "completed dep title", "result": "its result summary" }
    ],
    "graphContext": {
        "concepts": ["..."],
        "memories": ["..."],
        "excerpts": ["..."]
    },
    "stance": {
        "role": { ... },
        "color": { ... },
        "perspective": { ... }
    },
    "tools": "You have access to maenifold MCP tools: WriteMemory, ReadMemory, SearchMemories, BuildContext, Workflow, SequentialThinking, etc.",
    "executionMode": "simple | workflow | thinking",
    "workflowId": "optional"
}
```

The caller provides a `concept` name. The tool does the rest.

## 8. Submachine Attachment

When `executionMode` is `workflow` or `thinking`, the dispatched agent is expected to use the corresponding maenifold tool through its MCP connection.

### Workflow mode
- Agent receives instruction to start `Workflow(workflowId)` 
- Agent calls the Workflow tool via MCP, steps through it
- Maenifold tracks the workflow session ID
- When the workflow completes, the dispatch item can complete

### Thinking mode  
- Agent receives instruction to start a SequentialThinking session
- Agent calls SequentialThinking tool via MCP
- Agent drives the thinking session to conclusion
- Dispatch item completes when the thinking session finalizes

### Runtime attachment
A running item can also attach a submachine mid-flight:

```json
{
    "action": "attach",
    "itemId": "di-...-42017",
    "submachineType": "sequential",
    "submachineSessionId": "session-1773500000000-42017"
}
```

This transitions the item to `waiting-sub`. When the submachine reaches a terminal state (`completed`/`cancelled`/`abandoned`), the item returns to `running`.

## 9. ACP Execution Model

### Agent spawning

Agents are spawned as subprocesses via ACP:
- `copilot` → `copilot --acp --stdio` (or equivalent)
- `claude-code` → `claude-code --acp --stdio`
- `codex` → `codex --acp --stdio`

The agent gets:
1. The prompt envelope (assembled from graph context + item description)
2. MCP server access to a maenifold instance for tools
3. Role/color/perspective adoption instructions in the system prompt

### MCP tool access

The dispatched agent connects to maenifold as an MCP server. This means the agent can:
- Search and read memories
- Write new memory notes
- Run workflows and thinking sessions
- Build graph context
- Perform concept operations

**Circularity concern**: The agent connects to a *separate* maenifold process (or the existing one serves both stdio MCP and the tool call). This needs engineering investigation — the key constraint is that the Dispatch tool handler must not deadlock while the agent calls back into maenifold tools.

### Concurrency

- Default: **sequential** (1 slot) — knowledge work often has semantic dependencies
- Configurable: callers can dispatch multiple items if they know the work is independent
- No global slot manager in v1 — the orchestrating LLM manages pacing

## 10. Cognitive Flow — Orchestration Example

**User**: "Research rate-optimization across 3 dimensions in parallel, then synthesize."

**PM agent orchestration**:

```
1. Dispatch(create, title="Research pricing dimension", concept="rate-optimization",
            agent="copilot", role="researcher", color="white")
   → di-001, status: ready

2. Dispatch(create, title="Research workload-shape dimension", concept="rate-optimization",
            agent="copilot", role="researcher", color="white")  
   → di-002, status: ready

3. Dispatch(create, title="Research commitment-strategy dimension", concept="rate-optimization",
            agent="copilot", role="researcher", color="white")
   → di-003, status: ready

4. Dispatch(create, title="Synthesize rate-optimization findings", concept="rate-optimization",
            agent="copilot", role="researcher", color="blue",
            dependsOn=["di-001","di-002","di-003"])
   → di-004, status: pending (blocked by 3 items)

5. Dispatch(dispatch, itemId="di-001")  → running
6. Dispatch(dispatch, itemId="di-002")  → running
7. Dispatch(dispatch, itemId="di-003")  → running

   [agents execute, write memory notes, complete]

8. Dispatch(complete, itemId="di-001", outcome="succeeded",
            result="Completed [[pricing-models]] research...",
            artifacts=["memory://research/rate-optimization/pricing"])
   → succeeded, nextActions: [di-004 still blocked]

9. Dispatch(complete, itemId="di-002", ...) → succeeded
10. Dispatch(complete, itemId="di-003", ...) → succeeded
    → nextActions: [dispatch di-004, all deps satisfied]

11. Dispatch(dispatch, itemId="di-004")  → running
    [synthesis agent reads upstream artifacts, writes synthesis note]

12. Dispatch(complete, itemId="di-004", outcome="succeeded",
            result="Synthesized [[rate-optimization]] across all dimensions...",
            artifacts=["memory://research/rate-optimization/synthesis"])
    → succeeded, queue complete
```

## 11. Item ID Format

`di-{timestamp_ms}-{entropy}` (e.g. `di-1773500000000-42017`)

Matches the existing session ID pattern (`session-{ms}-{entropy}`, `workflow-{ms}`).

## 12. Open Questions

1. **Async bridge**: Tool handlers are sync `fn(&ToolContext, &Value) -> Result<String>`. ACP requires async. Options: (a) dispatch returns immediately, caller polls with `status`; (b) `tokio::runtime::Handle::block_on`; (c) background tokio task with SQLite status updates.

2. **MCP circularity**: How does the spawned agent access maenifold tools without deadlocking the parent process? Options: (a) separate maenifold subprocess; (b) the agent's MCP connection routes to the same rmcp server on a different thread; (c) agent gets tool access via a second stdio pipe.

3. **Who calls `complete`?**: Does the Dispatch tool auto-detect agent completion and update status? Or does the orchestrating LLM poll and call `complete` explicitly? The current design assumes explicit `complete` calls, matching the Workflow pattern where the caller drives transitions.

4. **Auto-dispatch**: Should Dispatch automatically dispatch items that become `ready`? Or always require explicit `dispatch` calls? Current design: explicit only (matches existing maenifold philosophy of caller-driven transitions).

5. **Run event capture**: Should `dispatch_runs` store individual tool calls made by the agent? Or just the final result? v1: just the final result. Event-level capture is a v2 concern.

6. **WikiLink requirements**: `result` on `complete` should require `[[WikiLinks]]` (matches existing pattern). `description` on `create` should encourage but not require them.
