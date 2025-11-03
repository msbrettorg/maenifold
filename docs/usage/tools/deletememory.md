# Delete Memory

**Permanently removes a memory FILE from the file system and knowledge graph.**

This tool provides safe deletion of memory files with mandatory confirmation to prevent accidental data loss. When a file is deleted, it is completely removed from disk and will no longer appear in searches, context building, or graph relationships after the next sync operation.

## When to Use This Tool

- **Removing outdated or incorrect information** that pollutes the knowledge graph
- **Cleaning up experimental files** or temporary notes that are no longer needed  
- **Deleting duplicate content** that creates confusion in search results
- **Removing sensitive information** that shouldn't persist in the memory system
- **Tidying up abandoned workflow/thinking sessions** that clutter the knowledge base
- **Correcting file organization mistakes** where a file was created in error

## Key Features

- **Mandatory confirmation** prevents accidental deletions (`confirm=true` required)
- **Flexible identifier support** accepts both memory:// URIs and file titles
- **Complete removal** deletes the actual file from disk permanently
- **Graph impact awareness** - deleted concepts will be cleaned up on next Sync
- **No backup creation** - deletion is immediate and irreversible
- **URI and title lookup** automatically resolves file location

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| identifier | string | Yes | Memory FILE identifier (memory:// URI or title) | "memory://notes/project-notes.md" or "Project Notes" |
| confirm | bool | No | Must be set to true to confirm deletion | true |

## Usage Examples

### Basic File Deletion
```json
{
  "identifier": "Project Planning Session",
  "confirm": true
}
```
Permanently deletes the file with title "Project Planning Session".

### URI-Based Deletion  
```json
{
  "identifier": "memory://experiments/test-workflow.md",
  "confirm": true
}
```
Deletes the specific file at the given memory:// URI path.

### Safety Check (Confirmation Required)
```json
{
  "identifier": "Important Research Notes",
  "confirm": false
}
```
Returns error: "Must set confirm=true to delete a memory file" - prevents accidental deletion.

## Common Patterns

- **Pre-deletion verification**: Use ReadMemory first to confirm you're deleting the correct file
- **Cleanup workflows**: Delete temporary files created during thinking sessions or experiments
- **Content consolidation**: Remove duplicate files after merging content into comprehensive documents
- **Privacy protection**: Delete files containing sensitive information that should not persist
- **Knowledge graph maintenance**: Remove files with incorrect [[concepts]] that create false relationships

## Related Tools

- **ReadMemory**: Verify file content before deletion to ensure you're removing the correct file
- **SearchMemories**: Find files to delete based on content patterns or metadata
- **MoveMemory**: Alternative to deletion - relocate files to archive folders instead of removing
- **Sync**: Must be run after deletion to update the concept graph and remove orphaned relationships
- **BuildContext**: Check if a concept will become orphaned before deleting its only source file

## Graph Impact and Relationships

### Concept Graph Effects
When you delete a memory file, the following happens to the knowledge graph:

- **Immediate**: File is removed from disk and can no longer be read
- **After Sync**: Concept mentions from this file are removed from the database
- **Orphaned Concepts**: If this was the only file containing certain [[concepts]], those concepts may become orphaned
- **Broken Relationships**: Concept relationships that depended on this file will be weakened or removed

### Impact Assessment Workflow
Before deleting important files:
1. Use `ExtractConceptsFromFile` to see what [[concepts]] will be affected
2. Use `BuildContext` to understand which concept relationships depend on this file  
3. Use `SearchMemories` to find other files that contain the same concepts
4. Only delete if concepts exist in other files or are truly obsolete

## Troubleshooting

### Common Errors and Solutions

- **Error**: "Must set confirm=true to delete a memory file" 
  → **Solution**: Always include `"confirm": true` in the JSON payload to enable deletion

- **Error**: "Memory file not found: [identifier]"
  → **Solution**: Check the identifier spelling or use SearchMemories to find the correct file name/URI

- **File not found by title**
  → **Solution**: Use exact title match or switch to memory:// URI format for precise targeting

- **Deletion impact concerns**
  → **Solution**: Run ExtractConceptsFromFile first to see which [[concepts]] will be affected

### Best Practices for Safe Deletion

1. **Always verify first**: Use ReadMemory to confirm file contents before deletion
2. **Check concept dependencies**: Use BuildContext to understand graph impact  
3. **Consider archiving**: Use MoveMemory to relocate instead of delete for important content
4. **Sync after cleanup**: Run Sync after deletion sessions to update the concept graph
5. **Batch cleanup carefully**: Delete files one at a time to avoid accidental mass deletion

## Security and Safety

- **No recovery mechanism**: Deleted files cannot be recovered through Maenifold
- **Immediate effect**: File is removed from disk as soon as the tool executes successfully
- **Confirmation requirement**: The `confirm` parameter prevents accidental execution
- **Graph consistency**: Run Sync after deletions to maintain knowledge graph integrity
- **Backup responsibility**: Maenifold does not create backups - use external version control

## Performance Considerations

- **Fast execution**: File deletion is immediate and lightweight
- **No batch operations**: Delete files individually for safety and control
- **Graph updates deferred**: Concept graph cleanup happens during next Sync operation
- **Search index impact**: Deleted files are removed from search results after Sync
- **Disk space recovery**: File system space is immediately reclaimed upon deletion