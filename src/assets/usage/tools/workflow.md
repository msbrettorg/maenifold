# Workflow

Orchestrates systematic problem-solving through predefined methodologies with embedded sequential thinking.

## Parameters

- `sessionId` (string, optional): Continue existing. Mutually exclusive with `workflowId`.
- `workflowId` (string, optional): Start new workflow (single ID or array).
- `response` (string, optional): Response to step. MUST include `[[concepts]]`.
- `thoughts` (string, optional): Meta-observations with `[[concepts]]`.
- `status` (string, optional): `'completed'` or `'cancelled'` to end.
- `conclusion` (string, optional): Required when `status='completed'`. MUST include `[[concepts]]`.
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

- **ListWorkflows**: Discover via `asset://workflows/*`
- **SequentialThinking**: Embedded at `🧠` steps
- **Sync**: Run after completion
