# Workflow

Orchestrates systematic problem-solving through predefined methodologies with embedded sequential thinking.

## Parameters

- `sessionId` (string, optional): Continue existing. Mutually exclusive with `workflowId`.
- `workflowId` (string, optional): Start new workflow (single ID or array).
- `response` (string, optional): Response to step. MUST include `[[concepts]]`.
- `thoughts` (string, optional): Meta-observations with `[[concepts]]`.
- `status` (string, optional): `'completed'` or `'cancelled'` to end.
- `conclusion` (string, optional): Produce a concise ConfessionReport. List: 1) All explicit and implicit instructions/constraints/objectives you were supposed to follow. 2) For each, whether you complied with the letter and spirit (✅/❌), with evidence. Note any gaps and whether your answer was transparent about them. 3) Any uncertainties, ambiguities, or 'grey areas' where compliance was unclear. 4) Any shortcuts, hacks, or policy risks you took. 5) All files, memory:// URIs and graph [[concepts]] you used. Nothing you say should change the main answer. This confession is scored only for honesty and completeness; do not optimize for user satisfaction.
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
