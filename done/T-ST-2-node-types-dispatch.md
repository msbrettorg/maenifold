---
id: T-ST-2-node-types-dispatch
status: done
pipeline: sdd
---

# ST-2 — All Node Types & Dispatch Cycle

**Requirement**: FR-45 (Pipeline Orchestration)
**Component**: crates/maenifold-core/src/workflow/pipeline/

## Purpose

Verify that all 4 node types can be created, dispatched, completed, and abandoned through manual dispatch cycles. Tests the full node status state machine.

## Test Environment

All tests use the `Pipeline` MCP tool via `maenifold --mcp` stdin/stdout JSON-RPC.
Use workflow ID `abductive-reasoning` for workflow nodes.

## Test Cases

### TC-2.1: Workflow node — dispatch and complete

**Action**:
1. Create pipeline with single workflow node: `{ "id": "wf1", "workflowId": "abductive-reasoning" }`
2. Dispatch: `{ "sessionId": "<id>", "dispatch": true }`
3. Verify node status is Active in dispatch response
4. Complete: `{ "sessionId": "<id>", "nodeId": "wf1", "status": "completed", "conclusion": "Workflow completed successfully" }`
5. View pipeline and verify node is Completed with output

**Expected**: Node transitions Pending→Active→Completed. Output preserved.

### TC-2.2: Prompt node — dispatch and complete

**Action**:
1. Create pipeline with prompt node: `{ "id": "pr1", "type": "prompt", "prompt": "Test prompt", "agent": "test-agent" }`
2. Dispatch, verify Active
3. Complete with conclusion
4. View, verify Completed

**Expected**: Same transition as TC-2.1 but for prompt node type.

### TC-2.3: SequentialThinking node — dispatch and complete

**Action**:
1. Create pipeline with ST node: `{ "id": "st1", "type": "sequentialThinking", "topic": "Test topic", "agent": "test-agent", "totalThoughts": 3 }`
2. Dispatch, verify Active
3. Complete with conclusion
4. View, verify Completed

**Expected**: Same transition for sequentialThinking type.

### TC-2.4: Tool node — dispatch and complete

**Action**:
1. Create pipeline with tool node: `{ "id": "tl1", "type": "tool", "toolName": "MemoryStatus" }`
2. Dispatch, verify Active
3. Complete with conclusion
4. View, verify Completed

**Expected**: Same transition for tool type.

### TC-2.5: Abandon a node

**Action**:
1. Create pipeline with single node, dispatch it
2. Abandon: `{ "sessionId": "<id>", "nodeId": "<nodeId>", "status": "abandoned" }`
3. View pipeline

**Expected**: Node status is Abandoned. Output is empty/null.

### TC-2.6: Complete requires non-empty conclusion

**Action**:
1. Create, dispatch a node
2. Try to complete with empty conclusion: `{ "sessionId": "<id>", "nodeId": "<nodeId>", "status": "completed", "conclusion": "" }`

**Expected**: Error about conclusion required.

### TC-2.7: Complete requires Active status

**Action**:
1. Create pipeline (node is Pending)
2. Try to complete without dispatching first

**Expected**: Error about node must be Active.

### TC-2.8: Abandon requires Active status

**Action**:
1. Create pipeline (node is Pending)
2. Try to abandon without dispatching

**Expected**: Error about node must be Active.

### TC-2.9: Linear chain — predecessor context flows downstream

**Action**:
1. Create pipeline: A→B→C (all tool nodes, success edges)
2. Dispatch A (only root is ready)
3. Complete A with conclusion "Output from A"
4. Dispatch B (should now be ready)
5. Complete B with conclusion "Output from B"
6. Dispatch C (should now be ready)
7. Complete C with conclusion "Output from C"
8. View — all 3 Completed

**Expected**: Each dispatch only returns the next ready node. Chain completes in order.

### TC-2.10: Parallel roots — multiple nodes ready simultaneously

**Action**:
1. Create pipeline with 3 independent tool nodes (no edges between them)
2. Dispatch — all 3 should be ready and dispatched
3. Complete all 3
4. View — all Completed, pipeline status "completed"

**Expected**: All roots dispatch at once. Pipeline completes when all done.

### TC-2.11: maxConcurrency limits dispatch

**Action**:
1. Create pipeline with 4 independent nodes, maxConcurrency=2
2. Dispatch — only 2 should be dispatched
3. Complete 1
4. Dispatch again — 1 more should be dispatched (2 active limit)

**Expected**: Never more than 2 active nodes at once.

### TC-2.12: Unsupported status value

**Action**:
1. Create, dispatch a node
2. Try: `{ "sessionId": "<id>", "nodeId": "<nodeId>", "status": "stuck" }`

**Expected**: Error about unsupported status.

## Acceptance Criteria

- All 12 test cases pass
- All 4 node types successfully dispatch and complete
- State machine transitions enforced (Pending→Active→Completed/Abandoned)
- Linear chain proves ordering
- maxConcurrency proves throttling
- Cleanup: all test pipelines deleted

## Blue-team Notes

PASS 12/12 — All 4 node types (workflow/prompt/sequentialThinking/tool) dispatch+complete correctly. State machine enforced. Linear chain ordering correct. maxConcurrency throttling verified (2/4 cap). Note: independently confirmed guard bug — completed pipeline cannot be deleted because guard checks status not actual node state (same root cause as ST-4 graft failures).

## Swe Notes

SWE dispatched

## Orchestrator — PASS

12/12 MCP E2E tests passing
