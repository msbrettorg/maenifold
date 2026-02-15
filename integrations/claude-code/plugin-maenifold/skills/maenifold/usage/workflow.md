
# Workflow

Orchestrates systematic problem-solving through predefined methodologies with embedded sequential thinking.

## Non-negotiable: follow the workflow (no skipping)

This tool runs **structured, multi-step workflows**. When using it, **execute the workflow step-by-step**.

**Do not** read the first step, jump to a conclusion, and abandon the workflow. That is a failure mode.

When a workflow is active:
- Execute steps **in order**.
- After each step, **call `Workflow` again** using the returned `sessionId`.
- Only produce a "final answer" when the workflow explicitly indicates, or upon completion with `status='completed'` + `conclusion`.

If the user asks for a workflow, stay in the workflow until it is **completed** or **cancelled**.

## How to use (the loop)

1. **Start** a workflow:
  - Call `Workflow` with `workflowId` (and optionally an initial `response`/`thoughts`).
2. **Read the current step** from the tool output.
3. **Do exactly what the step asks**, then provide a `response` that includes at least one `[[WikiLink]]`.
4. **Continue**:
  - Call `Workflow` with `sessionId` + the `response`.
5. Repeat until the workflow indicates completion.
6. **Complete**:
  - Call `Workflow` with `status='completed'` and provide a `conclusion` (with the required confession elements).

Use `view: true` to show the workflow queue for visibility into what is pending.

## Parameters

- `sessionId` (string, optional): Continue existing. Mutually exclusive with `workflowId`.
- `workflowId` (string | string[], optional): Start new workflow (single ID or array).
- `response` (string, optional): Response to step. MUST include `[[WikiLinks]]`.
- `thoughts` (string, optional): Meta-observations with `[[WikiLinks]]`.
- `status` (string, optional): `'completed'` or `'cancelled'` to end.
- `conclusion` (string, optional): Required when `status='completed'`. MUST include `[[WikiLinks]]` AND confession elements:
  1. Synthesize findings
  2. List instruction compliance (✅/❌ with evidence)
  3. Shortcuts or hacks taken
  4. Risks/uncertainties flagged
  5. Sources used (memory:// URIs, [[WikiLinks]])
- `view` (bool, default: false): Display queue.
- `append` (string, optional): Add workflow(s) to queue.

## Returns

Structured guidance: current step, tool hints, quality gates, progress, session persistence.

## Example

```json
{
  "workflowId": "agentic-dev",
  "response": "[[microservices]] over [[monolithic]] for [[scalability]]",
  "thoughts": "[[GraphRAG]] opportunities"
}
```

## Constraints

- Cannot use `workflowId` + `sessionId` together
- `response`/`conclusion` MUST contain at least one `[[WikiLink]]`
- Continue with `sessionId + response` (repeat until completion)
- Do not skip steps or “jump ahead” to a conclusion; follow the workflow’s step order

## Integration

- **ListAssets**: Discover workflows via `ListAssets { "type": "workflow" }` and URIs under `asset://workflows/*`
- **SequentialThinking**: Embedded at `🧠` steps
- **Sync**: Run after completion
