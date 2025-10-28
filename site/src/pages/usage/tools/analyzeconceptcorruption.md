---
layout: ../../../layouts/DocsLayout.astro
---

# AnalyzeConceptCorruption

⚠️ **MUST USE BEFORE RepairConcepts!** Analyzes [[knowledge-graph]] [[concept]] corruption patterns to identify what needs repair without making any changes to your memory files. This diagnostic tool reveals [[concept]] families and their variants to help plan SAFE [[graph]] repair operations that preserve semantic meaning.

## When to Use This Tool

- **Before RepairConcepts**: ALWAYS run this first to understand what you're about to change
- **Graph Health Audits**: Identify [[concept]] [[fragmentation]] patterns (plurals, case variations, compounds)
- **Semantic Safety Checks**: Distinguish safe merges (plurals) from dangerous ones (class names, specific types)
- **Knowledge Graph Quality**: Assess the cleanliness and consistency of your [[WikiLink]] usage
- **Planning Consolidation**: Generate repair command suggestions based on detected patterns
- **Understanding Structure**: Reveal how concepts relate within your knowledge system
- **Preventing Mistakes**: Avoid destroying semantic meaning by analyzing before acting

## Key Features

- **Zero-Risk Analysis**: Read-only operation that never modifies your memory files
- **Pattern Detection**: Automatically categorizes variants (Singular/Plural, Compound forms, File Paths, With Suffix)
- **Safety Classification**: Identifies which variants are safe to merge vs dangerous to consolidate
- **Usage Statistics**: Shows occurrence counts for each variant to inform prioritization
- **Repair Command Generation**: Suggests specific RepairConcepts commands based on detected patterns
- **Semantic Structure Discovery**: Reveals the conceptual organization of your knowledge graph
- **Family-Based Analysis**: Focuses on related concepts (e.g., all "tool" variants)

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| conceptFamily | string | Yes | Concept family to analyze (base concept to search for) | "tool", "agent", "test" |
| maxResults | int | No | Maximum variants to show (default 50) | 100 |

## Usage Examples

### Basic Concept Family Analysis
```json
{
  "conceptFamily": "tool"
}
```
Analyzes all concept variants containing "tool" - shows [[tools]] (plural), [[MCP-tool]], [[tool-agent]], etc.

### Large-Scale Analysis
```json
{
  "conceptFamily": "agent",
  "maxResults": 100
}
```
Comprehensive analysis of "agent" concept family with extended result limit for thorough review.

### Testing Concept Analysis
```json
{
  "conceptFamily": "test"
}
```
Reveals test-related concepts: [[tests]], [[testing]], [[test-agent]], [[test-coverage]], [[TestRunner.cs]], etc.

### MCP Concept Review
```json
{
  "conceptFamily": "MCP"
}
```
Shows MCP-related variants: [[MCP]], [[mcp]], [[MCP-server]], [[MCP tools]], etc.

## Output Interpretation Guide

### Safe to Merge (Low Risk)
- **Singular/Plural**: [[tool]] vs [[tools]] - Almost always safe to standardize on singular form
- **Case Variations**: [[MCP]] vs [[mcp]] vs [[Mcp]] - Safe to standardize casing
- **Whitespace Differences**: [[AI agent]] vs [[AI-agent]] - Usually safe if semantically identical

### DANGEROUS to Merge (High Risk)
- **File Paths**: [[GraphTools.cs]], [[/src/Tools/GraphTools.cs]] - These are FILE REFERENCES, not concepts
- **Class Names**: [[VectorTools]], [[GraphTools]] - These are CODE ENTITIES, merging with [[tool]] destroys meaning
- **Compound Forms**: [[coding-agent]], [[test-agent]] - These are SPECIFIC TYPES, not generic [[agent]]
- **With Suffix**: [[tool-framework]], [[agent-system]] - These are DISTINCT CONCEPTS with specialized meaning

### Pattern Categories Explained

**Singular/Plural**
```
Singular/Plural:
  • tool (245x)
  • tools (89x)
```
Safe to consolidate "tools" → "tool" - standard pluralization cleanup.

**Compound (with -)**
```
Compound (with -):
  • coding-agent (34x)
  • test-agent (12x)
  • MCP-server (67x)
```
⚠️ CAREFUL: These are often SPECIFIC TYPES. [[coding-agent]] is NOT the same as [[agent]]!

**File Paths**
```
File Paths:
  • GraphTools.cs (23x)
  • /src/Tools/VectorTools.cs (8x)
```
🚨 NEVER MERGE WITH GENERIC CONCEPTS: File paths are references, not conceptual [[WikiLinks]].

**With Suffix**
```
With Suffix:
  • agent-framework (45x)
  • tool-architecture (18x)
```
⚠️ REVIEW CAREFULLY: Often distinct concepts that should NOT be merged with base term.

