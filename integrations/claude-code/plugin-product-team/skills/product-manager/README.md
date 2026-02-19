# Product Manager Skill

A multi-agent orchestration system that enables an AI to assume the Product Manager role and coordinate specialized ephemeral subagents (SWE, red-team, blue-team, researcher) to execute software development workflows with full traceability and quality gates.

## Overview

The Product Manager skill transforms Claude Code into a software development team orchestrator. It provides:

- **Multi-agent coordination**: Spawn and manage specialized subagents for development tasks
- **Automatic context injection**: Hook system enriches subagent prompts with graph knowledge
- **Quality gates**: ConfessionReport enforcement ensures accountability and transparency
- **Full traceability**: Every commit traces through PRD → RTM → TODO → code
- **Knowledge graph integration**: All work contributes to institutional memory
- **Wave-based parallelization**: Execute up to 8 concurrent tasks for maximum throughput

## Prerequisites

**REQUIRED**: The maenifold skill must be installed first. The Product Manager skill depends on maenifold's knowledge graph, memory system, and reasoning tools.

Install maenifold skill:
```bash
claude plugin install msbrettorg/maenifold
```

See the [plugin-maenifold README](../../../../plugin-maenifold/README.md) for complete installation instructions.

## Architecture

### System Diagram

```mermaid
graph TB
    User[User] -->|invokes| PM[Product Manager<br/>Skill]
    PM -->|reads| Docs[PRD.md<br/>RTM.md<br/>TODO.md<br/>RETROSPECTIVES.md]
    PM -->|spawns via Task| Pool[Agent Pool<br/>8 concurrent slots]

    subgraph "Subagent Pool"
        SWE[SWE Instance 1]
        SWE2[SWE Instance 2]
        Red[Red-team]
        Blue[Blue-team]
        Research[Researcher]
        More[... 3 more slots]
    end

    Pool --> Hooks[Hook System]
    Hooks -->|PreToolUse| Context[Concept-as-Protocol<br/>Graph Context Injection]
    Hooks -->|SubagentStop| Gate[ConfessionReport<br/>Quality Gate]

    PM -->|orchestrates| Workflows[Workflow Engine]
    Workflows -->|embeds| SeqThink[Sequential Thinking<br/>Shared Sessions]

    Pool -->|contributes to| SeqThink
    SeqThink -->|persists| Memory[memory:// corpus]
    Memory -->|feeds| Graph[Knowledge Graph]

    Graph -->|provides context via| Hooks

    style PM fill:#0969DA
    style Pool fill:#0969DA
    style SWE fill:#0969DA
    style Red fill:#0969DA
    style Blue fill:#0969DA
    style Research fill:#0969DA
    style More fill:#0969DA
    style Hooks fill:#0969DA
    style Context fill:#0969DA
    style Gate fill:#0969DA
    style Workflows fill:#0969DA
    style SeqThink fill:#0969DA
    style Memory fill:#0969DA
    style Graph fill:#0969DA
```

### Hook Integration

The hook system provides automatic enhancements to subagent interactions:

```mermaid
sequenceDiagram
    participant PM as Product Manager
    participant Hook as Hook System
    participant Graph as Knowledge Graph
    participant Agent as Subagent

    PM->>Hook: Task("Fix [[authentication]] bug", agent="swe")
    Hook->>Hook: Extract [[authentication]] from prompt
    Hook->>Graph: BuildContext(authentication, depth=1)
    Graph-->>Hook: Related concepts + memory:// files
    Hook->>Agent: Augmented prompt with context
    Agent->>Agent: Execute with enriched knowledge
    Agent->>Hook: Attempt to stop
    Hook->>Hook: Search for "ConfessionReport"
    alt ConfessionReport found
        Hook->>PM: Allow agent stop
    else ConfessionReport missing
        Hook->>Agent: Block with reason
        Agent->>Agent: Write ConfessionReport
        Agent->>Hook: Retry stop
        Hook->>PM: Allow agent stop
    end
```

## How It Works

