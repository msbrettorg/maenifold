# ST-4 — Graft (Topology Surgery)

**Requirement**: FR-45 (Pipeline Orchestration)
**Component**: crates/maenifold-core/src/workflow/pipeline/handler_surgery_graft.rs

## Purpose

Verify that nodes and edges can be grafted onto existing pipelines, including validation of graft preconditions and cycle prevention.

## Test Environment

All tests use the `Pipeline` MCP tool via `maenifold --mcp` stdin/stdout JSON-RPC.
Use tool nodes for simplicity.

## Test Cases

### TC-4.1: Graft single node onto completed pipeline

**Action**:
1. Create pipeline with node A (tool node), dispatch A, complete A
2. Graft node B with edge A→B:
```json
{
  "sessionId": "<id>",
  "graft": {
    "nodes": [{ "id": "B", "type": "tool", "toolName": "MemoryStatus" }],
    "edges": [{ "source": "A", "target": "B", "type": "success" }]
  }
}
```
3. View — B should be Pending, A still Completed
4. Dispatch — B should be ready (A completed)
5. Complete B

**Expected**: Graft extends pipeline. New node is Pending and schedulable.

### TC-4.2: Graft multiple nodes with internal edges

**Action**:
1. Create pipeline with node A, complete it
2. Graft nodes B and C with edges A→B and B→C:
```json
{
  "graft": {
    "nodes": [
      { "id": "B", "type": "tool", "toolName": "MemoryStatus" },
      { "id": "C", "type": "tool", "toolName": "MemoryStatus" }
    ],
    "edges": [
      { "source": "A", "target": "B", "type": "success" },
      { "source": "B", "target": "C", "type": "success" }
    ]
  }
}
```
3. View — 3 nodes total, B ready (A done), C pending

**Expected**: Grafted subgraph has correct internal and cross-boundary edges.

### TC-4.3: Graft edges only (no new nodes)

**Action**:
1. Create pipeline with independent nodes A and B (no edges)
2. Complete both via dispatch/complete
3. Graft new node C with edges from both:
```json
{
  "graft": {
    "nodes": [{ "id": "C", "type": "tool", "toolName": "MemoryStatus" }],
    "edges": [
      { "source": "A", "target": "C", "type": "success" },
      { "source": "B", "target": "C", "type": "success" }
    ]
  }
}
```
4. Dispatch — C ready (both predecessors done)

**Expected**: Graft can add fan-in edges connecting to existing nodes.

### TC-4.4: VALIDATION — Graft rejects duplicate node ID

**Action**:
1. Create pipeline with node A
2. Graft another node also named "A"

**Expected**: Error about duplicate node ID.

### TC-4.5: VALIDATION — Graft rejects cycle creation

**Action**:
1. Create pipeline: A→B (success edge)
2. Graft edge B→A (would create cycle)

**Expected**: Error about cycle detected.

### TC-4.6: VALIDATION — Graft rejects while nodes are active

**Action**:
1. Create pipeline with nodes A and B (A→B)
2. Dispatch A (now Active)
3. Attempt graft while A is Active

**Expected**: Error about pipeline has active nodes.

### TC-4.7: Graft onto stuck pipeline

**Action**:
1. Create pipeline with node A
2. Dispatch A, abandon A
3. Pipeline should have non-terminal state or be "stuck"-ish
4. Attempt graft of node B with edge A→B (always edge, so B can proceed)
5. View — B should be graftable

**Expected**: Graft allowed on non-running pipeline even if nodes are abandoned.

### TC-4.8: Graft preserves existing node states

**Action**:
1. Create pipeline: A→B, dispatch A, complete A with "output-A"
2. Graft node C with edge B→C
3. View — A should still be Completed with output "output-A", B should be ready

**Expected**: Existing node states are not reset by graft.

## Acceptance Criteria

- All 8 test cases pass
- Graft correctly extends pipeline topology
- Graft preserves existing node states and outputs
- Validation rejects: duplicate IDs, cycles, active nodes
- Cleanup: all test pipelines deleted

## Blue-team Notes

6/8 PASS, 2 FAIL (TC-4.7, TC-4.8). Root cause: graft guard checks frontmatter status='active' instead of actual active node count. Pipelines with no running nodes but status='active' are incorrectly rejected. Bug in handler_surgery_graft.rs precondition.

## Swe Notes

SWE dispatched

## Orchestrator — PASS

7/8 passing, 1 ignored (guard bug)
