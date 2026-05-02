---
name: pipeline-sprint
description: "Orchestrate sprint execution using the maenifold Pipeline tool. Use this skill whenever the user mentions 'run sprint', 'pipeline sprint', 'execute backlog', 'burn down P1s', 'sprint pipeline', 'autonomous sprint', or any request to execute multiple gate tasks through a pipeline DAG."
---

# Pipeline Sprint Orchestration

This skill is the working knowledge of how to execute gate sprints via the Pipeline tool. Only verified-working patterns are documented here. Speculation is forbidden.

## Verified Capabilities (as of 2026-05-02)

### ✅ Tool node calling local maenifold tool

A pipeline `tool` node with `toolName: "maenifold.gate_Pass"` successfully invokes the local gate handler. The `maenifold.` prefix routes to `invoke_local_tool` → `ToolRegistry::try_invoke`.

**Verified by:** Pipeline `sprint-test-1777685540773` — `gate-start` tool node completed and advanced T-243.21 from pending to swe.

**Required precondition:** T-249.3 fix (Handle::try_current pattern) must be deployed. Without it, `block_on` panics inside the pipeline driver's async runtime.

### ✅ Linear success-edge cascade

A chain of nodes connected by `{type: "success"}` edges executes in order. Each node waits for its predecessor to complete before starting.

**Verified by:** Same pipeline — gate-start → swe → red-team → blue-team → gate-done all transitioned to completed in sequence.

### ✅ 7-node SDD pipeline with 4 gate transitions advances task to done

A DAG with 4 gate-transition tool nodes interleaved with 3 agent prompt nodes correctly advances a gate task all the way through the SDD process:

```
g1 (gate_Pass: pending→swe) → swe → g2 (gate_Pass: swe→red) → red-team → g3 (gate_Pass: red→blue) → blue-team → g4 (gate_Pass: blue→done)
```

**Verified by:** Pipeline `sprint-full-1777686582893` — all 7 nodes completed, T-243.22 went pending → swe → red-team → blue-team → done autonomously, file moved to `done/`.

This is the template for a single-task SDD pipeline. Each gate transition needs its own tool node because gate only allows one-step state advances.

### ✅ Tool node `learn=true` schema discovery

`maenifold tool Pipeline '{"learn":true}'` returns canonical parameter docs.

### ✅ Tool output can map into downstream tool arguments

An edge-level `mapping` object maps fields from a completed source tool node's JSON output into the downstream tool node's `toolArgs`.

Verified shape:

```json
{
  "source": "read-task",
  "target": "append-path",
  "type": "success",
  "mapping": {
    "notes": "path"
  }
}
```

Verified DAG:

```text
read-task (maenifold.gate_TaskBody)
  -- mapping notes = source output path -->
append-path (maenifold.gate_Append)
```

`gate_TaskBody` returned a JSON object with `path`, and the downstream `gate_Append` node received that value as its `notes` argument.

**Verified by:** Pipeline `mapping-proof-1777687802495` — gate history for T-249.5 contains an append by `pipeline-map-test` with notes equal to `/Users/brett/src/ma-collective/maenifold-rs/todo/T-249.5-pipeline-delete-dataless-fails.md`.

**Scope:** This is verified for tool-output → downstream tool-args mapping only. It does not prove structured mapping into prompt nodes.

### ✅ Conditional fanout with skipped non-match branch

Conditional edges can route from a JSON-producing node. With the T-249.6 fix deployed, a non-matching conditional-only branch becomes `skipped`, not `abandoned`, and the overall pipeline can still complete.

Verified DAG:

```text
read-task (maenifold.gate_TaskBody)
  -- taskId == T-249.6 --> append-match (runs)
  -- taskId == NOPE    --> append-miss  (skipped)
```

Observed result:

```text
read-task      completed
append-match   completed
append-miss    skipped (conditional predicate not satisfied)
pipeline       completed
```

**Verified by:** Pipeline `conditional-proof-skip-1777688125879` — gate history for T-249.6 contains only the matching branch append by `pipeline-conditional-match`; there is no append from `pipeline-conditional-miss`.

**Required precondition:** T-249.6 fix must be deployed. Before that fix, non-matching conditional branches became `abandoned` and made the whole pipeline `abandoned`.

### ✅ Conditional edges based on agent output

Conditional edges can route from a `prompt` node's JSON output. The prompt node output must be valid JSON with the condition path at the top level.

Verified DAG:

```text
judge (prompt, Copilot, gpt-5.5, toolMode disabled)
  output: {"verdict":"PASS","marker":"agent-conditional-proof"}
  -- verdict == PASS --> append-pass (runs)
  -- verdict == FAIL --> append-fail (skipped)
```

Observed result:

```text
judge        completed
append-pass  completed
append-fail  skipped (conditional predicate not satisfied)
pipeline     completed
```

**Verified by:** Pipeline `agent-conditional-proof-1777688863168` — `judge` returned exact JSON, the PASS branch appended to T-249.6 as `pipeline-agent-conditional-pass`, the FAIL branch skipped, and the overall pipeline completed.

**Important cwd constraint:** This proof worked only after omitting node `cwd`. The prompt node opened the ACP session at `/Users/brett/maenifold`.

### ✅ Tool output reaches downstream prompt nodes as predecessor context

A downstream `prompt` node receives predecessor output as context. This is separate from edge `mapping`; no mapping object is needed for the prompt to see the previous tool result.

Verified DAG:

```text
read-task (maenifold.gate_TaskBody)
  -- success -->
extract (prompt, Copilot, gpt-5.5, toolMode disabled)
  output: {"taskId":"T-249.6","sawPipeline":true}
  -- taskId == T-249.6 --> append-match (runs)
  -- taskId == NOPE    --> append-miss  (skipped)
```

