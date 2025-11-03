# EditMemory

Edit an existing memory FILE to add, modify, or replace content while maintaining [[concept]] connections to the knowledge graph. This tool provides safe editing operations with checksum validation to prevent conflicting changes and ensures all edits contribute to the Maenifold graph structure.

## When to Use This Tool

- **Updating existing knowledge**: Add new information to existing memory files
- **Correcting or refining content**: Fix errors or improve existing documentation
- **Replacing outdated sections**: Update specific sections with current information
- **Appending related insights**: Add new thoughts or discoveries to existing topics
- **Prepending context**: Add introductory or prerequisite information
- **Find and replace operations**: Systematically update terminology or references
- **Section-specific updates**: Replace entire sections while preserving structure

## Key Features

- **[[Concept]] requirement**: All new content must contain at least one [[concept]] to maintain graph connectivity
- **Checksum validation**: Prevents conflicting edits through file integrity checking
- **Multiple operation types**: append, prepend, find_replace, replace_section for different editing needs
- **Safe transformations**: Validates that edits don't orphan files from the knowledge graph
- **Automatic timestamps**: Updates modification timestamps for change tracking
- **Expected match validation**: find_replace operations can validate match counts for safety
- **Section-aware editing**: Can replace specific markdown sections by header name

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| identifier | string | Yes | Memory FILE identifier (URI or title) | "memory://documents/machine-learning.md" |
| operation | string | Yes | Edit operation type | "append", "prepend", "find_replace", "replace_section" |
| content | string | Yes | Content to add/replace - MUST contain [[concepts]] | "New insights about [[Neural Networks]]" |
| checksum | string | No | File checksum from ReadMemory (prevents stale edits) | "a1b2c3d4e5f6" |
| findText | string | No | Text to find (required for find_replace) | "old terminology" |
| sectionName | string | No | Section header name (required for replace_section) | "Implementation Details" |
| expectedCount | int | No | Expected match count for find_replace validation | 3 |

## Operation Types

### append
Adds new content to the end of the existing file.
```json
{
  "identifier": "memory://documents/machine-learning.md",
  "operation": "append",
  "content": "## Recent Discoveries\n\nNew research in [[Deep Learning]] shows promising results with [[Transformer Architecture]]."
}
```

### prepend
Adds new content to the beginning of the existing file.
```json
{
  "identifier": "memory://documents/machine-learning.md", 
  "operation": "prepend",
  "content": "# Prerequisites\n\nThis document assumes familiarity with [[Statistics]] and [[Linear Algebra]].\n\n"
}
```

### find_replace
Replaces all occurrences of specific text with new content.
```json
{
  "identifier": "memory://documents/machine-learning.md",
  "operation": "find_replace", 
  "content": "[[Machine Learning]] algorithms",
  "findText": "ML algorithms",
  "expectedCount": 5
}
```

### replace_section
Replaces content under a specific markdown section header.
```json
{
  "identifier": "memory://documents/machine-learning.md",
  "operation": "replace_section",
  "content": "Current [[TensorFlow]] and [[PyTorch]] implementations provide excellent performance.",
  "sectionName": "Implementation Tools"
}
```

## Usage Examples

### Basic Append Example
```json
{
  "identifier": "research-notes",
  "operation": "append", 
  "content": "## Latest Findings\n\nDiscovered important connection between [[Attention Mechanisms]] and [[Memory Networks]]."
}
```
Adds new section to the end of the research-notes file.

### Safe Find-Replace with Validation
```json
{
  "identifier": "memory://documents/ai-overview.md",
  "operation": "find_replace",
  "content": "[[Artificial Intelligence]] systems",
  "findText": "AI systems", 
  "checksum": "a1b2c3d4e5f6",
  "expectedCount": 12
}
```
Replaces "AI systems" with properly linked concept, validating exactly 12 replacements occur.

### Section Replacement Example
```json
{
  "identifier": "project-status", 
  "operation": "replace_section",
  "content": "Project completed successfully. Key insights include [[Agile Methodology]] benefits and [[Continuous Integration]] improvements.",
  "sectionName": "Current Status"
}
```
Replaces the entire "Current Status" section with updated information.

## Common Patterns

### Safe Editing Workflow
1. **Read first**: Use ReadMemory with includeChecksum=true to get current state
2. **Plan changes**: Identify what needs updating and ensure [[concepts]] are included
3. **Edit with checksum**: Use the checksum from ReadMemory to prevent conflicts
4. **Verify results**: Check that operation succeeded and file maintains graph connectivity

### Collaborative Editing
- Always use checksums when multiple editors might modify the same file
- Use expectedCount for find_replace to catch unexpected changes
- Coordinate section-based edits to avoid conflicts

### Graph Maintenance
- Every edit must include at least one [[concept]] to maintain knowledge graph connections
- Use existing concepts where possible, create new ones [[Like This]] when needed
- Avoid edits that would remove all [[concepts]] from a file

## Related Tools

- **ReadMemory**: Get current file content and checksum before editing
- **WriteMemory**: Create new memory files when editing isn't appropriate
- **SearchMemories**: Find files that need updating across the knowledge base
- **Sync**: Update the knowledge graph after significant edits
- **BuildContext**: Explore [[concept]] relationships after editing

## Troubleshooting

### "ERROR: Checksum mismatch"
**Cause**: File was modified between ReadMemory and EditMemory calls
**Solution**: Re-read the file with ReadMemory to get current checksum, then retry edit

### "ERROR: New content must contain at least one [[concept]]"
**Cause**: Edit content doesn't include any [[WikiLink]] concepts
**Solution**: Add relevant [[Concept Names]] to connect content to knowledge graph

### "ERROR: This edit would remove all [[concepts]] from the file"
**Cause**: find_replace or replace_section would eliminate all graph connections
**Solution**: Ensure replacement content includes appropriate [[concepts]]

### "Expected X matches but found Y" 
**Cause**: expectedCount validation failed in find_replace operation
**Solution**: Check if file was modified or adjust expectedCount parameter

### "ERROR: Memory file not found"
**Cause**: Invalid identifier or file doesn't exist
**Solution**: Use SearchMemories to find correct identifier or verify file exists

### Section replacement not working
**Cause**: sectionName doesn't match exact markdown header text
**Solution**: Check exact section header text including capitalization and punctuation

## Implementation Notes

- **Checksum validation**: Based on entire file content, detects any external changes
- **[[Concept]] validation**: Ensures both new content and final result maintain graph connectivity  
- **Timestamp updates**: Automatically updates 'modified' frontmatter field
- **Section matching**: Uses regex pattern matching for flexible section identification
- **File integrity**: Validates file structure and frontmatter after edits

## Ma Protocol Compliance

- **Simple operations**: Four clear operation types without complex parameter combinations
- **Safe by default**: Checksum validation prevents accidental overwrites
- **Graph-first design**: [[Concept]] requirements ensure knowledge graph integrity
- **Minimal complexity**: Each operation does exactly one thing well