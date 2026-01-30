# Workflow

Orchestrates systematic problem-solving through predefined methodologies with embedded sequential thinking.

## Parameters

- `sessionId` (string, optional): Continue existing. Mutually exclusive with `workflowId`.
- `workflowId` (string, optional): Start new workflow (single ID or array).
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
- `response`/`conclusion` MUST contain `[[WikiLink]]`
- Continue with `sessionId + response`

## Integration

- **ListAssets**: Discover workflows via `ListAssets { "type": "workflow" }` and URIs under `asset://workflows/*`
- **SequentialThinking**: Embedded at `🧠` steps
- **Sync**: Run after completion
