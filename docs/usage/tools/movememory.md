# MoveMemory

## Purpose
Move or rename memory FILES within the Maenifold system, updating URIs and maintaining file integrity with automatic timestamp tracking.

## When to Use This Tool
- Renaming files to better reflect updated content or purpose
- Reorganizing files into new folder structures for improved organization  
- Moving files between different thematic directories in your memory system
- Fixing filename issues like typos or inconsistent naming patterns
- Restructuring your knowledge base architecture after learning new organizational patterns
- Consolidating related files into subject-specific folders
- Moving orphaned files into appropriate categorical structures

## Key Features
- **Automatic URI Updates**: Source and destination URIs are automatically generated and tracked
- **Metadata Preservation**: All frontmatter including title, permalink, type, status, created date, and tags are preserved
- **Timestamp Tracking**: Modified timestamp is automatically updated during the move operation
- **Directory Creation**: Destination directories are created automatically if they don't exist
- **Source Cleanup**: Original file is automatically deleted after successful move to prevent duplicates
- **Flexible Path Handling**: Supports both simple renaming and complex directory restructuring
- **Title-Based Lookup**: Can identify source files by title instead of requiring URI
- **Cross-Platform Paths**: Handles both forward slashes and backslashes for destination paths

## Parameters
| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| source | string | Yes | Source FILE identifier (memory:// URI or file title) | "memory://learning-notes" or "Learning Notes" |
| destination | string | Yes | Destination FILE name or path | "deep-learning-notes" or "ai/machine-learning/deep-learning.md" |

## Usage Examples

### Basic File Renaming
```json
{"source": "old-filename", "destination": "new-filename"}
```
Renames "old-filename.md" to "new-filename.md" in the same directory while preserving all metadata and updating the modified timestamp.

### Moving to New Directory Structure
```json
{"source": "memory://scattered-notes", "destination": "research/ai/neural-networks"}
```
Moves the file from its current location to the "research/ai/" folder with filename "neural-networks.md", creating the directory structure if needed.

### Folder Reorganization 
```json
{"source": "temp-notes", "destination": "projects/2025/maenifold-research"}
```
Relocates a temporary file into a proper project structure, demonstrating how to organize files by time period and project scope.

### Cross-Directory File Movement
```json
{"source": "memory://general/random-thoughts", "destination": "philosophy/consciousness/maenifold-theory"}
```
Moves a file from a general folder into a specialized philosophical topic structure, showing deep categorization.

## Common Patterns

### **Pattern 1: Simple Renaming for Clarity**
Use when file titles need updating to reflect evolved understanding:
- From: "notes-about-ai" → To: "artificial-intelligence-fundamentals" 
- Improves searchability and semantic clarity without changing location

### **Pattern 2: Thematic Reorganization**
Organize files into subject-based hierarchies as knowledge accumulates:
- From: Flat structure → To: "domain/subdomain/specific-topic" 
- Creates logical knowledge pathways for better [[concept]] relationships

### **Pattern 3: Temporal Organization**
Structure files by time periods for project-based workflows:
- Pattern: "projects/YYYY/project-name/topic"
- Maintains chronological context while preserving subject categorization

### **Pattern 4: Research Pipeline Movement**
Move files through research stages as understanding deepens:
- "inbox/" → "processing/" → "research/domain/" → "knowledge/domain/"
- Reflects information maturity and processing stages

## Related Tools

- **ReadMemory**: Use to verify file content before moving, especially with URI-based identification
- **SearchMemories**: Find files by content when you need to identify the correct source for moving
- **WriteMemory**: Often used after MoveMemory when file location changes require content updates
- **EditMemory**: Update [[concept]] references that may need adjustment after reorganization
- **ListMemories**: Explore current folder structures to plan optimal destination paths
- **BuildContext**: Understand [[concept]] relationships that might influence organizational decisions

## Troubleshooting

### **Error**: "Source memory file not found: [identifier]"
**Solution**: Verify the source identifier is correct. If using a title, ensure exact match including capitalization. Use SearchMemories to find the correct title or URI.

### **Error**: File already exists at destination
**Solution**: MoveMemory will overwrite destination files. Use ReadMemory to check destination first, or choose a different destination path.

### **Issue**: Complex path separators causing confusion
**Solution**: Use forward slashes consistently in destination paths. MoveMemory automatically converts to system-appropriate separators.

### **Issue**: Lost track of file location after move
**Solution**: The move operation returns both source and destination URIs. Use SearchMemories with content snippets to relocate files if URIs are lost.

### **Issue**: Broken [[concept]] relationships after reorganization
**Solution**: Use BuildContext on affected concepts to identify files that reference moved content. Update references using EditMemory with find_replace operations.

### **Issue**: Directory structure becomes too deep
**Solution**: Plan folder hierarchies with 2-4 levels maximum. Prefer broader categories over deep nesting for better navigation and [[concept]] discovery.

## Advanced Usage Patterns

### **Batch Organization Strategy**
1. Use SearchMemories to identify files needing organization
2. Plan destination structure based on [[concept]] relationships  
3. Execute moves from least to most specific (general → detailed categories)
4. Use BuildContext to verify [[concept]] relationships remain intact
5. Update cross-references using EditMemory if needed

### **Knowledge Base Restructuring**
1. Map current organization with ListMemories across folders
2. Identify natural [[concept]] clusters through SearchMemories
3. Design new hierarchy based on actual usage patterns
4. Move files in dependency order (referenced files first)
5. Validate with SearchMemories that findability is improved

### **URI Management Best Practices**
- Always store URIs returned from MoveMemory for reference updates
- Use consistent naming patterns that reflect [[concept]] relationships
- Consider how file paths will appear in BuildContext traversals
- Plan folder names that align with your [[WikiLink]] concept naming

### **Integration with Knowledge Graph**
- Move files before major Sync operations to ensure clean graph structure
- Consider how file organization affects [[concept]] relationship discovery
- Use folder structures that mirror your conceptual hierarchies
- Organize related files near each other to strengthen [[concept]] clustering

This tool is essential for maintaining an organized, navigable knowledge base that supports effective [[concept]] discovery and relationship building within the Maenifold architecture.