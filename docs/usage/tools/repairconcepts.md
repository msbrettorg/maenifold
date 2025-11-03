# RepairConcepts

⚠️ **DANGER: This tool can PERMANENTLY DAMAGE your [[knowledge-graph]] if used incorrectly!** Repairs [[graph]] corruption by replacing [[concept]] variants with canonical form in source [[markdown]] files. Fixes the source of truth rather than just the database. Use this to consolidate [[concept]] families while preserving semantic meaning.

🛡️ **SEMANTIC VALIDATION**: Automatically blocks unsafe consolidations by checking semantic similarity between concepts using [[vector-embeddings]]. Only proceeds if ALL variants have similarity >= minSemanticSimilarity threshold (default 0.7).

## When to Use This Tool

- **After AnalyzeConceptCorruption**: ⚠️ ALWAYS run AnalyzeConceptCorruption FIRST to understand what you're changing
- **Plural Standardization**: Consolidate [[tools]] → [[tool]], [[tests]] → [[test]] (safe)
- **Case Normalization**: Standardize [[MCP]] vs [[mcp]] to consistent casing
- **Graph Cleanup**: Remove duplicate concepts that fragment knowledge connections
- **WikiLink Creation**: Convert plain text to [[WikiLinks]] for graph integration
- **WikiLink Removal**: Convert [[concepts]] back to plain text (use with extreme caution)
- **Concept Renaming**: Change concept names while updating all references

## Key Features

- **Semantic Similarity Validation**: Uses [[ONNX]] [[embeddings]] to verify concepts are truly equivalent before merging
- **Dry Run Mode**: Preview all changes before applying (ALWAYS start with dryRun=true)
- **Source File Modification**: Changes actual .md files, not just database - permanent and complete
- **WikiLink Creation Mode**: Convert plain text phrases to [[WikiLinks]] for graph integration
- **WikiLink Removal Mode**: Convert [[concepts]] to plain text (dangerous - breaks graph connections)
- **Batch Processing**: Process multiple variants in single operation
- **Folder Scoping**: Limit changes to specific memory subfolders
- **Change Preview**: Shows exactly what will be replaced before applying
- **Sync Integration**: Prompts to run Sync after repairs to rebuild graph

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| conceptsToReplace | string | Yes | Comma/semicolon-separated list of concept variants to replace | "tools,Tools,TOOLS" |
| canonicalConcept | string | Yes* | The canonical concept to use (*empty string for WikiLink removal) | "tool" |
| folder | string | No | Subfolder under memory root to process (default: all memory) | "research/ai" |
| dryRun | bool | No | **ALWAYS START WITH TRUE!** Preview changes without modifying files (default: true) | true |
| createWikiLinks | bool | No | Create [[WikiLinks]] from plain text instead of replacing existing (default: false) | false |
| minSemanticSimilarity | double | No | Minimum similarity threshold (0.0-1.0) for safe consolidation (default: 0.7) | 0.7 |

## Usage Examples

### Safe: Plural Consolidation (ALWAYS START HERE)
```json
{
  "conceptsToReplace": "tools,Tools,TOOLS",
  "canonicalConcept": "tool",
  "dryRun": true
}
```
⚠️ First run with dryRun=true to preview changes. This is SAFE - just plural standardization.

### After Dry Run: Apply the Changes
```json
{
  "conceptsToReplace": "tools,Tools,TOOLS",
  "canonicalConcept": "tool",
  "dryRun": false
}
```
Only run with dryRun=false AFTER reviewing the dry run output and confirming safety.

### Safe: Case Normalization
```json
{
  "conceptsToReplace": "mcp,Mcp,MCP",
  "canonicalConcept": "MCP",
  "dryRun": true
}
```
Standardizes casing - semantic validation will pass as these are identical concepts.

### Creating WikiLinks from Plain Text
```json
{
  "conceptsToReplace": "machine learning,neural network",
  "canonicalConcept": "Machine Learning",
  "createWikiLinks": true,
  "dryRun": true
}
```
Converts plain text "machine learning" → [[Machine Learning]] for graph integration.

### Scoped Repair (Specific Folder)
```json
{
  "conceptsToReplace": "agents,Agents",
  "canonicalConcept": "agent",
  "folder": "research/ai",
  "dryRun": true
}
```
Limits changes to research/ai subfolder - useful for targeted cleanup.

### Override Semantic Validation (NOT RECOMMENDED)
```json
{
  "conceptsToReplace": "test-coverage,coverage",
  "canonicalConcept": "coverage",
  "minSemanticSimilarity": 0.0,
  "dryRun": true
}
```
⚠️ Setting minSemanticSimilarity=0.0 disables safety checks - only use if you understand the risk.