## Suggested Repairs

The tool generates specific RepairConcepts commands based on detected patterns:

### Safe Repair Example
```
Fix plural forms:
  RepairConcepts conceptsToReplace='tools,Tools,TOOLS' canonicalConcept='tool'
```
This consolidation is SAFE - just standardizing singular/plural and casing.

### Potentially Dangerous Repair Example
```
Fix compound forms:
  RepairConcepts conceptsToReplace='coding-agent,test-agent,deployment-agent' canonicalConcept='agent'
```
⚠️ DANGEROUS! This would destroy specificity - [[coding-agent]] is a specialized agent type, not generic [[agent]].

## Critical Warnings

### DO NOT Blindly Run Suggested Repairs
The tool suggests repairs based on PATTERNS, not SEMANTIC MEANING. Always review:
1. Are these truly duplicates or distinct concepts?
2. Would consolidation lose important semantic information?
3. Are any of these class names, file references, or specific entity types?

### Understanding the Difference
- **Safe**: [[MCP Tools]] → [[MCP]] (case/plural standardization)
- **UNSAFE**: [[VectorTools]] → [[tool]] (destroys "this is the VectorTools CLASS" meaning)

- **Safe**: [[agents]] → [[agent]] (plural consolidation)
- **UNSAFE**: [[coding-agent]] → [[agent]] (destroys "this is a CODING agent" specificity)

## Common Patterns

### Graph Health Assessment Workflow
1. Run AnalyzeConceptCorruption on key concept families (tool, agent, test, MCP)
2. Review output to understand fragmentation patterns
3. Identify safe consolidations (plurals, casing) vs dangerous ones (compounds, file paths)
4. Plan repair operations with semantic safety in mind

### Before Major Refactoring
Run analysis on relevant concept families before using RepairConcepts to ensure you understand the full impact of proposed changes.

### Knowledge Graph Audits
Periodic analysis reveals how [[concept]] usage evolves and where consistency improvements are needed.

### New Concept Introduction
Before adding new concepts, check if similar ones exist to avoid creating duplicate concepts with different names.

## Related Tools

- **RepairConcepts**: ⚠️ ONLY use AFTER analyzing with this tool - makes actual changes to files
- **Sync**: Run after repairs to rebuild [[knowledge-graph]] with cleaned concepts
- **BuildContext**: Explore concept relationships to understand impact of consolidation
- **SearchMemories**: Find files using specific concepts before planning consolidation
- **Visualize**: See concept relationships graphically to understand merge impact

## Troubleshooting

### Result: "Found 0 unique variants"
**Cause**: No concepts match the conceptFamily search term
**Solution**: Try different search terms - use base words like "tool" not compounds like "tool-system"

### Too Many Results
**Cause**: Very common concept family with hundreds of variants
**Solution**: Increase maxResults parameter or focus on more specific concept families

### Missing Expected Variants
**Cause**: Concepts use different naming than expected
**Solution**: Run SearchMemories to find how the concept is actually used in your knowledge base

### Compound vs Simple Forms Unclear
**Cause**: Uncertain if compound is specific type or just variation
**Solution**: Use SearchMemories with the compound concept to see how it's used contextually

## Example Analysis Session

### Step 1: Analyze the Concept Family
```json
{
  "conceptFamily": "tool",
  "maxResults": 50
}
```

### Step 2: Review Output
```
=== Concept Family Analysis: 'tool' ===
Found 15 unique variants

Singular/Plural:
  • tool (245x)
  • tools (89x)
  • Tools (12x)

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

### Step 3: Evaluate Safety
- ✅ **SAFE**: Consolidating [[tools]], [[Tools]] → [[tool]] (plural/case standardization)
- ⚠️ **REVIEW**: [[MCP-tool]] might be distinct from [[tool]] - check usage context
- 🚨 **DANGER**: [[VectorTools.cs]], [[GraphTools.cs]] are FILE PATHS - NEVER merge with [[tool]]

### Step 4: Plan Repairs
Only consolidate the SAFE plurals:
```json
{
  "conceptsToReplace": "tools,Tools",
  "canonicalConcept": "tool",
  "dryRun": true
}
```

## Ma Protocol Compliance

AnalyzeConceptCorruption follows Maenifold's Ma Protocol principles:
- **Read-Only Safety**: Never modifies files - pure analysis for informed decision-making
- **Transparent Logic**: Clear pattern categorization showing exactly what was found
- **No Magic**: Direct file scanning with straightforward WikiLink pattern matching
- **Real Data**: Shows actual usage counts and file references, not estimates
- **Minimal Complexity**: Static method, simple parameters, clear output structure

This tool embodies the Ma principle of **creating space for understanding** before action - analyze first, repair second, never the reverse.
