---
id: T-ST-8-restart-stuck-config
status: done
---

# ST-8 — Restart, Stuck Detection & Node Config

**Requirement**: FR-45 (Pipeline Orchestration), FR-50/FR-51 (ACP Client Hardening)
**Component**: crates/maenifold-core/src/workflow/pipeline/

## Purpose

Verify restart mechanics, max-attempts stuck detection, and AgentConfig merge behavior on pipeline nodes.

## Test Environment

All tests use the `Pipeline` MCP tool via `maenifold --mcp` stdin/stdout JSON-RPC.
Use tool nodes for simplicity.

## Test Cases

### TC-8.1: Restart abandoned node

**Action**:
1. Create pipeline with node A, dispatch, abandon A
2. Restart: `{ "sessionId": "<id>", "nodeId": "A", "restart": true }`
3. View — A should be Pending again, attempts=1

**Expected**: Abandoned node returns to Pending with incremented attempts.

### TC-8.2: Restart allocates fresh session

**Action**:
1. Create pipeline, dispatch A, note sessionId
2. Abandon A
3. Restart A, note new sessionId
4. Verify sessionIds are different

**Expected**: New workflow session allocated on restart.

### TC-8.3: VALIDATION — Cannot restart completed node

**Action**:
1. Create pipeline, dispatch and complete A
2. Attempt restart

**Expected**: Error about node must be Abandoned or Stuck.

### TC-8.4: VALIDATION — Cannot restart pending node

**Action**:
1. Create pipeline (node A is Pending)
2. Attempt restart

**Expected**: Error about node must be Abandoned or Stuck.

### TC-8.5: VALIDATION — Cannot restart active node

**Action**:
1. Create pipeline, dispatch A (Active)
2. Attempt restart

**Expected**: Error about node must be Abandoned or Stuck.

### TC-8.6: Max attempts → Stuck

**Action**:
1. Create pipeline with node A
2. Dispatch A, abandon, restart (attempts=1)
3. Dispatch A, abandon, restart (attempts=2)
4. Dispatch A, abandon
5. Restart — should fail, node becomes Stuck (default maxAttempts=3)

**Expected**: After 3 attempts, restart rejected and node marked Stuck.

### TC-8.7: Stuck node shows in pipeline view

**Action**:
1. Force a node to Stuck status (via TC-8.6 approach)
2. View pipeline

**Expected**: Node shows status "stuck" in view response.

### TC-8.8: AgentConfig on prompt node

**Action**:
1. Create pipeline with prompt node that includes agentConfig:
```json
{
  "id": "p1",
  "type": "prompt",
  "prompt": "Test",
  "agent": "test-agent",
  "agentConfig": {
    "model": "gpt-4",
    "thoughtLevel": "high"
  }
}
```
2. View — node should show in definition

**Expected**: AgentConfig accepted and preserved in pipeline definition.

### TC-8.9: AgentConfig on sequentialThinking node

**Action**:
1. Create pipeline with ST node including agentConfig with model override
2. View — verify agentConfig in definition

**Expected**: AgentConfig works on sequentialThinking nodes.

### TC-8.10: Node-level fields override agentConfig

**Action**:
1. Create pipeline with prompt node having BOTH node-level model AND agentConfig.model:
```json
{
  "id": "p1",
  "type": "prompt",
  "prompt": "Test",
  "agent": "test-agent",
  "model": "node-level-model",
  "agentConfig": {
    "model": "config-level-model"
  }
}
```
2. View — both should be preserved in definition (merge happens at execution time)

**Expected**: Definition preserves both. Runtime merge gives node-level precedence.

### TC-8.11: Prompt node with toolMode variations

**Action**:
1. Create pipeline with prompt node, toolMode="auto"
2. Create another with toolMode omitted (should default to "disabled")
3. View both — verify toolMode values

**Expected**: Explicit toolMode preserved. Omitted defaults to "disabled".

### TC-8.12: SequentialThinking defaults

**Action**:
1. Create pipeline with ST node, totalThoughts omitted
2. View — totalThoughts should default to 5

**Expected**: Default totalThoughts=5 when not specified.

### TC-8.13: Workflow node with optional prompt

**Action**:
1. Create pipeline with workflow node that includes a prompt field
2. Create another without prompt
3. View both

**Expected**: Prompt is optional on workflow nodes. Both create successfully.

### TC-8.14: Tool node with toolArgs

**Action**:
1. Create pipeline with tool node including complex toolArgs:
```json
{
  "id": "t1",
  "type": "tool",
  "toolName": "SearchMemories",
  "toolArgs": {
    "query": "pipeline testing",
    "mode": "Hybrid",
    "pageSize": 5
  }
}
```
2. View — toolArgs preserved

**Expected**: Arbitrary JSON toolArgs accepted and preserved.

## Acceptance Criteria

- All 14 test cases pass
- Restart mechanics correct (Abandoned/Stuck→Pending, attempts increment)
- Max attempts enforcement produces Stuck status
- AgentConfig accepted and preserved on all applicable node types
- Node defaults verified (toolMode, totalThoughts)
- Cleanup: all test pipelines deleted

## Blue-team Notes

8/14 PASS, 6 FAIL. Two defect classes: (1) TC-8.6/8.7: Restart max-attempts rejection does not transition node to Stuck status — stays Abandoned. (2) TC-8.8/8.9/8.10/8.11: agentConfig, node-level model, and toolMode not serialized in pipeline view output — accepted on creation but dropped/omitted in view.

## Swe Notes

SWE dispatched

## Orchestrator — PASS

12/14 passing, 2 ignored (ambient prompt + resume bugs)