## Critical Warnings

### ⚠️ NEVER CONSOLIDATE THESE PATTERNS

1. **Class Names → Generic Concepts**
   - ❌ WRONG: [[VectorTools]] → [[tool]] (destroys "this is the VectorTools CLASS" meaning)
   - ✅ RIGHT: Keep [[VectorTools]] as distinct from [[tool]]

2. **Specific Types → Generic Categories**
   - ❌ WRONG: [[coding-agent]] → [[agent]] (loses "this is a CODING agent" specificity)
   - ✅ RIGHT: Keep [[coding-agent]] as distinct concept

3. **File Paths → Concepts**
   - ❌ WRONG: [[GraphTools.cs]] → [[tool]] (file reference != concept)
   - ✅ RIGHT: Never consolidate file paths with concepts

4. **Compound Concepts → Base Terms**
   - ❌ WRONG: [[knowledge-graph]] → [[graph]] (loses "knowledge" qualifier)
   - ✅ RIGHT: Evaluate semantic similarity first

### ✅ SAFE CONSOLIDATION PATTERNS

1. **Plurals → Singular**
   - ✅ [[tools]] → [[tool]]
   - ✅ [[agents]] → [[agent]]
   - ✅ [[tests]] → [[test]]

2. **Case Variations → Standard Case**
   - ✅ [[mcp]], [[Mcp]] → [[MCP]]
   - ✅ [[WikiLinks]], [[wikilinks]] → [[WikiLink]]

3. **Typos → Correct Spelling**
   - ✅ [[knowlege]] → [[knowledge]]
   - ✅ [[embedings]] → [[embeddings]]

## Semantic Validation Explained

### How It Works
1. Generates [[vector-embeddings]] for canonical concept
2. Generates embeddings for each variant to replace
3. Calculates cosine similarity between canonical and each variant
4. Blocks operation if ANY variant falls below threshold

### Similarity Thresholds
- **0.9-1.0**: Nearly identical (plurals, typos, case variations)
- **0.7-0.9**: Related but distinct (may be safe depending on context)
- **0.5-0.7**: Somewhat related (⚠️ review carefully)
- **< 0.5**: Semantically different (🚨 DO NOT merge)

### Example Validation Output
```
=== SEMANTIC VALIDATION ===
Checking semantic similarity with threshold 0.70...

WARNING: Unsafe consolidation detected! The following concepts are semantically dissimilar:
  - coding-agent: similarity 0.623
  - deployment-agent: similarity 0.589

This would destroy semantic meaning. Review concepts manually or increase similarity threshold.
```

## Dangerous Operations

### WikiLink Removal (Breaks Graph Connections)
```json
{
  "conceptsToReplace": "deprecated-concept",
  "canonicalConcept": "",
  "dryRun": true
}
```
⚠️ Empty canonicalConcept removes [[brackets]] entirely, breaking graph connections. Only use for de-linking obsolete concepts.

### Bypassing Semantic Validation
```json
{
  "minSemanticSimilarity": 0.0
}
```
🚨 Disables safety checks - you can destroy semantic meaning. Only use if semantic validation is preventing a consolidation you've manually verified as safe.

## Common Patterns

### Standard Repair Workflow
1. **Analyze First**: Run AnalyzeConceptCorruption to understand variants
2. **Dry Run**: Run RepairConcepts with dryRun=true
3. **Review Output**: Check what would be changed
4. **Verify Safety**: Ensure semantic validation passed
5. **Apply Changes**: Run with dryRun=false
6. **Sync Graph**: Run Sync tool to rebuild knowledge graph

### Incremental Cleanup
Process concept families one at a time rather than bulk operations:
```bash
# Step 1: Fix "tool" family
RepairConcepts conceptsToReplace='tools,Tools' canonicalConcept='tool' dryRun=true

# Step 2: Fix "agent" family
RepairConcepts conceptsToReplace='agents,Agents' canonicalConcept='agent' dryRun=true

# Step 3: Fix "test" family
RepairConcepts conceptsToReplace='tests,Tests' canonicalConcept='test' dryRun=true
```

### Folder-Scoped Repairs
Test repairs on small subfolders before processing entire memory:
```json
{
  "conceptsToReplace": "experimental,Experimental",
  "canonicalConcept": "experiment",
  "folder": "research/experiments",
  "dryRun": true
}
```

## Related Tools