### Agent Pool Architecture

The system provides **8 concurrent task slots** that can execute specialized subagents in parallel:

- **SWE**: Software engineer for implementation tasks
- **Red-team**: Security and adversarial testing
- **Blue-team**: Test creation and verification
- **Researcher**: Analysis and investigation

**Key insight**: Think "8 SWE instances" not "1 SWE". Each agent type can be instantiated multiple times for parallel execution.

### Hook Mechanisms

#### 1. Concept-as-Protocol (PreToolUse Hook)

When PM spawns a subagent with `[[WikiLinks]]` in the Task prompt, the PreToolUse hook:

1. Extracts all `[[WikiLink]]` concepts from the prompt
2. Enriches the task prompt with graph context from the community index
3. Injects context into the subagent's prompt before they see it

**Result**: Subagents receive automatic knowledge enrichment without manual searches.

#### 2. ConfessionReport Gate (SubagentStop Hook)

Every subagent must produce a ConfessionReport before termination. The SubagentStop hook:

1. Intercepts agent termination attempts
2. Searches transcript for "ConfessionReport" string
3. **Blocks** termination if not found, with explanation
4. **Allows** termination if present

**ConfessionReport Structure**:
1. All instructions/constraints/objectives followed
2. Compliance assessment (✅/❌) with evidence
3. Gaps and transparency about limitations
4. Uncertainties, ambiguities, grey areas
5. Shortcuts, hacks, policy risks
6. All files, memory:// URIs, [[WikiLinks]] used

**Purpose**: Ensures accountability, prevents silent failures, enables PM to verify quality.

#### 3. Session Start Context (SessionStart Hook)

On session initialization, the hook:

- Calls `RecentActivity` (limit=10, timespan=3d) to identify seed concepts
- Validates each discovered `[[WikiLink]]` against the graph
- Calls `BuildContext` (depth=1) on each valid concept to expand the neighborhood via the Louvain community index
- Formats output capped at ~150–350 tokens, with bold seed labels, up to 5 thread references, and an action footer
- Falls back to `SearchMemories` if `RecentActivity` returns no results
### Workflow Patterns

#### Wave-Based Orchestration

Complex work is decomposed into **waves** where multiple agents execute in parallel:

```mermaid
graph LR
    Start[Sprint Start] --> Discovery[Discovery Wave<br/>3 agents parallel]
    Discovery --> Spec[Specs Wave]
    Spec --> RTM[RTM Creation]
    RTM --> UserConfirm[User Confirmation]
    UserConfirm --> Validation[Validation Wave]
    Validation --> ImplPlan[Implementation Planning]
    ImplPlan --> ImplDispatch[Implementation Dispatch<br/>8 agents parallel]
    ImplDispatch --> RedTeam[Red Team Wave<br/>3 agents parallel]
    RedTeam --> Remediation[Remediation Wave]
    Remediation --> ImplVerify[Implementation Verification]
    ImplVerify --> TestWave[Test Wave<br/>3 agents parallel]
    TestWave --> RTMVerify[RTM Verification]
    RTMVerify --> Cleanup[Cleanup Wave]
    Cleanup --> Review[Sprint Review]

    style Start fill:#0969DA
    style Discovery fill:#0969DA
    style Spec fill:#0969DA
    style RTM fill:#0969DA
    style UserConfirm fill:#0969DA
    style Validation fill:#0969DA
    style ImplPlan fill:#0969DA
    style ImplDispatch fill:#0969DA
    style RedTeam fill:#0969DA
    style Remediation fill:#0969DA
    style ImplVerify fill:#0969DA
    style TestWave fill:#0969DA
    style RTMVerify fill:#0969DA
    style Cleanup fill:#0969DA
    style Review fill:#0969DA
```

**Key Principle**: Dispatch all agents in **one message** per wave for concurrent execution.

