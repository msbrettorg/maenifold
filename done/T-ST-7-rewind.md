---
id: T-ST-7-rewind
status: done
pipeline: sdd
---

# ST-7 — Rewind (Node Reset & Cascade)

**Requirement**: FR-45 (Pipeline Orchestration)
**Component**: crates/maenifold-core/src/workflow/pipeline/handler_lifecycle_rewind.rs

## Purpose

Verify that rewind correctly resets specified nodes AND all their downstream descendants, allocating fresh sessions while preserving upstream state.

## Test Environment

All tests use the `Pipeline` MCP tool via `maenifold --mcp` stdin/stdout JSON-RPC.
Use tool nodes for simplicity.

## Test Cases

### TC-7.1: Rewind single leaf node

**Action**:
1. Create pipeline: A→B, complete both
2. Rewind B: `{ "sessionId": "<id>", "rewind": { "nodeIds": ["B"] } }`
3. View — A still Completed, B reset to Pending

**Expected**: Only B is reset. A untouched.

### TC-7.2: Rewind cascades downstream

**Action**:
1. Create pipeline: A→B→C→D, complete all 4
2. Rewind B: `{ "rewind": { "nodeIds": ["B"] } }`
3. View — A Completed, B/C/D all Pending

**Expected**: Rewind of B cascades to C and D.

### TC-7.3: Rewind root resets everything

**Action**:
1. Create pipeline: A→B→C, complete all
2. Rewind A
3. View — all 3 Pending

**Expected**: Rewinding root resets entire pipeline.

### TC-7.4: Rewind resets attempts counter

**Action**:
1. Create pipeline with node A, dispatch and abandon it (attempts=1)
2. Restart A (attempts=2)
3. Dispatch and abandon again
4. Rewind A
5. View — A should be Pending with attempts=0

**Expected**: Rewind zeroes the attempts counter.

### TC-7.5: Rewind clears output

**Action**:
1. Create pipeline with node A, dispatch, complete with "my-output"
2. View — A has output "my-output"
3. Rewind A
4. View — A Pending, output null/empty

**Expected**: Output cleared on rewind.

### TC-7.6: Rewind allocates new session ID

**Action**:
1. Create pipeline with node A
2. View — note A's sessionId
3. Dispatch and complete A
4. Rewind A
5. View — A has DIFFERENT sessionId than before

**Expected**: Fresh session allocated, preventing state leakage from prior attempt.

### TC-7.7: Rewind multiple nodes simultaneously

**Action**:
1. Create pipeline: A→C, B→C (two roots, one sink)
2. Complete all 3
3. Rewind both A and B: `{ "rewind": { "nodeIds": ["A", "B"] } }`
4. View — A and B Pending, C also Pending (downstream of both)

**Expected**: Multiple rewind targets and their combined downstream all reset.

### TC-7.8: Rewind in diamond — partial reset

**Action**:
1. Create diamond: A→B, A→C, B→D, C→D, complete all
2. Rewind B only
3. View — A Completed, B Pending, C Completed, D Pending (downstream of B)

**Expected**: D resets because it's downstream of B, even though C is still Completed.

### TC-7.9: Rewind preserves upstream state

**Action**:
1. Create: A→B→C, complete all with outputs "outA", "outB", "outC"
2. Rewind C (leaf only)
3. View — A has "outA", B has "outB", C is Pending with no output

**Expected**: Only the rewound node and downstream lose their outputs.

### TC-7.10: VALIDATION — Rewind non-existent node

**Action**:
1. Create pipeline with node A
2. Rewind "nonexistent": `{ "rewind": { "nodeIds": ["nonexistent"] } }`

**Expected**: Error about unknown node.

## Acceptance Criteria

- All 10 test cases pass
- Rewind correctly cascades to all downstream descendants
- Rewind preserves upstream node states and outputs
- Attempts counter zeroed on rewind
- Fresh session IDs allocated
- Cleanup: all test pipelines deleted

## Blue-team Notes

PASS 10/10 — Rewind cascade, root reset, attempts zeroed, session rotation, multi-target, diamond partial reset, upstream preservation, unknown node rejection — all correct.

## Swe Notes

SWE dispatched

## Orchestrator — PASS

10/10 MCP E2E tests passing
