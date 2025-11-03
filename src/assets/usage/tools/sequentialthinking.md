# SequentialThinking

Test-time adaptive reasoning with revision, branching, and multi-agent collaboration. Persists to `memory://thinking/sequential/{session-id}`.

## Parameters

- `response` (string, optional): Current thought/reasoning. MUST contain `[[concepts]]` for graph integration.
- `conclusion` (string, optional): Final synthesis when `nextThoughtNeeded=false`. MUST contain `[[concepts]]`.
- `thoughts` (string, optional): Meta-observations about reasoning process. Use `[[concepts]]` liberally.
- `sessionId` (string, optional): Resume existing session. Omit to start new session.
- `thoughtNumber` (int, default: 0): Current thought index in sequence.
- `totalThoughts` (int, default: 0): Estimated total. Adjustable mid-session.
- `nextThoughtNeeded` (bool, default: false): True to continue reasoning.
- `needsMoreThoughts` (bool, default: false): True when estimate was too low.
- `isRevision` (bool, default: false): True when reconsidering prior thought.
- `revisesThought` (int, optional): Which thought number to revise.
- `branchFromThought` (int, optional): Branching point for alternative paths.
- `branchId` (string, optional): Branch identifier. REQUIRED for multi-agent to prevent conflicts.
- `parentWorkflowId` (string, optional): Link to parent workflow (bidirectional WikiLink).
- `analysisType` (string, optional): `bug`, `architecture`, `retrospective`, or `complex`.
- `cancel` (bool, default: false): True to abort session.

## Returns

```json
{
  "sessionId": "session-1758434799362",
  "uri": "memory://thinking/sequential/2025/session-1758434799362.md",
  "thoughtNumber": 3,
  "totalThoughts": 5,
  "status": "in_progress"
}
```

## Multi-Agent Collaboration

- Multiple agents work on same `sessionId` simultaneously
- Each thought tagged with agent ID + timestamp
- Use `branchId` to prevent conflicts when exploring alternatives
- All revisions tracked chronologically

## Integration

- **Workflow**: Embed thinking in workflow steps via `parentWorkflowId`
- **BuildContext**: Traverse `[[concepts]]` from session files
- **RecentActivity**: Resume sessions with `filter="thinking"`