Example:
```
# ✅ Parallel Dispatch (one message)
Task("Analyze RTM-001", agent="swe-1", session="sprint-discovery")
Task("Analyze RTM-002", agent="swe-2", session="sprint-discovery")
Task("Analyze RTM-003", agent="swe-3", session="sprint-discovery")

# ❌ Serial Dispatch (anti-pattern)
Task("Analyze RTM-001", agent="swe")
# wait...
Task("Analyze RTM-002", agent="swe")
# wait...
```

#### Traceability Chain

Every artifact maintains full traceability through the requirements hierarchy:

```mermaid
graph TD
    PRD[PRD.md<br/>Functional Requirements<br/>FR-X.X] --> RTM[RTM.md<br/>Test Cases<br/>FR-X.X → TC-X.X.X]
    RTM --> TODO[TODO.md<br/>Tasks<br/>T-X.X.X → FR-X.X]
    TODO --> Code[Code<br/>// T-X.X.X: RTM FR-X.X]
    Code --> Commit[Git Commit<br/>T-X.X.X in message]
    Commit --> Confession[ConfessionReport<br/>References T-X.X.X]

    style PRD fill:#0969DA
    style RTM fill:#0969DA
    style TODO fill:#0969DA
    style Code fill:#0969DA
    style Commit fill:#0969DA
    style Confession fill:#0969DA
```

**Rule**: Work without traceability is rejected.

### Sequential Thinking Collaboration

All agents in a wave can share the same sequential thinking session:

```mermaid
graph TD
    PM[PM creates session:<br/>sprint-20260129-discovery] --> Agent1[SWE-1 contributes<br/>thoughts + concepts]
    PM --> Agent2[SWE-2 contributes<br/>thoughts + concepts]
    PM --> Agent3[SWE-3 contributes<br/>thoughts + concepts]

    Agent1 --> Session[Consolidated Session<br/>All thoughts + concepts]
    Agent2 --> Session
    Agent3 --> Session

    Session --> Memory[memory://workflows/thinking/<br/>sprint-20260129-discovery.md]
    Memory --> Graph[Knowledge Graph<br/>Concepts become nodes]

    Graph --> Future[Future Sessions<br/>Benefit from richer graph]

    style PM fill:#0969DA
    style Agent1 fill:#0969DA
    style Agent2 fill:#0969DA
    style Agent3 fill:#0969DA
    style Session fill:#0969DA
    style Memory fill:#0969DA
    style Graph fill:#0969DA
    style Future fill:#0969DA
```

**Benefits**:
- Persistent thought process across ephemeral agents
- Cross-agent visibility and collaboration
- Institutional memory that compounds over time
- Graph building through `[[WikiLinks]]` in thoughts

## Component Overview

### PM Skill (`SKILL.md`)

Entry point that:
- Defines Product Manager responsibilities
- Embeds sprint lifecycle orchestration patterns
- Provides access to maenifold's cognitive stack
- Enforces traceability and quality principles

### Agent Definitions (`/integrations/claude-code/plugin-product-team/agents/*.md`)

Each agent has:
- **YAML frontmatter**: Color, skills, description
- **Markdown body**: Role-specific instructions and behavioral guidelines
- **Specialization**: SWE (implementation), red-team (security), blue-team (testing), researcher (analysis)

### Hook System

All hooks live in the **plugin-maenifold** plugin (`/integrations/claude-code/plugin-maenifold/`):

- **`hooks/hooks.json`**: Hook configuration (SessionStart, PreToolUse, SubagentStop)
- **`scripts/hooks.sh`**: Bash implementation with timeout handling, CLI discovery, concept extraction

### Workflow Assets (`src/assets/workflows/*.json`)

Structured JSON workflows that define:
- Step-by-step processes
- Tool hints and metadata
- Stop conditions and guardrails
- Examples: `agentic-slc`, `workflow-dispatch`

## Execution Flow Example

### Sprint Flow with Wave-Based Orchestration

