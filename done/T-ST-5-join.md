---
id: T-80.201
title: Join (Pipeline Merge)
status: done
pipeline: sdd
requirement: FR-45
---

# ST-5 — Join (Pipeline Merge)

**Requirement**: FR-45 (Pipeline Orchestration)
**Component**: crates/maenifold-core/src/workflow/pipeline/handler_surgery_join.rs

## Purpose

Verify that two pipelines can be merged into one, with correct node/edge/state merging and validation of preconditions.

## Test Environment

All tests use the `Pipeline` MCP tool via `maenifold --mcp` stdin/stdout JSON-RPC.
Use tool nodes for simplicity.

## Test Cases

### TC-5.1: Join two simple pipelines

**Action**:
1. Create pipeline-A with node A1 (tool node), dispatch and complete A1
2. Create pipeline-B with node B1 (tool node), dispatch and complete B1
3. Join B into A with connecting edge A1→B1:
```json
{
  "sessionId": "<pipeline-A-id>",
  "join": {
    "sourceSessionId": "<pipeline-B-id>",
    "edges": [{ "source": "A1", "target": "B1", "type": "success" }]
  }
}
```
4. View pipeline-A — should now contain both A1 and B1

**Expected**: Target pipeline contains all nodes from both. Edge connects them.

### TC-5.2: Join preserves node states from both pipelines

**Action**:
1. Create pipeline-A with node A1, dispatch and complete A1 with "output-A1"
2. Create pipeline-B with node B1, dispatch and complete B1 with "output-B1"
3. Join B into A
4. View pipeline-A — both nodes should show Completed with their outputs

**Expected**: Node states (completed, outputs) preserved from both sources.

### TC-5.3: Join with multi-node pipelines

**Action**:
1. Create pipeline-A: A1→A2 (complete both)
2. Create pipeline-B: B1→B2 (complete both)
3. Join B into A with edge A2→B1
4. View — 4 nodes total, all Completed

**Expected**: Full merge of both topologies.

### TC-5.4: VALIDATION — Join rejects conflicting node IDs

**Action**:
1. Create pipeline-A with node "shared-id"
2. Create pipeline-B with node "shared-id"
3. Attempt join

**Expected**: Error about conflicting/duplicate node IDs.

### TC-5.5: VALIDATION — Join rejects if target has active nodes

**Action**:
1. Create pipeline-A with node A1, dispatch A1 (Active)
2. Create pipeline-B with node B1
3. Attempt join B into A while A1 is Active

**Expected**: Error about target pipeline running/active.

### TC-5.6: VALIDATION — Join rejects if source has active nodes

**Action**:
1. Create pipeline-A with node A1 (completed)
2. Create pipeline-B with node B1, dispatch B1 (Active)
3. Attempt join B into A while B1 is Active

**Expected**: Error about source pipeline running/active.

### TC-5.7: Join creating new scheduling dependencies

**Action**:
1. Create pipeline-A with node A1 (completed)
2. Create pipeline-B with nodes B1→B2 (B1 completed, B2 Pending)
3. Join B into A with edge A1→B1
4. View — B2 should be in ready set (B1 already completed)

**Expected**: After join, the merged graph's ready set is correctly recomputed.

### TC-5.8: VALIDATION — Join that creates cycle

**Action**:
1. Create pipeline-A: A1→A2 (complete both)
2. Create pipeline-B: B1 (completed)
3. Join B into A with edges: A2→B1 AND B1→A1 (would create cycle)

**Expected**: Error about cycle in merged graph.

## Acceptance Criteria

- All 8 test cases pass
- Join correctly merges nodes, edges, and states from both pipelines
- Target pipeline ID is preserved after join
- Validation rejects: conflicting IDs, active nodes, cycles
- Cleanup: all test pipelines deleted

## Blue-team Notes

PASS 8/8 — Join correctly merges nodes/edges/states, preserves target ID, recomputes ready set. Validates: conflicting IDs, active nodes in source/target, cycle in merged graph.

## Swe Notes

SWE dispatched

## Orchestrator — PASS

8/8 MCP E2E tests passing
