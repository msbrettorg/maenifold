# AssumptionLedger

Declare, update, and track assumptions without auto-inference. Stores as markdown in `memory://assumptions/` with `[[WikiLink]]` integration (e.g., [[hypothesis]], [[validation-plan]]).

## Parameters

### action (required)
One of: `"append"`, `"update"`, `"read"`

### For action="append"

**Required:**
- `assumption` (string): Assumption statement
- `concepts` (string[]): Concept tags for graph integration. Example: `["workflow", "sequential-thinking"]`

**Optional:**
- `context` (string): Session reference. Example: `"workflow://thinking/session-1756610546730"`
- `validationPlan` (string): Validation approach
- `confidence` (string): Free text level. Example: `"high"`, `"medium"`, `"low"`, `"needs-verification"`

### For action="update"

**Required:**
- `uri` (string): Memory URI. Example: `"memory://assumptions/2025/09/assumption-1759186965105"`

**Optional:**
- `status` (string): One of: `"active"`, `"validated"`, `"invalidated"`, `"refined"`
- `confidence` (string): Updated confidence level
- `validationPlan` (string): Updated validation plan
- `notes` (string): Timestamped notes to append

### For action="read"

**Required:**
- `uri` (string): Memory URI

## Returns

### Append
```json
{
  "uri": "memory://assumptions/2025/09/assumption-1759186965105",
  "status": "active",
  "confidence": "medium",
  "nextStep": "Run Sync() for search integration"
}
```

### Update
```json
{
  "uri": "memory://assumptions/2025/09/assumption-1759186965105",
  "status": "validated",
  "nextStep": "Run Sync() to update graph"
}
```

### Read
Returns full frontmatter (status, confidence, timestamps, context, validation plan) and markdown content.

## Example

```json
{
  "action": "append",
  "assumption": "Current implementation uses recursive traversal",
  "concepts": ["graph-traversal", "performance"],
  "context": "session-1759186950",
  "validationPlan": "Profile actual query performance",
  "confidence": "medium"
}
```

## Integration Patterns

### With SequentialThinking

```markdown
Before session:
AssumptionLedger(append) → log dominant assumptions

During session:
Reference assumption URIs in thoughts with [[WikiLinks]] (e.g., [[assumption-validation]], [[risk-assessment]])

After session:
AssumptionLedger(update) → mark validated/invalidated
```

### With SearchMemories

```markdown
# First sync to graph
Sync()

# Then search assumptions
SearchMemories(query: "workflow orchestration", folder: "assumptions")
```

## File Structure

Stored at: `memory://assumptions/YYYY/MM/assumption-{timestamp}.md`

```markdown
---
created: 2025-09-29T23:02:45Z
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

## Constraints

- **[[WikiLink]] required**: Must include concept tags for graph integration (e.g., [[assumption]], [[hypothesis]], [[risk]])
- **No auto-inference**: Tool stores declarations only - validation is agent's responsibility
- **Sync required**: Run `Sync()` after append/update for search integration
