# SequentialThinking

Persistent, branching thought sessions with `[[WikiLink]]` graph integration and markdown file storage.

## When to Use

- Breaking down complex problems into revisable steps
- Planning and design with room for course correction
- Analysis where full scope is unclear initially
- Multi-step reasoning that must persist across sessions
- Multi-agent collaboration on shared thought chains
- Hypothesis generation and verification loops

## Parameters

- `response` (string, optional): Main thought content. MUST include `[[WikiLinks]]` unless cancelling. Represents the current thinking step: analysis, revision, question, realization, hypothesis, or verification.
- `nextThoughtNeeded` (bool, optional): `true` if more thinking is needed (default: `false`). Set `false` only when truly done.
- `thoughtNumber` (int, optional): Current position in sequence (default: `0`). New sessions MUST start at `0`. Can exceed `totalThoughts` if needed.
- `totalThoughts` (int, optional): Estimated total thoughts needed (default: `0`). Adjust up or down freely as understanding evolves.
- `sessionId` (string, optional): Continue an existing session. Omit to auto-create `session-{timestamp}-{random}`. Format must be `session-{unix-milliseconds}[-suffix]`.
- `cancel` (bool, optional): Cancel the session (default: `false`). Skips `[[WikiLink]]` validation and conclusion requirement.
- `thoughts` (string, optional): Ambient/meta observations. MUST include `[[WikiLinks]]` if provided.
- `isRevision` (bool, optional): Marks this thought as revising previous thinking (default: `false`).
- `revisesThought` (int, optional): Which thought number is being reconsidered. Use with `isRevision=true`.
- `branchFromThought` (int, optional): Thought number to branch from. Requires `branchId`.
- `branchId` (string, optional): Identifier for the branch. Required when `branchFromThought` is set (multi-agent safety).
- `needsMoreThoughts` (bool, optional): Signal that more thoughts are needed beyond `totalThoughts` (default: `false`). Extends displayed total.
- `analysisType` (string, optional): Annotation label. One of: `"bug"`, `"architecture"`, `"retrospective"`, `"complex"`.
- `parentWorkflowId` (string, optional): Links session to an active workflow. Only valid on the first thought (`thoughtNumber=0`). Creates a `[[workflow/{id}]]` back-link.
- `conclusion` (string, optional): Required when `nextThoughtNeeded=false` (unless cancelling). MUST include `[[WikiLinks]]` AND confession elements:
  1. Synthesize findings
  2. List instruction compliance (check/cross with evidence)
  3. Shortcuts or hacks taken
  4. Risks/uncertainties flagged
  5. Sources used (memory:// URIs, `[[WikiLinks]]`)
## Returns

### New session (thoughtNumber=0)

```
Created session: session-1756610546730-48291

💭 Continue with thought 1/5
💡 **CHECK YOUR MEMORY:** search_memories for what exists and build_context on [[WikiLinks]] | sync new findings to add them to the graph
```

### Continuation (thoughtNumber > 0)

```
Added thought 3 to session: session-1756610546730-48291

💭 Continue with thought 4/5
```

### Completion (nextThoughtNeeded=false)

```
Added thought 5 to session: session-1756610546730-48291

✅ Thinking complete
```

### Cancellation (cancel=true)

```
Added thought 2 to session: session-1756610546730-48291

❌ Thinking cancelled
```

Checkpoint hints appear on the first thought and every 3 thoughts thereafter.

## Examples

**Start a new session (omit sessionId):**
```json
{
  "response": "Analyzing [[authentication]] flow for [[JWT]] token refresh",
  "nextThoughtNeeded": true,
  "thoughtNumber": 0,
  "totalThoughts": 5
}
```

**Continue an existing session:**
```json
{
  "sessionId": "session-1756610546730-48291",
  "response": "The [[token-refresh]] mechanism needs [[rate-limiting]] to prevent abuse",
  "nextThoughtNeeded": true,
  "thoughtNumber": 2,
  "totalThoughts": 5
}
```

**Revise a previous thought:**
```json
{
  "sessionId": "session-1756610546730-48291",
  "response": "Reconsidering [[rate-limiting]] - should use [[sliding-window]] instead of fixed",
  "nextThoughtNeeded": true,
  "thoughtNumber": 3,
  "totalThoughts": 5,
  "isRevision": true,
  "revisesThought": 2
}
```

**Branch from a thought (multi-agent):**
```json
{
  "sessionId": "session-1756610546730-48291",
  "response": "Exploring [[caching]] strategy as alternative to [[rate-limiting]]",
  "nextThoughtNeeded": true,
  "thoughtNumber": 3,
  "totalThoughts": 5,
  "branchFromThought": 2,
  "branchId": "T-2.1.2-swe"
}
```

**Complete with conclusion:**
```json
{
  "sessionId": "session-1756610546730-48291",
  "response": "Final verification of [[authentication]] design confirms [[JWT]] approach",
  "nextThoughtNeeded": false,
  "thoughtNumber": 5,
  "totalThoughts": 5,
  "conclusion": "Determined [[sliding-window]] [[rate-limiting]] for [[JWT]] refresh. ✅ Followed [[authentication]] best practices. No shortcuts. Risk: [[token-revocation]] latency under load. Sources: memory://tech/auth-patterns.md"
}
```

**Link to parent workflow:**
```json
{
  "response": "Starting [[architecture]] review linked to active workflow",
  "nextThoughtNeeded": true,
  "thoughtNumber": 0,
  "totalThoughts": 4,
  "parentWorkflowId": "session-1756610500000-12345",
  "analysisType": "architecture"
}
```

**Cancel a session:**
```json
{
  "sessionId": "session-1756610546730-48291",
  "cancel": true
}
```

## Constraints

- **`[[WikiLinks]]` required**: `response` and `thoughts` MUST contain at least one `[[WikiLink]]` (unless cancelling). `conclusion` MUST also include `[[WikiLinks]]`.
- **New sessions start at `thoughtNumber=0`**: Calling `thoughtNumber > 0` without an existing session errors.
- **`sessionId` on `thoughtNumber=0`**: Providing a `sessionId` that does not exist returns a "session not found" error. Omit `sessionId` to auto-create.
- **`branchId` required with `branchFromThought`**: Branching without a branch identifier errors (multi-agent safety).
- **`parentWorkflowId` only on first thought**: Setting it on `thoughtNumber > 0` errors. The referenced workflow must exist and be active (not completed/cancelled/abandoned).
- **`conclusion` required on completion**: When `nextThoughtNeeded=false` and `cancel=false`, `conclusion` must be provided with `[[WikiLinks]]` and the confession structure.
- **Session ID format**: Must match `session-{unix-milliseconds}[-optional-suffix]`. Invalid formats are rejected.
- **Session persistence**: Sessions persist to `memory://thinking/sequential/{sessionId}.md` with agent tags, timestamps, and frontmatter.

## Common Errors

| Condition | Error |
|-----------|-------|
| No `[[WikiLinks]]` in response/thoughts | `WIKILINK_REQUIRED` |
| Missing conclusion on completion | `CONCLUSION_REQUIRED` |
| Conclusion without `[[WikiLinks]]` | `CONCLUSION_WIKILINK_REQUIRED` |
| Invalid sessionId format | `INVALID_SESSION_ID` |
| sessionId provided but session not found | `SESSION_NOT_FOUND` |
| `thoughtNumber > 0` without existing session | `SESSION_MISSING` |
| `thoughtNumber=0` with existing sessionId (no branch/revision) | `SESSION_EXISTS` |
| `branchFromThought` without `branchId` | `BRANCH_ID_REQUIRED` |
| `parentWorkflowId` on `thoughtNumber > 0` | `INVALID_PARENT_WORKFLOW` |
| `parentWorkflowId` referencing missing workflow | `PARENT_WORKFLOW_NOT_FOUND` |
| `parentWorkflowId` referencing closed workflow | `PARENT_WORKFLOW_CLOSED` |

## Integration

- **Workflow**: Embedded at workflow steps; link via `parentWorkflowId` for bidirectional references
- **SearchMemories**: Find existing knowledge before starting a thinking session
- **BuildContext**: Traverse `[[WikiLinks]]` discovered during thought steps
- **AssumptionLedger**: Log assumptions before sessions, validate after completion
- **Sync**: Rebuild graph after completing sessions to index new `[[WikiLinks]]`
