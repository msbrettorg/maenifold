---
id: T-ST-1-pipeline-crud-validation
status: done
pipeline: sdd
---

# ST-1 — Pipeline CRUD & Validation

**Requirement**: FR-45 (Pipeline Orchestration)
**Component**: crates/maenifold-core/src/workflow/pipeline/

## Purpose

Verify all CRUD operations and validation error paths for the Pipeline MCP tool.

## Test Environment

All tests use the `Pipeline` MCP tool via `maenifold --mcp` stdin/stdout JSON-RPC.
Each test creates its own pipeline and cleans up via delete.
Use a workflow ID that exists: `abductive-reasoning`.

## Test Cases

### TC-1.1: Create single-node workflow pipeline

**Action**: Call Pipeline with:
```json
{
  "definition": {
    "nodes": [{ "id": "root", "workflowId": "abductive-reasoning" }],
    "edges": []
  }
}
```

**Expected**: Success response containing `pipelineId`, `ready` array with `["root"]`, and `nodes` array with one entry.

### TC-1.2: Create multi-node pipeline with all 4 node types

**Action**: Call Pipeline with:
```json
{
  "definition": {
    "nodes": [
      { "id": "wf", "type": "workflow", "workflowId": "abductive-reasoning" },
      { "id": "pr", "type": "prompt", "prompt": "Summarize findings", "agent": "test-agent" },
      { "id": "st", "type": "sequentialThinking", "topic": "Analyze results", "agent": "test-agent", "totalThoughts": 3 },
      { "id": "tl", "type": "tool", "toolName": "ReadMemory", "toolArgs": { "identifier": "test" } }
    ],
    "edges": [
      { "source": "wf", "target": "pr", "type": "success" },
      { "source": "wf", "target": "st", "type": "success" },
      { "source": "pr", "target": "tl", "type": "always" },
      { "source": "st", "target": "tl", "type": "always" }
    ]
  }
}
```

**Expected**: Success with pipelineId, ready=["wf"] (only root is ready), 4 nodes returned.

### TC-1.3: Create with custom maxConcurrency

**Action**: Create pipeline with `"maxConcurrency": 2` and 3 root nodes (no edges).

**Expected**: Success. Pipeline created with concurrency limit of 2.

### TC-1.4: View pipeline

**Action**: Create a pipeline, then call Pipeline with `{ "sessionId": "<id>", "view": true }`.

**Expected**: Response includes `pipelineId`, `status`, `nodes` array with status/sessionId per node, `edges`, `ready`, `active`, `completed`, and `mermaid` diagram string.

### TC-1.5: List pipelines

**Action**: Create 2 pipelines, then call Pipeline with `{ "list": true }`.

**Expected**: Response includes `pipelines` array with entries showing `pipelineId`, `status`, `nodeCount`, `completedCount`. Both created pipelines appear.

### TC-1.6: Delete pipeline

**Action**: Create a pipeline, then call Pipeline with `{ "sessionId": "<id>", "delete": true }`.

**Expected**: Success with `deletedSessionId` matching. Subsequent view returns error.

### TC-1.7: VALIDATION — Reject empty node list

**Action**: Create pipeline with `{ "definition": { "nodes": [], "edges": [] } }`.

**Expected**: Error containing "must contain at least one node".

### TC-1.8: VALIDATION — Reject duplicate node IDs

**Action**: Create pipeline with two nodes both having `"id": "dup"`.

**Expected**: Error containing "duplicate node ID".

### TC-1.9: VALIDATION — Reject cycle

**Action**: Create pipeline with A→B→C→A edges.

**Expected**: Error containing "cycle".

### TC-1.10: VALIDATION — Reject unknown edge source

**Action**: Create pipeline with edge source referencing non-existent node.

**Expected**: Error containing "unknown source node".

### TC-1.11: VALIDATION — Reject unknown edge target

**Action**: Create pipeline with edge target referencing non-existent node.

**Expected**: Error containing "unknown target node".

### TC-1.12: VALIDATION — Reject duplicate edge pair

**Action**: Create pipeline with two edges both from A→B (even with different types).

**Expected**: Error containing "duplicate edge".

### TC-1.13: VALIDATION — Reject blank workflowId

**Action**: Create workflow node with `"workflowId": "  "` (whitespace only).

**Expected**: Error about empty/blank workflowId field.

### TC-1.14: VALIDATION — Reject blank prompt on prompt node

**Action**: Create prompt node with `"prompt": ""`.

**Expected**: Error about blank prompt field.

### TC-1.15: VALIDATION — Reject blank agent on prompt node

**Action**: Create prompt node with valid prompt but `"agent": ""`.

**Expected**: Error about blank agent field.

### TC-1.16: VALIDATION — Reject zero totalThoughts

**Action**: Create sequentialThinking node with `"totalThoughts": 0`.

**Expected**: Error about totalThoughts must be positive.

### TC-1.17: VALIDATION — Reject blank toolName

**Action**: Create tool node with `"toolName": "  "`.

**Expected**: Error about blank toolName.

### TC-1.18: VALIDATION — Reject unknown node type

**Action**: Create node with `"type": "banana"`.

**Expected**: Error about unknown type.

### TC-1.19: VALIDATION — Delete while active nodes exist

**Action**: Create pipeline, dispatch a node (making it Active), then try to delete.

**Expected**: Error about pipeline running / active nodes prevent deletion.

### TC-1.20: VALIDATION — Invalid maxConcurrency

**Action**: Create pipeline with `"maxConcurrency": 0`.

**Expected**: Error about maxConcurrency must be positive.

## Acceptance Criteria

- All 20 test cases execute and produce expected results
- Each validation error returns a meaningful error message (not a generic failure)
- Create/View/List/Delete complete without errors on valid input
- Cleanup: all test pipelines deleted at end

## Verdicts

- Each TC: PASS if expected result matches, FAIL with actual output if not

## Blue-team Notes

PASS 20/20 — All CRUD ops + 14 validation error paths verified. Error messages specific and meaningful.

## Swe Notes

SWE dispatched

## Orchestrator — PASS

20/20 MCP E2E tests passing
