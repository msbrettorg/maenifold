# Bootstrap Guide

How to build domain expertise with maenifold: from empty graph to institutional memory.

---

## Overview

maenifold isn't just storage—it's infrastructure for building domain expertise over time. This guide walks through the full journey:

1. **Seed** — Initial knowledge with `[[WikiLinks]]`
2. **Research** — Workflows that explore and expand the graph
3. **Specialize** — Custom roles for your domain
4. **Systematize** — Custom workflows for your operations
5. **Operate** — Daily usage that compounds knowledge
6. **Maintain** — Sleep cycles that consolidate and prune

**Important**: Workflows require an LLM to drive them. The CLI provides primitives; the LLM provides intelligence.

| Phase | CLI-Only | LLM Required |
|-------|:--------:|:------------:|
| Seed domain | ✅ | |
| Research workflows | | ✅ |
| Create custom roles | | ✅ |
| Create custom workflows | | ✅ |
| Query/search/context | ✅ | |
| Run workflows | | ✅ |
| Sleep cycle | | ✅ |

---

## Phase 1: Seed Your Domain

Start by writing what you already know. Every `[[WikiLink]]` becomes a graph node.

```bash
# Write initial knowledge
maenifold --tool WriteMemory --payload '{
  "title": "Infrastructure Overview",
  "content": "Our [[microservices]] run on [[kubernetes]] with [[istio]] for [[service-mesh]]. We use [[golang]] for backend services and [[postgresql]] for persistence.",
  "folder": "architecture"
}'

# Sync to build the graph
maenifold --tool Sync --payload '{}'

# Verify the graph is growing
maenifold --tool BuildContext --payload '{"conceptName":"kubernetes","depth":1}'
```

**Tips**:
- Use `[[WikiLinks]]` liberally—they're free and build structure
- Organize with folders: `architecture/`, `decisions/`, `incidents/`, `runbooks/`
- Link concepts that relate: `[[kubernetes]]` + `[[deployment]]` + `[[rollback]]`

---

## Phase 2: Research & Expand

Use research workflows to systematically explore your domain. This requires an AI assistant with MCP access.

### Option A: Deep Single-Agent Research

The `agentic-research` workflow implements HyDE, reflexion, and information gain checks:

```
Run the agentic-research workflow to investigate [[kubernetes]] [[deployment-patterns]] for [[zero-downtime]] releases
```

The workflow will:
1. Establish knowledge baseline (search existing graph)
2. Generate hypothetical documents (HyDE)
3. Research external sources
4. Synthesize findings with `[[WikiLinks]]`
5. Check information gain; loop if insufficient

### Option B: Multi-Agent Collaborative Research

The `think-tank` workflow orchestrates multiple agents in waves:

```
Run the think-tank workflow to research [[service-mesh]] [[security-patterns]]
```

Waves:
1. **Charter** — Decompose the problem
2. **Wave 1** — Parallel domain scoping
3. **Wave 2** — Deep dives (nested `agentic-research`)
4. **Wave 3** — Cross-domain synthesis
5. **Wave 4** — Peer review and validation

### Option C: Manual Sequential Thinking

For more control, use `sequential-thinking` directly:

```bash
# Start a thinking session
maenifold --tool SequentialThinking --payload '{
  "response": "Investigating [[kubernetes]] [[network-policies]] for [[multi-tenant]] isolation. First, I need to understand the threat model...",
  "thoughtNumber": 0,
  "totalThoughts": 5,
  "nextThoughtNeeded": true
}'
```

Continue the session by passing the returned `sessionId` with subsequent thoughts.

---

## Phase 3: Create Domain Roles

Once you have foundational knowledge, create specialist roles using constitutional AI.

### Using constitutional-role-architecture

```
Run the constitutional-role-architecture workflow to create a [[kubernetes-architect]] role specializing in [[cloud-native]] [[production]] systems
```

The workflow guides you through:
1. **Define expertise** — What does this role know?
2. **Define principles** — What does this role prioritize?
3. **Define constraints** — What does this role avoid?
4. **Generate examples** — How does this role respond?

Output: A JSON role file at `assets/roles/kubernetes-architect.json`

### Role Structure

```json
{
  "id": "kubernetes-architect",
  "name": "Kubernetes Architect",
  "description": "Cloud-native systems specialist",
  "personality": {
    "expertise": ["kubernetes", "service-mesh", "gitops"],
    "principles": ["reliability over features", "observable by default"],
    "constraints": ["never skip health checks", "always consider blast radius"]
  }
}
```

### Adopting Roles

```bash
# CLI
maenifold --tool Adopt --payload '{"type":"role","identifier":"kubernetes-architect"}'

# Or in AI assistant
Adopt the kubernetes-architect role
```

The role conditions how the LLM reasons over your graph—same data, different perspective.

---

## Phase 4: Create Domain Workflows

With domain knowledge and a specialist role, create workflows tailored to your operations.

### Using higher-order-thinking

First adopt your domain role, then design workflows:

```
Adopt kubernetes-architect role

Run the higher-order-thinking workflow to design a [[deployment-review]] workflow for [[production-readiness]] validation
```

The workflow guides meta-cognitive design:
1. **Analyze the task** — What does this workflow need to accomplish?
2. **Identify steps** — What's the logical sequence?
3. **Define quality gates** — How do we know each step succeeded?
4. **Embed reasoning** — Which steps need `sequential-thinking`?
5. **Generate JSON** — Output the workflow definition

