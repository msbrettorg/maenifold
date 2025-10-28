---
layout: ../../../layouts/DocsLayout.astro
---

# ExtractConceptsFromFile

## Purpose
Extract [[WikiLink]] concepts from memory files to analyze what knowledge graph nodes exist in specific files before graph synchronization.

## When to Use This Tool
- Before running Sync to see what concepts a file would contribute to the knowledge graph
- To audit existing memory files for concept density and knowledge graph connectivity
- When debugging why certain concepts aren't appearing in BuildContext results
- To validate that memory files contain proper [[WikiLink]] formatting before graph operations
- During content analysis to understand the conceptual structure of stored knowledge

## Key Features
- **WikiLink Extraction**: Identifies all [[Concept Name]] patterns in file content
- **Graph Node Discovery**: Shows exactly what nodes would be added to the knowledge graph
- **Content Validation**: Helps identify files that lack proper concept linking
- **Pre-Sync Analysis**: Enables verification before expensive graph rebuild operations
- **Debugging Support**: Assists in troubleshooting missing concepts in graph queries

## Parameters
| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| identifier | string | Yes | Memory FILE identifier (URI or title) | "memory://advanced-ai-concepts" |

## Usage Examples

### Basic Extraction
```json
{"identifier": "memory://machine-learning-fundamentals"}
```
Extracts all [[WikiLink]] concepts from the machine learning fundamentals file, showing concepts like [[Neural Networks]], [[Deep Learning]], [[Supervised Learning]].

### Title-Based Lookup  
```json
{"identifier": "AI Research Notes"}
```
Finds the file by title and extracts concepts, useful when you remember the title but not the URI.

### Pre-Sync Validation
```json
{"identifier": "memory://new-research-paper"}
```
Validate that a newly created file contains proper [[concept]] links before running Sync to update the knowledge graph.

## Common Patterns
- **Content Audit**: Extract concepts from multiple files to understand knowledge graph coverage
- **Pre-Sync Check**: Verify files have concepts before expensive graph rebuild operations  
- **Debug Missing Concepts**: When BuildContext doesn't find expected connections, check if concepts exist in source files
- **Quality Control**: Ensure new memory files follow [[WikiLink]] conventions for graph connectivity
- **Knowledge Mapping**: Analyze what conceptual domains are covered in your memory collection

## Related Tools
- **Sync**: Use ExtractConceptsFromFile to preview what concepts Sync will process from files
- **BuildContext**: After extraction, use BuildContext to explore how concepts relate in the knowledge graph
- **SearchMemories**: Find files containing specific concepts, then extract to see all concepts in those files
- **WriteMemory/EditMemory**: Ensure content has proper [[concepts]] before extraction analysis

## Troubleshooting

### No Concepts Found
- **Issue**: "No [[WikiLink]] concepts found in file"
- **Solution**: File content lacks double-bracketed concept names. Add [[Concept Name]] formatting to connect content to knowledge graph.

### File Not Found  
- **Issue**: "ERROR: Memory file not found: {identifier}"
- **Solutions**:
  - Verify URI format: should be "memory://filename" not "memory:filename"  
  - Check file exists with ReadMemory or SearchMemories first
  - Ensure title matches exactly if using title-based lookup

### Empty Results vs Expected Concepts
- **Issue**: Extraction shows fewer concepts than expected
- **Solutions**:
  - Check for proper [[double bracket]] formatting (not [single] or ((double parens)))
  - Verify concepts aren't split across lines: [[Concept]] not [[\nConcept]]
  - Look for typos in bracket formatting: [[Concept]]] (extra bracket) won't match

### Performance with Large Files
- **Issue**: Slow extraction from very large files
- **Solutions**:
  - ExtractConceptsFromFile is optimized for Ma Protocol 250-line files
  - For oversized files, consider splitting content using EditMemory with replace_section
  - Large files may indicate violation of Maenifold architectural principles

## Integration Examples

### Pre-Sync Workflow
```
1. ExtractConceptsFromFile → See what concepts exist
2. Sync → Update knowledge graph with those concepts  
3. BuildContext → Explore the newly connected concept relationships
```

### Content Quality Pipeline
```  
1. WriteMemory → Create file with [[concepts]]
2. ExtractConceptsFromFile → Verify concepts were properly formatted
3. SearchMemories → Confirm file is discoverable via concept search
```

### Debug Missing Connections
```
1. BuildContext fails to find expected connections
2. ExtractConceptsFromFile → Check if concepts exist in source files
3. If missing: EditMemory to add proper [[WikiLink]] formatting
4. Sync → Rebuild graph with corrected concept links
```

## Technical Notes
- **File Format**: Works with Maenifold markdown files containing YAML frontmatter
- **URI Resolution**: Supports both memory:// URIs and title-based file lookup
- **Concept Format**: Extracts only [[double bracketed]] text as valid concepts
- **Case Sensitivity**: Concept extraction preserves exact case from file content
- **Performance**: Optimized for Ma Protocol file sizes (under 250 lines)
- **Graph Independence**: Shows concepts without requiring knowledge graph to be built