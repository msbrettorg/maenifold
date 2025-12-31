# Workflow

Execute structured workflow(s) for systematic problem-solving and complex orchestration.
  This tool runs predefined workflows that guide you through proven methodologies with embedded sequential thinking, multi-agent coordination, and sophisticated process control.
  Each workflow provides structured steps, tool hints, reasoning requirements, and quality gates for specific problem types.
  REQUIRES [[concepts]] in responses to build connections in the knowledge graph!

## When to Use This Tool
  - Complex multi-step processes requiring orchestration
  - Following established methodologies (design thinking, agile, SLC development)
  - Multi-agent coordination with parallel execution waves
  - Systematic analysis requiring structured approaches with embedded thinking
  - Quality-gated processes with validation checkpoints
  - Sprint execution with RTM (Requirements Traceability Matrix) validation
  - Team collaboration using shared frameworks and workflows
  - When you need proven workflows with embedded sequential thinking vs pure free-form exploration

  Key features:
  - 42+ predefined workflows covering various domains and methodologies
  - **Embedded sequential thinking** - workflows specify when to use SequentialThinking tool
  - Multi-agent coordination with parallel task dispatch
  - Queue-based execution - run multiple workflows in sequence
  - Session persistence across workflow steps with SequentialThinking integration
  - Tool hints and orchestration metadata for each step
  - Quality gates, guardrails, and escape hatches
  - RTM validation for sprint workflows
  - Git integration and tracking for development workflows
  - Enhanced thinking requirements with reasoning effort control
  - Process control with stop conditions and next actions
  - Integration with maenifold ecosystem (CodeNav, SearchMemories, WriteMemory, etc.)

  Workflow orchestration capabilities:
  - Wave-based execution (discovery wave, validation wave, implementation wave)
  - Parallel agent dispatch (coding-agent, test-agent, desloppification-agent)
  - Cross-tool coordination (CodeNav + SequentialThinking + WriteMemory)
  - Git workflow integration with branch management
  - Requirements traceability with RTM tracking
  - Quality validation with build/test gates
  - Retrospective analysis with pattern extraction

  Available workflow types include:
  - Development methodologies (agentic-dev, agentic-slc sprint orchestration)
  - Design frameworks (design-thinking, lean-startup)
  - Analysis methodologies (critical-thinking, strategic-thinking)
  - Reasoning approaches (deductive, inductive, abductive)
  - Creative processes (divergent-thinking, oblique-strategies)
  - Collaboration methods (world-cafe, parallel-thinking)
  - Software development (agile, sdlc)

  Parameters explained:
  - sessionId: Continue existing workflow session (maintains state across complex orchestration)
  - workflowId: Start new workflow(s) - single ID or JSON array for multi-workflow execution
  - response: Your response to current workflow step - MUST include [[concepts]]
  - thoughts: Meta-observations about workflow progress and orchestration insights
  - status: Set to 'completed' or 'cancelled' to end session
  - view: Display current queue status and orchestration progress
  - append: Add workflow(s) to existing session queue for extended orchestration

  Workflow vs Sequential relationship:
  - **Workflow embeds Sequential**: Many workflow steps specify `🧠 USE SEQUENTIAL THINKING TOOL`
  - **Workflow orchestrates**: Coordinates multiple tools, agents, and thinking sessions
  - **Sequential executes**: Provides iterative thinking within workflow steps
  - **Use Workflow**: For complex orchestrated processes with multiple phases
  - **Use Sequential**: For focused iterative thinking within a single problem space
  - **Combined power**: Workflows specify when Sequential thinking is required with estimated thoughts, focus areas, and reasoning effort

  Advanced orchestration features:
  - ToolHints metadata specifying required tools and operations
  - Reasoning effort classification (low/medium/high)
  - Stop conditions and quality gates
  - Guardrails preventing scope creep and ensuring quality
  - Enhanced thinking requirements with systematic validation
  - Multi-agent collaboration patterns with shared sessions
  - Git discipline and commit tracking
  - RTM (Requirements Traceability Matrix) compliance
  - Escape hatches for ambiguous requirements

  You should:
  1. Use ListAssets (type=workflow) to discover available workflows from asset://workflows/* resources
  2. Choose appropriate workflow(s) for your problem complexity
  3. Follow embedded SequentialThinking requirements when specified
  4. Provide thoughtful responses with [[concepts]] at each orchestration step
  5. Complete workflows fully rather than abandoning mid-orchestration
  6. Use multi-workflow approaches for comprehensive systematic analysis
  7. Respect tool hints and quality gates built into workflow steps
  8. Leverage maenifold ecosystem integration (CodeNav, SearchMemories, etc.)

  maenifold Integration:
  - Workflows coordinate maenifold's full tool ecosystem
  - SearchMemories and WriteMemory for knowledge graph integration
  - BuildContext for concept relationship exploration
  - Multi-agent dispatch with Task tool coordination
  - Git workflow management and tracking
  - RTM-driven development with traceability validation