```mermaid
sequenceDiagram
    participant User
    participant PM as Product Manager
    participant Graph as Knowledge Graph
    participant Agents as Agent Pool
    participant Session as Sequential Thinking

    User->>PM: Start sprint for RTM-001, RTM-002, RTM-003
    PM->>PM: Read PRD.md, RTM.md, TODO.md
    PM->>PM: Create sprint-20260129-auth branch

    Note over PM,Agents: Discovery Wave (Parallel)
    PM->>Agents: Task("Analyze RTM-001", agent="swe-1", session="sprint-discovery")
    PM->>Agents: Task("Analyze RTM-002", agent="swe-2", session="sprint-discovery")
    PM->>Agents: Task("Analyze RTM-003", agent="swe-3", session="sprint-discovery")

    Agents->>Graph: PreToolUse Hook enriches prompts
    Agents->>Session: All contribute to same session
    Agents-->>PM: ConfessionReports with findings

    PM->>Session: Read consolidated discovery

    Note over PM,Agents: Implementation Wave (Parallel)
    PM->>Agents: Task("Implement RTM-001", agent="swe-1")
    PM->>Agents: Task("Implement RTM-002", agent="swe-2")
    PM->>Agents: Task("Implement RTM-003", agent="swe-3")

    Agents-->>PM: ConfessionReports with commits

    Note over PM,Agents: Red Team Wave (Parallel)
    PM->>Agents: Task("Red team RTM-001", agent="red-team-1")
    PM->>Agents: Task("Red team RTM-002", agent="red-team-2")
    PM->>Agents: Task("Red team RTM-003", agent="red-team-3")

    Agents-->>PM: ConfessionReports with issues

    Note over PM,Agents: Remediation Wave (Parallel)
    PM->>Agents: Fix all issues found

    Note over PM,Agents: Test Wave (Parallel)
    PM->>Agents: Task("Test RTM-001", agent="blue-team-1")
    PM->>Agents: Task("Test RTM-002", agent="blue-team-2")
    PM->>Agents: Task("Test RTM-003", agent="blue-team-3")

    Agents-->>PM: ConfessionReports with coverage

    PM->>PM: Verify all ConfessionReports
    PM->>PM: Verify RTM traceability
    PM->>User: Sprint complete
```

## Key Principles

### 1. Traceability Everywhere

```
Code → Commit (T-*) → TODO.md (T-* → FR-*) → RTM.md (FR-* → TC-*) → PRD.md (FR-*)
```

Work without traceability is rejected.

### 2. Quality Gates

ConfessionReport requirement ensures:
- Subagents can't silently fail
- Non-compliance is visible
- PM can re-assign work to new instances
- Accountability through transparency

### 3. Parallel by Default

PM always asks: "Could I give this to 8 agents instead of 1?"
- Decompose to file/component/test-suite level
- Dispatch all agents in ONE message per wave
- Maximize throughput, minimize idle time

### 4. SLC Philosophy

Simple, Lovable, Complete:
- No MVPs, scaffolds, stubs, mocks
- Ruthlessly prune non-value-adding work
- Real systems, real data, real artifacts
- Cargo-cult avoidance

### 5. Graph-First Knowledge

Never rely on internal model knowledge:
- Always search graph first (buildcontext, findsimilarconcepts, searchmemories)
- Use external docs when memory:// insufficient
- Write lineage-backed memory:// notes
- Include memory:// URIs in responses

## Installation

### Quick Install

```bash
# Install maenifold plugin (includes Product Manager skill)
claude plugin install msbrettorg/maenifold
```

### Verification

Start Claude Code and invoke the Product Manager skill:

```
Adopt the product manager role and review TODO.md
```

The skill should load automatically and the hook system should be active.

## Configuration

### Hook Configuration

Hooks are configured in `.mcp.json` via the plugin. The default configuration includes:

- **SessionStart**: Automatically load maenifold skill and inject repository context
- **PreToolUse**: Inject graph context when Task includes `[[WikiLinks]]`
- **SubagentStop**: Enforce ConfessionReport before agent termination

### Custom Hook Timeout

Edit `.mcp.json` to adjust hook timeout (default: 5 seconds):

