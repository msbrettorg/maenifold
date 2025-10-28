---
layout: ../../../layouts/DocsLayout.astro
---

# Assumption Ledger

A minimal, manifesto-aligned tool for agents to declare, revisit, and resolve assumptions.
Provides traceable skepticism without auto-inference, scoring, or retry logic.
Integrates with SequentialThinking and Workflow sessions via context references.

## When to Use This Tool

- **Before SequentialThinking**: Log dominant assumptions alongside initial prompt to establish context
- **During Workflow Execution**: Reference ledger entries when decisions depend on unvalidated assumptions
- **After Completion**: Update status to show what held true, capturing lessons in memory
- **Multi-agent Coordination**: Share assumptions across agent boundaries for continuity
- **Risk Management**: Make implicit assumptions explicit and trackable

Key features:
- Captures assumption statements without inference or analysis
- Stores as durable Markdown artifacts in memory://assumptions/
- Integrates with knowledge graph through [[concept]] tags
- Searchable via SearchMemories after Sync()
- Supports status tracking (active, validated, invalidated, refined)
- Maintains complete audit trail with timestamps
- Zero auto-reasoning - agents decide what to validate and when

## What This Tool Does NOT Do

In alignment with Maenifold philosophy:
- ❌ Does NOT automatically discover or rank assumptions
- ❌ Does NOT cache, score, or batch operations beyond markdown writes
- ❌ Does NOT implement telemetry, analytics, or usage tracking
- ❌ Does NOT provide helper UI, wizards, or configuration generators
- ❌ Does NOT infer relationships or create automated ontologies

**Why:** Every decision we make removes a decision the agent could make. The ledger is a mirror, not a mind.

## Parameters

### action (required)
One of: "append", "update", or "read"

### For action="append" (Create New Assumption)

**Required:**
- `assumption`: The assumption statement (free text)
- `concepts`: Array of concept tags for knowledge graph integration (e.g., ["workflow", "sequential-thinking"])

**Optional:**
- `context`: Reference to workflow or sequential thinking session (e.g., "workflow://thinking/session-1756610546730")
- `validationPlan`: How you plan to validate this assumption (free text)
- `confidence`: Free text confidence level (e.g., "high", "medium", "low", "needs-verification")

### For action="update" (Modify Existing Assumption)

**Required:**
- `uri`: Memory URI of the assumption (e.g., "memory://assumptions/2025/09/assumption-1759186965105")

**Optional:**
- `status`: New status - one of: "active", "validated", "invalidated", or "refined"
- `confidence`: Updated confidence level (free text)
- `validationPlan`: Updated validation plan
- `notes`: Additional notes to append (creates timestamped section)

### For action="read" (View Existing Assumption)

**Required:**
- `uri`: Memory URI of the assumption

## Output Format

### Append
Returns confirmation with:
- Memory URI for the new assumption
- Statement, confidence, and status summary
- Declarative next-step suggestion (run Sync() for search integration)
- Validation plan reminder if provided

### Update
Returns confirmation with:
- Updated URI
- New status and/or confidence if changed
- Declarative next-step suggestion (run Sync() to update graph)

### Read
Returns full assumption details:
- All frontmatter metadata (status, confidence, timestamps, context, validation plan)
- Complete content including statement, validation plan, and concept links
- Update history if notes have been added

## Integration Patterns

### With SequentialThinking

```markdown
Before starting session:
AssumptionLedger(
  action: "append",
  assumption: "Current implementation uses recursive traversal",
  context: "session-1759186950",
  concepts: ["graph-traversal", "performance"],
  validationPlan: "Profile actual query performance"
)

Then in thoughts:
"Analyzing [[graph-traversal]] performance assuming [[recursive-implementation]]...
See memory://assumptions/2025/09/assumption-X for baseline assumption"
```

### With Workflow

```markdown
AssumptionLedger(
  action: "append",
  assumption: "Bug fix requires schema migration",
  context: "workflow://thinking/workflow-1759186950",
  concepts: ["database", "migration"]
)
```

### With SearchMemories

```markdown
# First, sync assumptions to knowledge graph
Sync()

# Then search across all assumptions
SearchMemories(
  query: "workflow orchestration",
  mode: "Hybrid",
  folder: "assumptions"
)
```

## File Structure

Assumptions stored at: `memory://assumptions/YYYY/MM/assumption-{timestamp}.md`

Each file contains:
1. **YAML Frontmatter**: status, confidence, context, validation_plan, timestamps
2. **Markdown Content**: Human-readable assumption statement and related concepts

Example:
```markdown
---
created: 2025-09-29T23:02:45.1054396Z
status: active
confidence: medium
context: workflow://thinking/session-1756610546730
validation_plan: Validate once dialogue MCP hooks are reintroduced
---

# Assumption: The dialogue tool will remain MCP-only

## Statement

The dialogue tool will remain MCP-only

## Validation Plan

Validate once dialogue MCP hooks are reintroduced

## Related Concepts

- [[dialogue]]
- [[workflow-dispatch]]
```

## Workflow Guidance

### Before SequentialThinking
Log dominant assumptions alongside the initial prompt to establish context.

### During Workflow Execution
Reference ledger entries in thoughts when a decision depends on them.

### After Completion
Update status to show what held true, capturing lessons in memory:// rather than proposing new features.

## Notes

- Requires [[concept]] tags for knowledge graph integration
- Files persist across sessions in date-organized folders
- Searchable after running Sync()
- Updates create timestamped sections maintaining complete history
- Zero automation beyond file write - validation is agent's responsibility

## See Also

- [SequentialThinking](sequentialthinking.md)
- [Workflow](workflow.md)
- [SearchMemories](searchmemories.md)
- [Sync](sync.md)
- [Assumption Ledger Full Guide](../../../docs/ASSUMPTION_LEDGER.md)
