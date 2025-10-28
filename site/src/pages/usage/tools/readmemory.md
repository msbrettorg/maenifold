---
layout: ../../../layouts/DocsLayout.astro
---

# ReadMemory

## Purpose
Retrieve content from existing memory files using either URI or title lookup patterns. Essential for accessing stored knowledge before editing, referencing, or building context around existing concepts.

## When to Use This Tool
- **Before EditMemory**: Always read first to get checksum for safe editing
- **Content retrieval**: Access full file content with metadata for review
- **Verification**: Confirm file exists and check current content before modifications  
- **Context building**: Retrieve files to understand existing knowledge before creating related content
- **Checksum validation**: Get current file state for preventing edit conflicts
- **File exploration**: Browse content when you know title but not exact location
- **URI resolution**: Convert between file titles and memory:// URI references

## Key Features
- **Dual lookup methods**: Find files by exact `memory://` URI or by file title
- **Checksum integration**: Returns SHA-256 checksum for edit conflict prevention
- **Metadata display**: Shows creation/modification timestamps in local timezone
- **Location tracking**: Displays both URI and relative file path for reference
- **Title-based search**: Automatically finds files when given human-readable titles
- **Full content retrieval**: Returns complete file content with frontmatter metadata
- **URI format handling**: Seamlessly converts between `memory://path/file` and filesystem paths

## Parameters
| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| identifier | string | Yes | Memory FILE identifier - either `memory://uri` or file title | `"memory://projects/ai-research"` or `"AI Research Notes"` |
| includeChecksum | bool | No | Return checksum with content for safe editing (default: true) | `true` |

## Usage Examples

### Basic URI Lookup
```json
{
    "identifier": "memory://projects/machine-learning",
    "includeChecksum": true
}
```
Retrieves file at `memory://projects/machine-learning` with full metadata and checksum for safe editing.

### Title-Based Lookup  
```json
{
    "identifier": "Machine Learning Fundamentals",
    "includeChecksum": true
}
```
Finds file by title, automatically resolving to correct file path regardless of folder location.

### Quick Content Check (No Checksum)
```json
{
    "identifier": "memory://daily/2024-01-15",
    "includeChecksum": false
}
```
Retrieves content without checksum when you only need to read, not edit.

## Common Patterns

### **Pre-Edit Pattern**: Always read before editing
```json
// Step 1: Read to get current state
{"identifier": "Project Notes", "includeChecksum": true}

// Step 2: Use returned checksum in EditMemory
{"identifier": "Project Notes", "operation": "append", "content": "New info about [[AI]]", "checksum": "abc123..."}
```

### **URI vs Title Decision Making**:
- **Use URI** when you have exact memory:// reference from previous operations
- **Use Title** when you know the human-readable file name but not location
- **URI is faster** - direct file access without search
- **Title is flexible** - finds files regardless of folder structure

### **Content Verification Pattern**:
```json
// Verify file exists and check current content
{"identifier": "Research Notes", "includeChecksum": false}

// If exists, proceed with operations; if not, create with WriteMemory
```

## Related Tools
- **WriteMemory**: Create new files (use ReadMemory first to avoid duplicates)
- **EditMemory**: Modify existing files (requires ReadMemory checksum for safety)  
- **SearchMemories**: Find files by content when you don't know exact title
- **BuildContext**: Explore concept relationships from files found via ReadMemory
- **ExtractConceptsFromFile**: Analyze [[WikiLink]] concepts in retrieved files

## Troubleshooting

### **Error**: "Memory file not found: [identifier]"
**Cause**: File doesn't exist or identifier is incorrect
**Solutions**:
- Use SearchMemories to find similar file titles
- Check if identifier is `memory://` URI or plain title
- Verify file wasn't moved or deleted with ListMemories
- Try with different title variations (check spelling)

### **Issue**: Getting wrong file with title lookup
**Cause**: Multiple files with similar slugified names
**Solutions**:
- Use exact `memory://` URI instead of title
- Use SearchMemories to find all matching files
- Add folder path to make title more specific

### **Issue**: Missing content or metadata
**Cause**: File exists but has formatting issues
**Solutions**:
- Check file directly with ListMemories
- File may have corrupted frontmatter
- Content might be empty (valid but shows no body text)

### **Issue**: Checksum needed but not included
**Cause**: Called with `includeChecksum: false` but need it for editing
**Solutions**:
- Call again with `includeChecksum: true` (default)
- Checksum is required for safe EditMemory operations
- Always include checksum when planning to edit content

## Memory System Navigation

### URI Format Understanding
- **Full URI**: `memory://folder/subfolder/filename` 
- **Root file**: `memory://filename`
- **Auto .md extension**: URI format drops .md, filesystem adds it back
- **Path separators**: Always use forward slashes in URIs

### Title Lookup Behavior  
- **Slug matching**: Titles converted to lowercase-hyphenated form
- **Case insensitive**: "AI Research" matches "ai-research.md"
- **Folder agnostic**: Searches all subdirectories automatically
- **First match wins**: Returns first file found with matching slug

### Content Structure
ReadMemory returns formatted content with:
1. **File title** (from frontmatter or filename)
2. **URI reference** for future tool calls
3. **Relative file location** for human reference  
4. **Creation/modification timestamps** in local timezone
5. **Checksum** (if requested) for edit safety
6. **Full file content** after metadata separator

This tool is essential for safe file operations and knowledge retrieval in the Maenifold memory system.