- **AnalyzeConceptCorruption**: ⚠️ ALWAYS run FIRST to understand what needs repair
- **Sync**: ALWAYS run AFTER repairs to rebuild [[knowledge-graph]] with cleaned concepts
- **SearchMemories**: Find files using concepts before deciding to consolidate
- **BuildContext**: Explore concept relationships to understand consolidation impact
- **Visualize**: See concept graph structure to verify repairs worked correctly

## Troubleshooting

### Error: "WARNING: Unsafe consolidation detected!"
**Cause**: Semantic similarity validation failed - variants are not semantically equivalent
**Solution**: Review the similarity scores. If consolidation is truly safe, either:
  1. Reconsider if these concepts should remain distinct (usually the right answer)
  2. Set minSemanticSimilarity=0.0 to override (only if manually verified safe)

### Error: "Cannot both create WikiLinks and remove them"
**Cause**: Set createWikiLinks=true with empty canonicalConcept
**Solution**: Choose one mode: create WikiLinks OR remove them, not both

### Result: "Files scanned: 0"
**Cause**: Folder parameter points to non-existent directory
**Solution**: Verify folder path relative to memory root, or omit for full memory scan

### Semantic Validation Warning But Operation Seems Safe
**Cause**: Embeddings don't capture context (e.g., "tests" vs "test" may have lower similarity than expected)
**Solution**: Review specific use cases, then override with minSemanticSimilarity=0.0 if truly safe

### Changes Not Reflected in Graph
**Cause**: Forgot to run Sync after applying repairs
**Solution**: Always run Sync tool after RepairConcepts to rebuild knowledge graph

## Output Structure

### Dry Run Output
```
=== SEMANTIC VALIDATION ===
Checking semantic similarity with threshold 0.70...
All variants passed semantic validation.

Scanning 245 markdown files...
Looking for variants: tools, Tools, TOOLS
Will replace with: [[tool]] (normalized: tool)

Would modify: research/machine-learning/transformers.md
  [[tools]] → [[tool]]
  [[Tools]] → [[tool]]
Would modify: projects/2024/ai-infrastructure.md
  [[TOOLS]] → [[tool]]

=== SUMMARY ===
Files scanned: 245
Files to modify: 2
Total replacements: 3

This was a DRY RUN. To apply changes, run with dryRun=false

After applying changes, run 'sync' to rebuild the graph with clean concepts.
```

### Actual Run Output
```
✓ Modified: research/machine-learning/transformers.md
  [[tools]] → [[tool]]
  [[Tools]] → [[tool]]
✓ Modified: projects/2024/ai-infrastructure.md
  [[TOOLS]] → [[tool]]

=== SUMMARY ===
Files scanned: 245
Files modified: 2
Total replacements: 3

✓ Changes applied successfully!
Run 'sync' to rebuild the graph with the cleaned concepts.
```

## Example Repair Session

### Step 1: Analyze the Problem
```json
{
  "conceptFamily": "tool"
}
```
Output shows: [[tool]] (245x), [[tools]] (89x), [[Tools]] (12x), [[VectorTools.cs]] (18x)

### Step 2: Plan Safe Repairs
✅ Safe to merge: [[tools]], [[Tools]] → [[tool]]
🚨 DO NOT merge: [[VectorTools.cs]] (file path)

### Step 3: Dry Run
```json
{
  "conceptsToReplace": "tools,Tools",
  "canonicalConcept": "tool",
  "dryRun": true
}
```

### Step 4: Review Semantic Validation
```
=== SEMANTIC VALIDATION ===
All variants passed semantic validation.
  - tools: similarity 0.983
  - Tools: similarity 0.983
```

### Step 5: Review Changes
```
Would modify: 15 files
Total replacements: 101
```

### Step 6: Apply Repairs
```json
{
  "conceptsToReplace": "tools,Tools",
  "canonicalConcept": "tool",
  "dryRun": false
}
```

### Step 7: Rebuild Graph
```json
{
  "tool": "Sync"
}
```

## Ma Protocol Compliance

RepairConcepts follows Maenifold's Ma Protocol principles:
- **Real File Modification**: Changes source markdown files, not just database caches
- **Safety First**: Dry run mode and semantic validation prevent accidental damage
- **Transparent Operation**: Shows exactly what will change before applying
- **No Magic**: Direct file text replacement with clear regex patterns
- **Validation Required**: Forces understanding through AnalyzeConceptCorruption workflow
- **Single Responsibility**: Focused on concept consolidation, nothing more

This tool embodies Ma Protocol's principle of **preserving space through careful action** - the semantic validation ensures you don't collapse important conceptual distinctions in your [[knowledge-graph]].
