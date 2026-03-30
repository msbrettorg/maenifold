# ST-6 — Fork (Pipeline Cloning)

**Requirement**: FR-45 (Pipeline Orchestration)
**Component**: crates/maenifold-core/src/workflow/pipeline/handler_surgery_fork.rs

## Purpose

Verify that pipelines can be forked from any node, creating independent copies of the downstream subgraph with predecessor context preservation.

## Test Environment

All tests use the `Pipeline` MCP tool via `maenifold --mcp` stdin/stdout JSON-RPC.
Use tool nodes for simplicity.

## Test Cases

### TC-6.1: Fork from root — creates complete copy

**Action**:
1. Create pipeline: A→B→C (tool nodes, success edges)
2. Fork from A: `{ "sessionId": "<id>", "fork": { "fromNodeId": "A" } }`
3. View forked pipeline — should have 3 nodes (all Pending)
4. View original — unchanged

**Expected**: Fork from root clones entire graph. All nodes reset to Pending. Source untouched.

### TC-6.2: Fork from middle — downstream subgraph only

**Action**:
1. Create pipeline: A→B→C, dispatch and complete A
2. Fork from B: `{ "fork": { "fromNodeId": "B" } }`
3. View forked pipeline — should have 2 nodes (B and C), both Pending
4. Original still has A (Completed), B (Pending), C (Pending)

**Expected**: Only B and C in fork. A is not included.

### TC-6.3: Fork from leaf — single node

**Action**:
1. Create pipeline: A→B→C, complete A and B
2. Fork from C
3. View forked pipeline — single node C, Pending

**Expected**: Fork of leaf produces single-node pipeline.

### TC-6.4: Fork with explicit newSessionId

**Action**:
1. Create pipeline with node A
2. Fork with explicit ID: `{ "fork": { "fromNodeId": "A", "newSessionId": "my-custom-fork" } }`
3. View using "my-custom-fork" as sessionId

**Expected**: Forked pipeline uses the specified session ID.

### TC-6.5: VALIDATION — Fork rejects non-existent node

**Action**:
1. Create pipeline with node A
2. Fork from "nonexistent": `{ "fork": { "fromNodeId": "nonexistent" } }`

**Expected**: Error about source node not found.

### TC-6.6: VALIDATION — Fork rejects duplicate session ID

**Action**:
1. Create pipeline-X with node A
2. Create pipeline-Y with node B
3. Fork from A in pipeline-X with `newSessionId` set to pipeline-Y's ID

**Expected**: Error about session ID already exists.

### TC-6.7: Fork preserves branching topology

**Action**:
1. Create diamond: A→B, A→C, B→D, C→D
2. Fork from A
3. View forked — should have A, B, C, D with diamond edges preserved

**Expected**: Complex topology preserved in fork.

### TC-6.8: Fork from node with completed predecessors captures context

**Action**:
1. Create pipeline: A→B→C, dispatch and complete A with "context-from-A"
2. Fork from B
3. View forked pipeline
4. Dispatch B in forked pipeline

**Expected**: B in forked pipeline should be dispatchable. The predecessor context from A should be captured into B's workflow session.

### TC-6.9: Source pipeline unaffected by fork operations

**Action**:
1. Create pipeline: A→B, complete A, dispatch B
2. Fork from A
3. Complete B in original pipeline
4. View original — B Completed
5. View fork — all Pending

**Expected**: Operations on fork don't affect source and vice versa.

## Acceptance Criteria

- All 9 test cases pass
- Fork correctly creates independent downstream subgraphs
- Source pipeline never mutated by fork
- Predecessor context captured for fork roots
- Validation rejects: non-existent nodes, duplicate session IDs
- Cleanup: all test pipelines (including forks) deleted

## Blue-team Notes

PASS 9/9 — Fork correctly clones downstream subgraphs, preserves diamond topology, captures predecessor context, rejects non-existent nodes and duplicate session IDs, maintains source/fork isolation.

## Swe Notes

SWE dispatched

## Orchestrator — PASS

9/9 MCP E2E tests passing