Observed result:

```text
read-task     completed
extract       completed
append-match  completed
append-miss   skipped (conditional predicate not satisfied)
pipeline      completed
```

**Verified by:** Pipeline `prompt-context-proof-1777689070577` — `gate_TaskBody` output reached the downstream prompt; the prompt extracted `taskId: "T-249.6"` and `sawPipeline: true`; the matching branch appended to T-249.6 as `pipeline-prompt-context-match`.

**Important output-shape constraint:** Prompt instructions must use actual upstream output keys. `prompt-context-proof-1777688991037` asked for nonexistent `canonicalTaskId`; the prompt returned `null` for that key even though it saw the predecessor context.

## Known Limitations (verified failures)

### ❌ Single gate-done node cannot fast-forward through stages (RESOLVED)

A pipeline with only two gate tool nodes (gate-start at pending→swe, gate-done at end) leaves the gate state at whatever stage the second node tried to set. Gate's `gate_Pass` only advances ONE stage per call.

**Resolution:** Use 4 gate transition tool nodes for SDD, one between each agent stage. See verified 7-node template above. Pipeline `sprint-full-1777686582893` proved this works.

### ❌ Tool name format `gate.pass` (without prefix) fails

The pipeline executor splits tool names on `.`. A name like `gate.pass` is interpreted as server=`gate`, tool=`pass`, which fails: "MCP server 'gate' not found in global config". The correct format is `maenifold.gate_Pass`.

### ❌ Tool name `GatewayExecute` from a tool node fails

Wrapping gate calls inside `GatewayExecute` from a tool node panicked with the runtime nesting bug (now fixed). Even with the fix, this is the wrong path — call the registered tool directly via `maenifold.gate_Pass`.

### ❌ Conditional fanout marked non-matching branches abandoned before T-249.6

Before T-249.6, conditional edges could match source JSON output and run the selected branch, but a non-matching conditional-only target was marked `abandoned` with `conditional predicate not satisfied`. That made the overall pipeline status `abandoned` even when the matching branch completed.

**Verified by:** Pipeline `conditional-proof-1777687858822`.

Observed result:

```text
read-task      completed
append-match   completed
append-miss    abandoned (conditional predicate not satisfied)
pipeline       abandoned
```

**Resolution:** Deploy T-249.6. `conditional-proof-skip-1777688125879` proves the fixed behavior.

### ❌ Prompt node `cwd` outside maenifold root gets stuck before inference

A prompt node with `cwd: "/Users/brett/src/ma-collective/maenifold-rs"` was rejected by the pipeline driver before any agent session produced output:

```text
Pipeline node cwd '/Users/brett/src/ma-collective/maenifold-rs' escapes maenifold root '/Users/brett/maenifold'
```

**Verified by:** Pipeline `agent-conditional-proof-1777688785604` — `judge`, `append-pass`, and `append-fail` all became `stuck`; no agent output was produced.

**Current working pattern:** Omit `cwd` from prompt nodes. Pipeline `agent-conditional-proof-1777688863168` proved that the same prompt and conditional routing work without node `cwd`.

## Untested (do not document until verified)

- Edge `mapping` into prompt-node structured fields or prompt arguments
- Rewind / remediation cycles via failure routing
- Parallel lanes with file contention isolation
- Cross-pipeline join / fork / graft topology operations
- Nested pipelines

## Working Pipeline Definition Template (linear SDD, single task)

This is the minimum verified template. Extend as new patterns are tested.

```json
{
  "nodes": [
    {
      "id": "gate-start",
      "type": "tool",
      "toolName": "maenifold.gate_Pass",
      "toolArgs": {
        "root": "<repo-root>",
        "taskId": "<task-id>",
        "agent": "pipeline",
        "notes": "Pipeline dispatch: pending -> swe"
      }
    },
    {
      "id": "swe",
      "type": "prompt",
      "prompt": "<full SWE instructions including spec content>",
      "agent": "Copilot",
      "toolMode": "auto"
    },
    {
      "id": "red-team",
      "type": "prompt",
      "prompt": "<full red-team instructions>",
      "agent": "Copilot",
      "toolMode": "auto"
    },
    {
      "id": "blue-team",
      "type": "prompt",
      "prompt": "<full blue-team instructions>",
      "agent": "Copilot",
      "toolMode": "auto"
    }
  ],
  "edges": [
    {"source": "gate-start", "target": "swe", "type": "success"},
    {"source": "swe", "target": "red-team", "type": "success"},
    {"source": "red-team", "target": "blue-team", "type": "success"}
  ]
}
```

## Operations

### Create
```
maenifold tool Pipeline '{"name":"<prefix>","definition":<defn>,"maxConcurrency":N}'
```

### Run autonomously
```
maenifold tool Pipeline '{"sessionId":"<id>","run":true,"prompt":"<ambient context>"}'
```

### View state
```
maenifold tool Pipeline '{"sessionId":"<id>","view":true}'
```

### Cancel running pipeline
```
maenifold tool Pipeline '{"sessionId":"<id>","cancel":true}'
```

### Delete (must cancel first if running)
```
maenifold tool Pipeline '{"sessionId":"<id>","delete":true}'
```

For completed pipelines that refuse delete, use `force:true`.

## Process for Adding New Verified Capabilities

When you want to use a new pipeline feature:

1. Build a minimal test DAG that exercises ONLY that feature
2. Run it against a real task
3. Verify the outcome matches your expectation
4. Document the verified pattern here
5. If it fails, document the failure mode here (not the workaround you guessed)

Never write speculative documentation. Run the experiment first.

## Reference

For exact Pipeline tool parameters:
```
maenifold tool Pipeline '{"learn":true}'
```
