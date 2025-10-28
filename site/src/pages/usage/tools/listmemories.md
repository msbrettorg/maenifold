---
layout: ../../../layouts/DocsLayout.astro
---

# ListMemories

Explore memory system file structure and navigate folder hierarchies for content discovery and file organization understanding. ListMemories provides detailed directory listings with file counts, sizes, and folder structures to support memory system navigation and file organization decisions.

## When to Use This Tool

- **Memory System Exploration**: Browse folder structures to understand how knowledge is organized
- **File Organization Discovery**: See file counts and folder layouts before creating new content
- **Directory Structure Analysis**: Understand folder hierarchies and content distribution patterns
- **Pre-Write Exploration**: Check existing folder structure before writing new memory files
- **Content Location Discovery**: Find where specific types of files are stored in your memory system
- **Folder Navigation**: Explore subdirectories to understand knowledge organization patterns
- **File Management Planning**: Assess folder structures before moving or reorganizing memory files
- **Memory System Health**: Monitor file distribution and identify over-populated directories

## Key Features

- **Hierarchical Directory Listing**: Shows folders first, then files with clear visual separation
- **File Count Indicators**: Displays markdown file count for each folder (e.g., "📁 projects/ (12 files)")
- **File Size Information**: Shows individual file sizes in KB for storage management
- **Relative Path Navigation**: Accepts relative paths from memory root for targeted exploration
- **Memory System Focus**: Only shows .md files relevant to Maenifold knowledge system
- **Visual Organization**: Uses folder 📁 and file 📄 emojis for clear content type identification
- **Root Directory Default**: Starts at memory root when no path specified
- **Error Handling**: Clear feedback for non-existent or inaccessible directories

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| path | string | No | Directory path relative to memory root (default: memory root "/") | "projects/ai-research" |

## Usage Examples

### Root Directory Exploration
```json
{}
```
Lists the root memory directory showing all top-level folders and files, providing overview of entire knowledge system organization.

### Specific Folder Navigation
```json
{
    "path": "projects"
}
```
Explores the "projects" folder, showing all project-related subfolders and files with their metadata.

### Deep Directory Navigation
```json
{
    "path": "projects/machine-learning/experiments"
}
```
Navigates to specific nested directory to understand detailed folder structure in specialized knowledge areas.

### Research Area Exploration  
```json
{
    "path": "research"
}
```
Browses research folder to see how research notes and findings are organized before adding new content.

### Daily Notes Discovery
```json
{
    "path": "daily"
}
```
Explores daily notes structure to understand chronological organization patterns for planning new entries.

## Directory Listing Format

### Folder Display
```
### Folders
📁 projects/ (15 files)
📁 research/ (8 files)  
📁 daily/ (45 files)
📁 reference/ (3 files)
```

### File Display
```
### Files
📄 ai-philosophy.md (12.3 KB)
📄 maenifold-overview.md (8.7 KB)
📄 quick-notes.md (2.1 KB)
```

## Common Patterns

### **Pre-Write Exploration Pattern**: Check folder structure before creating new files
```json
// Step 1: Explore target folder structure
{"path": "projects"}

// Step 2: Based on structure, decide on file organization
// Step 3: Use WriteMemory with appropriate folder path
```

### **Memory System Navigation**: Progressive folder exploration
```json
// Start broad: explore root structure
{}

// Narrow focus: explore specific areas
{"path": "projects/current"}

// Deep dive: specific folder for targeted work
{"path": "projects/current/ai-research"}
```

### **Content Organization Assessment**: Understand existing structure
```json
// Check how knowledge is currently organized
{"path": "research"}

// Assess folder sizes for reorganization planning
{"path": "daily"}

// Find optimal location for new content types
```

### **File Management Planning**: Before moving or reorganizing
```json
// Understand current state before reorganization
{"path": "old-structure"}

// Plan new organization based on file distribution
{"path": "target-location"}
```

## Navigation Strategies

### **Top-Down Exploration**
1. Start with root directory (`{}`) to see overall structure
2. Identify interesting folders with high file counts
3. Navigate to specific folders for detailed exploration
4. Use findings to guide WriteMemory, SearchMemories decisions

### **Targeted Navigation**  
- Use specific paths when you know where to look
- Follow folder naming conventions discovered in exploration
- Navigate to timestamp-based folders (daily, monthly) for chronological content

### **Structure Discovery**
- Look for folder patterns that indicate knowledge organization schemes
- Identify heavily populated folders that might need subdivision
- Find sparse folders that might need consolidation

## Related Tools

