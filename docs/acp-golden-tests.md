# ACP Golden Tests (AcpExec E2E)

AcpExec is the **only** tool under test. These are live integration tests against real ACP agents — not unit tests. They dogfood the tool itself and verify that the full chain works: AcpExec → sacp client → agent process → response → memory:// persistence.

Use `model: gpt-5.4`, `thoughtLevel: low`, `timeoutSeconds: 60` for all golden tests. Agent: any of `Codex`, `OpenCode`, `Copilot`.

## Test 1: Prompt Mode

**Call:** `AcpExec { mode: "prompt", agent, prompt: "Reply with exactly: PING_OK", model, thoughtLevel, timeoutSeconds }`

**Verify response:**
- `text` contains "PING_OK"
- `error` is null
- `stopReason` is "end_turn"

**Verify artifacts:** None expected — prompt mode is stateless.

## Test 2: Think Mode (Sequential Thinking via ACP)

**Call:** `AcpExec { mode: "think", agent, topic: "What is 2+2? Show your reasoning.", totalThoughts: 2, model, thoughtLevel, timeoutSeconds }`

**Verify response:**
- `text` is non-empty
- `error` is null

**Verify artifacts (memory://):**
- A sequential thinking session was created at `memory://thinking/sequential/...`
- Use `RecentActivity { filter: "thinking" }` to find the session
- Read the session file — it must contain:
  - Frontmatter with `status: completed` (or `active` if still in progress)
  - At least 1 thought entry with `[[WikiLinks]]`
  - A conclusion / ConfessionReport if completed

## Test 3: Workflow Mode

**Call:** `AcpExec { mode: "workflow", agent, workflowId: "abductive-reasoning", prompt: "Why does 1+1=2?", model, thoughtLevel, timeoutSeconds: 120 }`

**Verify response:**
- `text` is non-empty
- `error` is null

**Verify artifacts (memory://):**
- A workflow session was created — check `RecentActivity { filter: "all" }`
- Read the session file — it must contain:
  - The workflow ID in frontmatter
  - Step responses with `[[WikiLinks]]`
  - If a submachine was spawned (🧠 step), verify it's linked
  - A conclusion / ConfessionReport if completed

## What Constitutes Failure

- Response with `error` set = **FAIL**
- Timeout (no response within `timeoutSeconds`) = **FAIL**
- Think/workflow mode produces no memory:// artifact = **FAIL**
- Memory artifact missing `[[WikiLinks]]` = **FAIL** (the agent didn't follow instructions)
- Memory artifact missing conclusion on a completed session = **FAIL**

## PM Checklist (after running golden tests)

1. Run all 3 tests sequentially (not parallel — shared agent pool)
2. For each think/workflow test, use `RecentActivity` to find the session
3. Use `ReadMemory` to inspect the session artifact
4. Verify state machine integrity: status, thoughts/steps, links, confession
5. Record results in gate via `append` on T-88.15