Output: A JSON workflow file at `assets/workflows/deployment-review.json`

### Workflow Structure

```json
{
  "id": "deployment-review",
  "name": "Deployment Review",
  "description": "Production readiness validation for Kubernetes deployments",
  "steps": [
    {
      "id": "resource-check",
      "name": "Resource Validation",
      "description": "Verify resource requests/limits are set appropriately",
      "requiresEnhancedThinking": true,
      "toolHints": ["BuildContext", "SearchMemories"]
    },
    {
      "id": "security-check",
      "name": "Security Posture",
      "description": "Validate security contexts and network policies",
      "requiresEnhancedThinking": true
    }
  ]
}
```

---

## Phase 5: Daily Operations

Now you have:
- **Knowledge graph** — Growing corpus of domain knowledge
- **Custom role** — Specialist persona for your domain
- **Custom workflows** — Systematic methodologies for your tasks

### Pattern: Context-First Operations

Before any task, build context:

```bash
# What do we know about this concept?
maenifold --tool BuildContext --payload '{"conceptName":"auth-service","depth":2}'

# What's related?
maenifold --tool FindSimilarConcepts --payload '{"conceptName":"auth-service","maxResults":10}'

# Search for specific patterns
maenifold --tool SearchMemories --payload '{"query":"auth-service incident","mode":"Hybrid"}'
```

### Pattern: Workflow-Driven Tasks

For systematic work, run your custom workflows:

```
Adopt kubernetes-architect role
Run deployment-review workflow for the auth-service release
```

The workflow:
1. Builds context from graph via `[[WikiLinks]]`
2. Applies domain role perspective
3. Steps through systematic checks
4. Embeds `sequential-thinking` for complex reasoning
5. Writes findings back to graph

### Pattern: Capture As You Go

Every insight should become a memory:

```bash
maenifold --tool WriteMemory --payload '{
  "title": "Auth Service Rollback Procedure",
  "content": "When [[auth-service]] fails [[health-checks]] after [[deployment]], execute [[rollback]] via...",
  "folder": "runbooks"
}'
```

The graph grows with every operation. Future sessions benefit from past work.

---

## Phase 6: Maintenance

Knowledge isn't static. Memories decay without use. Sleep cycles consolidate and prune.

### Sleep Cycle

Run periodically (daily recommended):

```
Run the memory-cycle workflow
```

Four specialists run in parallel:
1. **Consolidation** — Replays high-significance episodic memories, promotes to semantic
2. **Decay** — Analyzes access patterns, flags severely decayed content
3. **Repair** — Normalizes WikiLink variants, cleans orphaned concepts
4. **Epistemic** — Reviews assumptions, validates or invalidates based on evidence

### What Happens

| Memory Type | Grace Period | Half-Life | Fate |
|-------------|--------------|-----------|------|
| Episodic (`thinking/`) | 7 days | 14 days | Consolidated or decayed |
| Semantic (`research/`, `decisions/`) | 14 days | 30 days | Stable if accessed |
| Immortal (validated assumptions) | ∞ | ∞ | Permanent |

### Dream Synthesis

During consolidation, `FindSimilarConcepts` discovers novel connections across domains:

```
[[deployment-patterns]] ↔ [[incident-response]] (0.78 similarity)
Insight: Rollback procedures should be deployment patterns, not incident responses
```

These insights surface during sleep—connections you didn't explicitly make.

---

## The Compound Effect

```
Week 1:   Empty graph → Initial seeding
Month 1:  Research workflows expand graph → Custom roles created
Month 3:  Custom workflows systematize operations → Sleep cycles consolidate
Year 1:   Institutional memory that would take a team years to document
```

Every `[[WikiLink]]` compounds. Every workflow run teaches. Every sleep cycle refines.

The graph becomes your second brain for the domain—one that remembers, forgets appropriately, and surfaces what matters.

---

## Quick Reference

### CLI Primitives

```bash
# Memory
maenifold --tool WriteMemory --payload '{"title":"...","content":"...","folder":"..."}'
maenifold --tool SearchMemories --payload '{"query":"...","mode":"Hybrid"}'
maenifold --tool ReadMemory --payload '{"identifier":"memory://..."}'

# Graph
maenifold --tool Sync --payload '{}'
maenifold --tool BuildContext --payload '{"conceptName":"...","depth":2}'
maenifold --tool FindSimilarConcepts --payload '{"conceptName":"...","maxResults":10}'

# Reasoning
maenifold --tool SequentialThinking --payload '{"response":"...with [[WikiLinks]]...","thoughtNumber":0,"totalThoughts":5,"nextThoughtNeeded":true}'
maenifold --tool Adopt --payload '{"type":"role","identifier":"..."}'
maenifold --tool Workflow --payload '{"workflowId":"..."}'
```

### Key Workflows

| Workflow | Purpose |
|----------|---------|
| `agentic-research` | Deep single-agent research with HyDE and reflexion |
| `think-tank` | Multi-agent collaborative research in waves |
| `constitutional-role-architecture` | Create domain-specific roles |
| `higher-order-thinking` | Design custom workflows |
| `workflow-dispatch` | Auto-select optimal methodology |
| `memory-cycle` | Consolidation, decay, repair, epistemic maintenance |

### MCP Configuration

```json
{
  "mcpServers": {
    "maenifold": { "command": "maenifold", "args": ["--mcp"], "type": "stdio" }
  }
}
```

---

*The graph grows smarter over time. So does your AI assistant.*
