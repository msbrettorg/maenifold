
# Sequential Thinking

A detailed tool for dynamic and reflective problem-solving through thoughts. This tool helps analyze problems through a flexible thinking process that can adapt and evolve. Each thought can build on, question, or revise previous insights as understanding deepens.

When to use this tool:
- Breaking down complex problems into steps
- Planning and design with room for revision
- Analysis that might need course correction
- Problems where the full scope might not be clear initially
- Problems that require a multi-step solution
- Tasks that need to maintain context over multiple steps
- Situations where irrelevant information needs to be filtered out

Key features:
- You can adjust total_thoughts up or down as you progress
- You can question or revise previous thoughts
- You can add more thoughts even after reaching what seemed like the end
- You can express uncertainty and explore alternative approaches
- Not every thought needs to build linearly - you can branch or backtrack
- Generates a solution hypothesis
- Verifies the hypothesis based on the Chain of Thought steps
- Repeats the process until satisfied
- Provides a correct answer
- Supports workflow linking and graph persistence in maenifold
- Auto-creates `session-{timestamp}` when no sessionId is provided
- Returns continuation/status text with periodic checkpoint reminders (every 3 thoughts, and on the first thought)
- New sessions must start at `thoughtNumber=0`; calling with `thoughtNumber=1` without an existing session errors

Parameters explained:
- response: Main thought content; MUST include [[concepts]] unless cancelling
- thoughts: Optional meta/ambient notes; also must include [[concepts]] if provided
- thought: Your current thinking step, which can include:
  * Regular analytical steps
  * Revisions of previous thoughts
  * Questions about previous decisions
  * Realizations about needing more analysis
  * Changes in approach
  * Hypothesis generation
  * Hypothesis verification
- nextThoughtNeeded: True if you need more thinking, even if at what seemed like the end
- thoughtNumber: Current number in sequence (start new sessions at 0; can go beyond initial total if needed)
- totalThoughts: Current estimate of thoughts needed (can be adjusted up/down)
- isRevision: A boolean indicating if this thought revises previous thinking
- revisesThought: If is_revision is true, which thought number is being reconsidered
- branchFromThought: If branching, which thought number is the branching point
- branchId: Identifier for the current branch (if any)
- needsMoreThoughts: If reaching end but realizing more thoughts needed
- sessionId: Continue an existing session; omit to start a new one
- analysisType: Optional annotation (bug, architecture, retrospective, complex)
- parentWorkflowId: Optional; only on thought 1; links to an active workflow and creates a [[workflow/{id}]] back-link
- conclusion: Required when nextThoughtNeeded is false. Must include [[concepts]] AND confession elements:
  1. Synthesize findings
  2. List instruction compliance (✅/❌ with evidence)
  3. Shortcuts or hacks taken
  4. Risks/uncertainties flagged
  5. Sources used (memory:// URIs, [[concepts]])
- cancel: Set to true to cancel a session; skips concept validation and conclusion
- learn: Set to true to return this help text instead of executing

maenifold specifics:
- response/thoughts MUST include [[concepts]]; conclusion with [[concepts]] and confession structure is required when nextThoughtNeeded is false
- Sessions persist to memory://thinking/sequential/{sessionId}.md with agent tag, timestamps, and frontmatter
- New sessions start at thoughtNumber=0 (sessionId auto-created). thoughtNumber>0 requires the session to exist unless isRevision is true; an existing sessionId with thoughtNumber=1 is rejected unless revising
- branchId is required when branchFromThought is set (multi-agent safety)
- parentWorkflowId can be set only on the first thought and must reference an active workflow; creates a [[workflow/{id}]] back-link
- needsMoreThoughts extends the displayed total when you exceed the estimate
- Completion appends the conclusion and sets status=completed; cancel sets status=cancelled
- Output is a status string (created/added thought plus continuation or completion cues, with checkpoint hints every 3 thoughts and on the first thought)
- If sessionId is omitted, a new `session-{timestamp}` is created automatically
- cancel=true skips concept validation and conclusion requirements

Common errors (expect these if violated):
- Missing [[concepts]] in response/thoughts → `ERROR: Must include [[concepts]]. Example: 'Analyzing [[Machine Learning]] algorithms'`
- Missing conclusion when nextThoughtNeeded=false → `ERROR: Conclusion required when completing session...`
- Conclusion without [[concepts]] → `ERROR: Conclusion must include [[concepts]]...`
- Invalid sessionId format (must be `session-{unix-milliseconds}`) → `ERROR: Invalid sessionId format...`
- Providing sessionId on thought 1 when session does not exist → `ERROR: Session {id} not found. To start new session, don't provide sessionId.`
- Trying thoughtNumber>1 without an existing session → `ERROR: Session {id} missing. Start with thoughtNumber=0.` (use thoughtNumber=0 to create a new session)
- Branching without branchId → `ERROR: branchId required when branchFromThought is specified...`
- Parent workflow on thought>1 or missing/closed workflow → corresponding parent workflow errors

You should:
1. Start with an initial estimate of needed thoughts, but be ready to adjust
2. Feel free to question or revise previous thoughts
3. Don't hesitate to add more thoughts if needed, even at the "end"
4. Express uncertainty when present
5. Mark thoughts that revise previous thinking or branch into new paths
6. Ignore information that is irrelevant to the current step
7. Generate a solution hypothesis when appropriate
8. Verify the hypothesis based on the Chain of Thought steps
9. Repeat the process until satisfied with the solution
10. Provide a single, ideally correct answer as the final output
11. Only set nextThoughtNeeded to false when truly done and a satisfactory answer is reached
