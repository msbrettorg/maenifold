# Sequential Thinking

Persistent, revision-friendly sessions with [[concepts]] that write to markdown and the graph.

## Quick start (tested via mcp_maenifold / `maenifold --tool`)
- Server: `mcp_maenifold` tool `SequentialThinking` (alias `sequentialthinking`). Same payload works with `maenifold --tool SequentialThinking --payload '<json>'`.
- Start: call with `thoughtNumber=0`, `nextThoughtNeeded=true`, and a response containing [[concepts]]. SessionId is auto-generated. (Calling with `thoughtNumber=1` without an existing session will error.)  
  Example: `MAENIFOLD_ROOT=$PWD/.tmp/maenifold-skill-test AGENT_ID=codex-max src/bin/Debug/net9.0/maenifold --tool SequentialThinking --payload '{"response":"Kickoff [[sequential-thinking]] skill doc","nextThoughtNeeded":true,"thoughtNumber":0,"totalThoughts":3}'`
- Continue: increment `thoughtNumber`. Use `thoughtNumber=1` only when continuing an existing session and include a `branchId` (e.g., `"main"`); starting at 1 without a session will be rejected. Example payload addition: `"sessionId":"<returned id>","branchId":"main","thoughtNumber":1`.
- Complete: set `nextThoughtNeeded=false` and pass `response` **and** `conclusion` with [[concepts]]. Example payload addition: `"thoughtNumber":2,"response":"Finalizing [[sequential-thinking]] doc tests","conclusion":"Synthesized [[sequential-thinking]] flow with tested [[conclusion-requirements]]","sessionId":"<id>","branchId":"main"`.
- Output returns a status string (Created/Added, continue cue, checkpoints every 3 thoughts starting at the first).

### Tested CLI flow (mcp_maenifold)
```bash
# Start (auto sessionId)
MAENIFOLD_ROOT=$PWD/.tmp/seq-skill-test AGENT_ID=codex \
  src/bin/Debug/net9.0/maenifold --tool SequentialThinking --payload '{
    "response":"Start run [[sequential-thinking]] [[mcp-maenifold]]",
    "nextThoughtNeeded":true,
    "thoughtNumber":0,
    "totalThoughts":2
  }'

# Continue (thoughtNumber=1 on existing session requires branchId)
MAENIFOLD_ROOT=$PWD/.tmp/seq-skill-test AGENT_ID=codex \
  src/bin/Debug/net9.0/maenifold --tool SequentialThinking --payload '{
    "sessionId":"<session-id>",
    "branchId":"main",
    "response":"Progress [[sequential-thinking]] [[mcp-maenifold]]",
    "nextThoughtNeeded":true,
    "thoughtNumber":1
  }'

# Complete (response + conclusion still need [[concepts]])
MAENIFOLD_ROOT=$PWD/.tmp/seq-skill-test AGENT_ID=codex \
  src/bin/Debug/net9.0/maenifold --tool SequentialThinking --payload '{
    "sessionId":"<session-id>",
    "branchId":"main",
    "response":"Wrap up [[sequential-thinking]] [[mcp-maenifold]]",
    "conclusion":"Completed [[sequential-thinking]] session with tested [[mcp-maenifold]] flow",
    "nextThoughtNeeded":false,
    "thoughtNumber":2
  }'
```

## Parameter notes (tested)
- `response` / `thoughts`: must include [[concepts]] unless `cancel=true` (still required on the final call, otherwise you get the missing [[concepts]] error). `learn=true` returns this help text.
- `thoughtNumber`: start at 0 for new sessions (sessionId auto `session-{unix-ms}`). `thoughtNumber>0` requires an existing session; include `branchId` when using `thoughtNumber=1` on an existing session.
- `branchFromThought`: requires `branchId`; use branches for multi-agent safety.
- `parentWorkflowId`: only on the first thought; workflow must exist and be active; creates a `[[workflow/{id}]]` back-link.
- `needsMoreThoughts`: extends the displayed total when you blow past the estimate.
- Completion requires `conclusion` with [[concepts]]; cancel marks the session cancelled without a conclusion.

## Storage
- Sessions persist to `MAENIFOLD_ROOT/memory/thinking/sequential/YYYY/MM/DD/{sessionId}.md` with frontmatter (`status`, `thoughtCount`, timestamps) and per-thought sections tagged by `AGENT_ID`.

## Common errors & resolutions (tested)
- `ERROR: Must include [[concepts]]. Example: 'Analyzing [[Machine Learning]] algorithms'` → add at least one [[concept]] in `response` or `thoughts`.
- `ERROR: Conclusion required when completing session...` → provide `conclusion` when `nextThoughtNeeded=false`.
- `ERROR: Conclusion must include [[concepts]] for knowledge graph integration.` → add [[concepts]] inside `conclusion`.
- `ERROR: branchId required when branchFromThought is specified for multi-agent coordination` → supply `branchId` whenever `branchFromThought` is set.
- `ERROR: Session session-{id} not found. To start new session, don't provide sessionId.` when calling with `thoughtNumber=1` and no existing session → start with `thoughtNumber=0`.
- `ERROR: Session {id} missing. Start with thoughtNumber=0.` when calling with `thoughtNumber>1` and no existing session → create at `thoughtNumber=0` first.
- `ERROR: Session {id} exists. Use different sessionId or continue existing.` when continuing with `thoughtNumber=1` and no `branchId` → include a `branchId` (e.g., `"main"`) or move to `thoughtNumber=2+`.