## Tested CLI flow (mcp_maenifold)
Tested end-to-end with `higher-order-thinking` workflow and embedded SequentialThinking sessions.
```bash
# Start workflow
MAENIFOLD_ROOT=$PWD/.tmp/workflow-skill-test AGENT_ID=codex \
  src/bin/Debug/net9.0/maenifold --tool Workflow --payload '{
    "workflowId":"higher-order-thinking",
    "response":"Kickoff [[higher-order-thinking]] workflow with sequential thinking",
    "thoughts":"Aligning with instructions [[workflow-testing]]"
  }'

# Sequential thinking for a step (multi-thought; branchId needed at thoughtNumber=1)
MAENIFOLD_ROOT=$PWD/.tmp/workflow-skill-test AGENT_ID=codex \
  src/bin/Debug/net9.0/maenifold --tool SequentialThinking --payload '{
    "response":"Thought 0: map heuristics and biases [[higher-order-thinking]] [[workflow-testing]]",
    "nextThoughtNeeded":true,
    "thoughtNumber":0,
    "totalThoughts":3
  }'
MAENIFOLD_ROOT=$PWD/.tmp/workflow-skill-test AGENT_ID=codex \
  src/bin/Debug/net9.0/maenifold --tool SequentialThinking --payload '{
    "sessionId":"<seq-session-id>",
    "branchId":"main",
    "response":"Thought 1: adjust anchors [[higher-order-thinking]] [[workflow-testing]]",
    "nextThoughtNeeded":true,
    "thoughtNumber":1,
    "totalThoughts":3,
    "thoughts":"Bias check [[cognitive-bias]]"
  }'
MAENIFOLD_ROOT=$PWD/.tmp/workflow-skill-test AGENT_ID=codex \
  src/bin/Debug/net9.0/maenifold --tool SequentialThinking --payload '{
    "sessionId":"<seq-session-id>",
    "branchId":"main",
    "response":"Thought 2: finalize meta-approach [[higher-order-thinking]] [[workflow-testing]]",
    "conclusion":"Conclusion: sharper meta-awareness and bias checks [[higher-order-thinking]] [[metacognition]]",
    "nextThoughtNeeded":false,
    "thoughtNumber":2,
    "totalThoughts":3
  }'

# Advance workflow step (repeat sequential thinking for steps requiring it)
MAENIFOLD_ROOT=$PWD/.tmp/workflow-skill-test AGENT_ID=codex \
  src/bin/Debug/net9.0/maenifold --tool Workflow --payload '{
    "sessionId":"<workflow-id>",
    "response":"Used sequential session <seq-session-id> to surface heuristics [[higher-order-thinking]] [[workflow-testing]]",
    "thoughts":"Branch guard hit at thought 1 requires branchId main [[process]]"
  }'

# Complete workflow (after all steps)
MAENIFOLD_ROOT=$PWD/.tmp/workflow-skill-test AGENT_ID=codex \
  src/bin/Debug/net9.0/maenifold --tool Workflow --payload '{
    "sessionId":"<workflow-id>",
    "response":"Set monitoring loop with checkpoints [[higher-order-thinking]] [[workflow-testing]]",
    "thoughts":"Adjust prompts on drift [[metacognition]]",
    "status":"completed",
    "conclusion":"Workflow complete: sequential insights integrated into strategy [[higher-order-thinking]] [[workflow-testing]]"
  }'
```

## Common errors & resolutions (tested)
- `ERROR: Must include [[concepts]]. Example: 'Analyzing [[Machine Learning]] algorithms'` → add at least one [[concept]] in `response` or `thoughts`.
- `ERROR: Conclusion required when completing session...` → provide `conclusion` when `status='completed'`.
- `ERROR: Conclusion must include [[concepts]] for knowledge graph integration.` → add [[concepts]] inside `conclusion`.
- `ERROR: branchId required when branchFromThought is specified for multi-agent coordination` → supply branchId whenever branchFromThought is set.
- `ERROR: Session session-{id} not found. To start new session, don't provide sessionId.` when calling SequentialThinking with `thoughtNumber=1` and no existing session → start SequentialThinking with `thoughtNumber=0`.
- `ERROR: Session {id} exists. Use different sessionId or continue existing.` when continuing SequentialThinking with `thoughtNumber=1` and no `branchId` → include a `branchId` (e.g., `"main"`) or move to `thoughtNumber=2+`.
- `ERROR: Parent workflow can only be set on first thought.` → parentWorkflowId only allowed with `thoughtNumber=1`; starting a new SequentialThinking session at `thoughtNumber=1` triggers “Session ... not found,” so linking a brand-new sequential session to the workflow currently fails. No workaround without code change; avoid parentWorkflowId for new ST sessions.

## Step-by-step usage (explicit, tested)
1) Start Workflow: call `Workflow` with `workflowId` + `response` containing [[concepts]] (optional `thoughts]); record returned `workflow-<ts>`.
2) For steps requiring SequentialThinking: run a multi-thought ST session.
   - Start: `thoughtNumber=0`, `nextThoughtNeeded=true`, include [[concepts]] (starting at 1 errors: “Session ... not found”).
   - Continue: `thoughtNumber=1` **must** include `branchId` (e.g., `"main"`), otherwise: “Session ... exists. Use different sessionId or continue existing.”
   - Complete ST: set `nextThoughtNeeded=false` and provide `conclusion` with [[concepts]]. Missing conclusion → completion error; conclusion without [[concepts]] → concept error.
3) Advance Workflow step: call `Workflow` with same `sessionId`, include [[concepts]] in `response`, note the ST session you just ran.
4) Finish Workflow: final `Workflow` call sets `status="completed"` and includes a `conclusion` with [[concepts]] (missing → completion error).
5) Parent linkage limitation: `parentWorkflowId` is only allowed when `thoughtNumber=1`, but new ST sessions cannot start at `thoughtNumber=1` (they error). Do not try to parent-link a brand-new ST session; proceed without parentWorkflowId unless the session already exists.