```json
{
  "plugins": {
    "product-team": {
      "hookTimeout": 10000
    }
  }
}
```

## Usage Examples

### Example 1: Simple Bug Fix

```
User: Fix the authentication timeout bug

PM Response:
1. [Spawns researcher with [[authentication]], [[timeout]] concepts]
2. [PreToolUse hook injects graph context]
3. [Researcher produces findings + ConfessionReport]
4. [Spawns SWE with research findings + T-123 identifier]
5. [SWE implements fix, commits with T-123]
6. [Spawns blue-team to verify fix]
7. [Reviews all ConfessionReports, verifies traceability]
```

### Example 2: Multi-Item Sprint

```
User: Start sprint for RTM-001, RTM-002, RTM-003

PM Response:
1. [Creates sprint-20260129-auth branch]
2. Discovery Wave: 3 agents in parallel (all in one message)
3. Implementation Wave: 3 agents in parallel (all in one message)
4. Red Team Wave: 3 agents in parallel (all in one message)
5. Remediation Wave: Fix all issues
6. Test Wave: 3 agents in parallel (all in one message)
7. [Verifies all ConfessionReports and traceability]
```

### Example 3: TDD Workflow

```
User: Implement feature X using TDD workflow

PM Response:
1. Stage 1: SWE defines contract (optional)
2. Stage 2: Blue-team writes tests from RTM
3. Stage 3: SWE implements to pass tests
4. Stage 4: Red-team attacks implementation
5. Stage 5: Blue-team verifies coverage held

[All stages require compliant ConfessionReport]
```

## Troubleshooting

### Hook Latency

**Symptom**: Task invocations take 5-10 seconds
**Cause**: Bash hook queries graph for every Task
**Mitigation**: Hooks have 5-second timeout; failures degrade gracefully

### Agent Blocked by SubagentStop Hook

**Symptom**: Agent can't terminate, receives "ConfessionReport missing" error
**Cause**: Agent didn't write ConfessionReport
**Solution**: Agent must write ConfessionReport including all required sections

### Context Window Overflow

**Symptom**: Large context injections consume token budget
**Cause**: Multiple parallel agents with 8000-token contexts each
**Mitigation**: Use maenifold memory tools to save important context before compaction

### Graph Fragmentation

**Symptom**: Context injection returns weak or duplicate concepts
**Cause**: Poor concept tagging (e.g., [[Tool]], [[tool]], [[tools]], [[Tool.cs]])
**Solution**: Use concept repair tools: `analyzeconceptcorruption` → `repairconcepts` with `dryRun=true`

## Limitations

1. **Hook Performance**: 5-10s latency per Task (bash + CLI + graph queries)
2. **Context Budget**: 8000 tokens per Task can accumulate with parallel agents
3. **Pool Limits**: 8 concurrent tasks maximum (requires wave sequencing for larger sprints)
4. **ConfessionReport Enforcement**: Simple string matching (agent could game system)
5. **Graph Fragmentation Risk**: Poor concept tagging dilutes context relevance

## Related Documentation

- [Plugin-Maenifold README](../../../../plugin-maenifold/README.md) - Required prerequisite plugin
- [Architecture Analysis](memory://tech/maenifold/product-manager-skill-architecture-analysis) - Complete technical analysis
- [SKILL.md](./SKILL.md) - Skill definition and instructions
- [Agent Definitions](../../agents/) - SWE, red-team, blue-team, researcher (at `/integrations/claude-code/plugin-product-team/agents/`)
- [Hooks System](../../claude-code/plugin-maenifold/hooks/) - Hook implementation (in plugin-maenifold)

## Source

This documentation is based on analysis of the maenifold codebase:

- `/integrations/claude-code/plugin-product-team/skills/product-manager/SKILL.md`
- `/integrations/claude-code/plugin-product-team/agents/*.md`
- `/integrations/claude-code/plugin-maenifold/hooks/` (all hooks)
- `src/assets/workflows/*.json`

**Date**: 2026-01-29
**Repository**: https://github.com/msbrettorg/maenifold
