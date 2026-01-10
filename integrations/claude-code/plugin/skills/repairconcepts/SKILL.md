---
name: ma:repairconcepts
description: Repairs graph corruption by replacing concept variants with canonical form in source markdown files
---

# RepairConcepts

Repairs graph corruption by replacing `[[concept]]` variants with canonical form in source markdown files. **DANGER: Permanently modifies files.**

## Parameters

- `conceptsToReplace` (string, required): Comma/semicolon-separated variants. Example: `"tools,Tools,TOOLS"`
- `canonicalConcept` (string, required): Canonical form. Empty string removes WikiLinks. Example: `"tool"`
- `folder` (string, optional): Subfolder under memory root. Default: all memory
- `dryRun` (bool, optional): Preview changes without applying. **Always start with true.** Default: `true`
- `createWikiLinks` (bool, optional): Convert plain text to `[[WikiLinks]]` instead of replacing existing. Default: `false`
- `minSemanticSimilarity` (double, optional): Similarity threshold (0.0-1.0) for semantic validation. Default: `0.7`. Set `0.0` to skip.

## Returns

```
=== SEMANTIC VALIDATION ===
All variants passed semantic validation.

Scanning 245 markdown files...
Would modify: 15 files
Total replacements: 101

This was a DRY RUN. To apply changes, run with dryRun=false
```

## Usage Patterns

### Standard Workflow (Always Start Here)

1. **Analyze**: Run `AnalyzeConceptCorruption` to identify variants
2. **Dry run**: Run with `dryRun=true`
3. **Review**: Check semantic validation and proposed changes
4. **Apply**: Run with `dryRun=false`
5. **Sync**: Run `Sync` to rebuild graph

### Examples

```json
// Safe: Plural consolidation
{"conceptsToReplace": "tools,Tools,TOOLS", "canonicalConcept": "tool", "dryRun": true}

// Safe: Case normalization
{"conceptsToReplace": "mcp,Mcp,MCP", "canonicalConcept": "MCP", "dryRun": true}

// Create WikiLinks from plain text
{"conceptsToReplace": "machine learning", "canonicalConcept": "Machine Learning", "createWikiLinks": true, "dryRun": true}

// Scoped to folder
{"conceptsToReplace": "agents,Agents", "canonicalConcept": "agent", "folder": "research/ai", "dryRun": true}

// Override validation (not recommended)
{"conceptsToReplace": "test-coverage,coverage", "canonicalConcept": "coverage", "minSemanticSimilarity": 0.0, "dryRun": true}
```

## Semantic Validation

Generates embeddings for canonical + variants, calculates cosine similarity. Blocks if any variant < threshold.

Ranges: **0.9-1.0** (safe), **0.7-0.9** (review), **0.5-0.7** (caution), **< 0.5** (blocked)

Example failure: `coding-agent: 0.623` → blocked (< 0.7 threshold)

## Safe vs Dangerous Patterns

### SAFE (Always Allowed)
- Plurals: `[[tools]]` → `[[tool]]`
- Case: `[[mcp]], [[Mcp]]` → `[[MCP]]`
- Typos: `[[knowlege]]` → `[[knowledge]]`

### DANGEROUS (Never Do This)
- Class names → generic: `[[VectorTools]]` → `[[tool]]` (destroys meaning)
- Specific → generic: `[[coding-agent]]` → `[[agent]]` (loses specificity)
- File paths → concepts: `[[GraphTools.cs]]` → `[[tool]]` (file ≠ concept)
- Compounds → base: `[[knowledge-graph]]` → `[[graph]]` (loses qualifier)

## WikiLink Removal (Dangerous)

Empty `canonicalConcept` removes `[[brackets]]`, breaking graph connections:

```json
{"conceptsToReplace": "deprecated-concept", "canonicalConcept": "", "dryRun": true}
```

## Integration

- **AnalyzeConceptCorruption**: ALWAYS run first to understand variants
- **Sync**: ALWAYS run after repairs to rebuild graph
- **SearchMemories**: Find files using concepts before consolidating
- **BuildContext**: Explore relationships to understand impact
- **Visualize**: Verify repairs in concept graph

## Constraints

- **Permanent modification**: Changes source files, not just database
- **Semantic validation**: Default threshold 0.7 prevents unsafe merges
- **Always dry run first**: Default is `dryRun=true` for safety
- **Sync required**: Graph won't update until Sync runs