- **WriteMemory**: Use ListMemories findings to choose optimal folder paths for new content
- **SearchMemories**: When ListMemories shows many files, use SearchMemories to find specific content within
- **ReadMemory**: Access specific files discovered through directory listing
- **MoveMemory**: Use directory structure insights to reorganize files between folders  
- **RecentActivity**: Combine with ListMemories to understand both content and activity patterns
- **MemoryStatus**: Get overall system statistics to complement detailed directory views

## Memory System Organization Insights

### Folder Naming Patterns
Based on directory exploration, you might discover patterns like:
- **Functional folders**: `research/`, `projects/`, `reference/`
- **Temporal folders**: `daily/`, `2024/`, `archive/`
- **Topic folders**: `ai/`, `programming/`, `philosophy/`
- **Status folders**: `active/`, `completed/`, `drafts/`

### File Distribution Analysis
Use ListMemories to identify:
- **Hotspot folders**: High file counts indicating active knowledge areas
- **Sparse areas**: Low file counts suggesting potential consolidation opportunities
- **Deep hierarchies**: Nested structures that might need flattening
- **Organization effectiveness**: Whether current structure supports knowledge discovery

## Troubleshooting

### **Error**: "Directory not found: [path]"
**Cause**: Specified path doesn't exist in memory system  
**Solution**: 
- Check path spelling and case sensitivity
- Use root exploration (`{}`) to discover correct folder names
- Remember paths are relative to memory root, not filesystem root
- Verify folder exists with parent directory listing first

### **Issue**: No folders or files shown
**Cause**: Directory exists but is empty or contains no .md files  
**Solution**:
- This is normal for empty directories
- Only .md files are shown (Maenifold knowledge files)
- Other file types are filtered out intentionally

### **Issue**: Path navigation confusion  
**Cause**: Unclear relationship between memory:// URIs and directory paths
**Solution**:
- ListMemories uses filesystem paths relative to memory root
- memory:// URIs map to these same paths (memory://projects = path: "projects")
- Forward slashes work for all path separators

### **Issue**: Large directories showing many files
**Cause**: Folder has grown to contain many knowledge files  
**Solution**:
- This indicates active knowledge area
- Consider using SearchMemories to find specific content instead of browsing
- Might indicate need for subfolder organization

### **Issue**: Difficulty finding optimal folder for new content
**Cause**: Complex directory structure unclear for content placement  
**Solution**:
- Start with root exploration to understand overall organization
- Look for similar content folders using SearchMemories
- Follow existing naming patterns discovered through exploration
- Create new folders when existing structure doesn't fit new content type

## Maenifold Integration

### Knowledge Graph Support
- Directory structure influences [[concept]] organization
- Related concepts often cluster in same folders
- Use ListMemories to understand concept locality patterns
- Folder organization can guide BuildContext exploration depth

### Memory System Navigation
ListMemories serves as the **structural overview tool** for your Maenifold:

1. **Spatial Understanding**: See where different types of knowledge live
2. **Growth Patterns**: Identify areas of active knowledge development  
3. **Organization Assessment**: Evaluate current folder structure effectiveness
4. **Content Planning**: Choose optimal locations for new knowledge creation
5. **System Health**: Monitor file distribution and identify organizational issues

### Workflow Integration
Use ListMemories as part of larger workflows:
- **Before WriteMemory**: Explore target folders to understand organization
- **With SearchMemories**: Browse structure first, then search within specific areas
- **Before MoveMemory**: Understand destination folder structure for better organization
- **With RecentActivity**: Combine structure view with activity patterns

## Ma Protocol Compliance

ListMemories follows Maenifold's Ma Protocol principles:

- **Single Responsibility**: Focused purely on directory structure exploration
- **No Magic**: Direct filesystem interaction with transparent path handling
- **Simple Parameters**: Single optional path parameter with clear defaults
- **Real Data**: Shows actual directory structure with real file counts and sizes
- **Minimal Complexity**: Static method, straightforward directory traversal
- **Memory System Focus**: Only shows .md files relevant to Maenifold knowledge

This tool provides essential **spatial navigation** within your Maenifold memory system, enabling informed decisions about content organization and knowledge structure while maintaining Ma Protocol simplicity and directness.

## Performance Characteristics

- **Fast Enumeration**: Lightweight directory and file listing operations
- **Filtered Results**: Only .md files counted and displayed for relevance
- **Minimal Memory Usage**: Streams directory contents without loading file data
- **Path Validation**: Quick existence checks before processing
- **Efficient Metadata**: File size calculation without content loading

Use ListMemories as your **memory system compass** - the essential tool for understanding where you are, where your knowledge lives, and where new content should go in your Maenifold ecosystem.