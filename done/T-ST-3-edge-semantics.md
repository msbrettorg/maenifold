---
id: T-80.199
title: Edge Semantics (Success vs Always)
status: done
pipeline: sdd
requirement: FR-45
---

# ST-3 — Edge Semantics (Success vs Always)

**Requirement**: FR-45 (Pipeline Orchestration)
**Component**: crates/maenifold-core/src/workflow/pipeline/

## Purpose

Verify that `success` and `always` edge types produce correct blocking/unblocking behavior. This is the core DAG scheduling semantic.

## Test Environment

All tests use the `Pipeline` MCP tool via `maenifold --mcp` stdin/stdout JSON-RPC.
Use tool nodes (simplest, no external dependencies) for all tests.

## Test Cases

### TC-3.1: Success edge — completed predecessor unblocks

**Action**:
1. Create: A→B (success edge), both tool nodes
2. Dispatch — only A is ready
3. Complete A
4. Dispatch — B should now be ready
5. Complete B

**Expected**: B blocked until A completes. After A completes, B dispatches.

### TC-3.2: Success edge — abandoned predecessor BLOCKS

**Action**:
1. Create: A→B (success edge)
2. Dispatch A
3. Abandon A
4. Dispatch — B should NOT be dispatched
5. View — B should still be Pending, not in ready set

**Expected**: B is permanently blocked. It does NOT appear in `ready` array.

### TC-3.3: Always edge — completed predecessor unblocks

**Action**:
1. Create: A→B (always edge)
2. Dispatch A, complete A
3. Dispatch — B ready

**Expected**: Same as success edge when predecessor completes.

### TC-3.4: Always edge — abandoned predecessor STILL unblocks

**Action**:
1. Create: A→B (always edge)
2. Dispatch A
3. Abandon A
4. Dispatch — B SHOULD be dispatched despite A being abandoned

**Expected**: B becomes ready and dispatches. This is the key difference from success edges.

### TC-3.5: Diamond pattern with success edges

**Action**:
1. Create diamond: A→B, A→C, B→D, C→D (all success edges)
2. Dispatch — only A ready
3. Complete A
4. Dispatch — B and C both ready (parallel fan-out)
5. Complete B
6. Dispatch — D NOT ready (C still pending/active)
7. Complete C  
8. Dispatch — D now ready (both predecessors done)
9. Complete D

**Expected**: D waits for BOTH predecessors. Fan-out works. Fan-in waits for all.

### TC-3.6: Diamond with mixed edges — one success, one always

**Action**:
1. Create: A→B (success), A→C (success), B→D (success), C→D (always)
2. Dispatch, complete A
3. Dispatch B and C
4. Complete B, abandon C
5. Dispatch — D should be ready (B completed on success edge, C abandoned but on always edge)

**Expected**: D dispatches because: B completed (success satisfied) AND C is terminal (always satisfied even though abandoned).

### TC-3.7: Chain with abandoned middle — downstream blocked

**Action**:
1. Create: A→B→C (all success edges)
2. Dispatch A, complete A
3. Dispatch B, abandon B
4. Dispatch — C NOT ready (B abandoned on success edge)
5. View — C still Pending

**Expected**: Abandonment cascades — blocks all downstream on success edges.

### TC-3.8: Default edge type is success

**Action**:
1. Create pipeline with edge that OMITS the type field: `{ "source": "A", "target": "B" }`
2. Dispatch A, abandon A
3. Dispatch — B should NOT be dispatched (default=success, so abandoned blocks)

**Expected**: Edge type defaults to "success" when omitted.

### TC-3.9: Wide fan-out — one root, many children

**Action**:
1. Create: root→child1, root→child2, root→child3, root→child4, root→child5 (success edges)
2. Dispatch root, complete root
3. Dispatch — all 5 children ready

**Expected**: All children unblock simultaneously.

### TC-3.10: Wide fan-in — many parents, one child

**Action**:
1. Create: p1→sink, p2→sink, p3→sink (success edges)
2. All 3 parents are roots, dispatch all 3
3. Complete p1 and p2 (leave p3 active)
4. Dispatch — sink NOT ready (p3 still active)
5. Complete p3
6. Dispatch — sink now ready

**Expected**: Sink waits for ALL success predecessors.

## Acceptance Criteria

- All 10 test cases pass
- Success edge blocking on abandon is verified (TC-3.2, TC-3.7)
- Always edge non-blocking on abandon is verified (TC-3.4, TC-3.6)
- Diamond fan-out/fan-in correctly handles ordering (TC-3.5, TC-3.6)
- Default edge type is success (TC-3.8)
- Cleanup: all test pipelines deleted

## Blue-team Notes

PASS 10/10 — Success edges block on abandon, Always edges unblock regardless. Fan-out/fan-in correct. Diamond pattern with mixed edges correct. Default edge type=success confirmed.

## Swe Notes

SWE dispatched

## Orchestrator — PASS

10/10 MCP E2E tests passing
