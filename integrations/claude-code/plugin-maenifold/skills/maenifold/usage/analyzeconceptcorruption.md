# AnalyzeConceptCorruption

⚠️ **MUST USE BEFORE RepairConcepts.** Read-only diagnostic revealing `[[WikiLink]]` families and variants for safe consolidation planning.

## Parameters

- `conceptFamily` (string, required): Base concept to analyze. Example: `"tool"`, `"agent"`, `"test"`
- `maxResults` (int, optional): Maximum variants to show. Default: 50

## Returns

```
=== Concept Family Analysis: 'tool' ===
Found 15 unique variants

Singular/Plural:
  • tool (245x)
  • tools (89x)

Compound (with -):
  • MCP-tool (34x)
  • tool-agent (23x)

File Paths:
  • VectorTools.cs (18x)
  • GraphTools.cs (15x)

=== SUGGESTED REPAIRS ===
Fix plural forms:
  RepairConcepts conceptsToReplace='tools,Tools' canonicalConcept='tool'
```

## Examples

### Basic Analysis
```json
{
  "conceptFamily": "tool"
}
```

### Large-Scale Audit
```json
{
  "conceptFamily": "agent",
  "maxResults": 100
}
```

## Output Interpretation

### ✅ Safe to Merge
- **Singular/Plural**: `[[tool]]` vs `[[tools]]` - Standardize on singular
- **Case Variations**: `[[MCP]]` vs `[[mcp]]` - Standardize casing
- **Whitespace**: `[[AI agent]]` vs `[[AI-agent]]` - Consolidate if semantically identical

### 🚨 DANGEROUS to Merge
- **File Paths**: `[[GraphTools.cs]]` - FILE REFERENCES, not concepts
- **Class Names**: `[[VectorTools]]` - CODE ENTITIES, merging with `[[tool]]` destroys meaning
- **Compound Forms**: `[[coding-agent]]` - SPECIFIC TYPES, not generic `[[agent]]`
- **With Suffix**: `[[tool-framework]]` - DISTINCT CONCEPTS with specialized meaning

## Pattern Categories

| Category | Description | Risk Level | Example |
|----------|-------------|------------|---------|
| Singular/Plural | Standard pluralization | Low | `[[tool]]` vs `[[tools]]` |
| Case Variations | Different casing | Low | `[[MCP]]` vs `[[mcp]]` |
| Compound (with -) | Hyphenated forms | High | `[[coding-agent]]` vs `[[agent]]` |
| File Paths | File/class references | Critical | `[[GraphTools.cs]]` vs `[[tool]]` |
| With Suffix | Extended concepts | Medium-High | `[[agent-framework]]` vs `[[agent]]` |

## Critical Warnings

**DO NOT blindly run suggested repairs.** Tool suggests repairs based on PATTERNS, not SEMANTIC MEANING.

**Safe Consolidation**:
- `[[MCP Tools]]` → `[[MCP]]` (case/plural standardization)
- `[[agents]]` → `[[agent]]` (plural consolidation)

**UNSAFE Consolidation**:
- `[[VectorTools]]` → `[[tool]]` (destroys class reference)
- `[[coding-agent]]` → `[[agent]]` (destroys type specificity)

## Workflow

1. Run AnalyzeConceptCorruption on target `conceptFamily`
2. Review output categorization
3. Identify safe consolidations (plurals, casing) vs dangerous (compounds, file paths)
4. Validate semantic equivalence with SearchMemories/BuildContext
5. Execute RepairConcepts with `dryRun: true` first
6. Apply repairs if safe, run Sync to rebuild graph

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| "Found 0 variants" | No matching concepts | Use base words (`"tool"`) not compounds (`"tool-system"`) |
| Too many results | Common concept | Increase `maxResults` or use specific conceptFamily |
| Missing variants | Different naming | Run SearchMemories to find actual usage |

## Integration

- **RepairConcepts**: ⚠️ ONLY after analysis - makes actual file changes
- **Sync**: Run after repairs to rebuild graph
- **BuildContext**: Explore concept relationships before consolidation
- **SearchMemories**: Find concept usage context
- **Visualize**: See concept relationships graphically
