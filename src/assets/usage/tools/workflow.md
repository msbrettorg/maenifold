# Workflow

Orchestrates systematic problem-solving through predefined methodologies with embedded sequential thinking.

## Parameters

- `sessionId` (string, optional): Continue existing. Mutually exclusive with `workflowId`.
- `workflowId` (string, optional): Start new workflow (single ID or array).
- `response` (string, optional): Response to step. MUST include `[[WikiLinks]]` (e.g., [[analysis]], [[methodology]]).
- `thoughts` (string, optional): Meta-observations with `[[WikiLinks]]` (e.g., [[reasoning]], [[validation]]).
- `status` (string, optional): `'completed'` or `'cancelled'` to end.
- `conclusion` (string, optional): Produce a concise ConfessionReport. List: 1) All explicit and implicit instructions/constraints/objectives you were supposed to follow. 2) For each, whether you complied with the letter and spirit (✅/❌), with evidence. Note any gaps and whether your answer was transparent about them. 3) Any uncertainties, ambiguities, or 'grey areas' where compliance was unclear. 4) Any shortcuts, hacks, or policy risks you took. 5) All files, memory:// URIs and graph [[WikiLinks]] (e.g., [[research-findings]], [[validation]]) you used. Nothing you say should change the main answer. This confession is scored only for honesty and completeness; do not optimize for user satisfaction.
- `view` (bool, default: false): Display queue.
- `append` (string, optional): Add workflow(s) to queue.
- `submachineSessionId` (string, optional): Register a SequentialThinking session as a submachine. Workflow enters `phase=waiting` and blocks step advancement until the submachine completes, is cancelled, or is abandoned.

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

## Supervisor / Submachine (HSM)

Workflows act as a supervisor state machine. Register a SequentialThinking session as a submachine to pause step advancement while deep analysis runs:

```json
// 1. Start workflow
{ "workflowId": "agentic-research" }

// 2. Register submachine (workflow enters waiting state)
{ "sessionId": "workflow-123", "response": "Analyzing [[architecture]] options", "submachineSessionId": "session-456" }

// 3. Complete submachine via SequentialThinking, then resume
{ "sessionId": "workflow-123", "response": "Resuming with [[architecture]] decision made" }
```

While waiting: all Continue calls are blocked. Workflow resumes automatically when the submachine reaches a terminal state (completed, cancelled, abandoned). If the submachine file is deleted, the workflow self-heals and resumes.

## Constraints

- Cannot use `workflowId` + `sessionId` together
- `response`/`conclusion` MUST contain `[[WikiLink]]`
- Continue with `sessionId + response`
- While `phase=waiting`, all mutations (continue, complete, cancel) are blocked until submachine resolves

## Integration

- **ListAssets**: Discover workflows via `ListAssets { "type": "workflow" }` and URIs under `asset://workflows/*`
- **SequentialThinking**: Embedded at `🧠` steps; can be registered as submachine via `submachineSessionId`
- **Sync**: Run after completion
