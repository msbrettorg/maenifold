---
layout: ../../../layouts/DocsLayout.astro
---

# Sync

Synchronize [[WikiLink]] concepts from memory files into the knowledge graph database, building searchable concept relationships and enabling graph-based operations. This tool extracts all [[concepts]] from your memory files, analyzes their co-occurrence patterns, and constructs the SQLite-based knowledge graph that powers BuildContext, Visualize, and enhanced SearchMemories functionality.

## When to Use This Tool

- After creating or editing multiple memory files to refresh the knowledge graph
- Before using BuildContext or Visualize tools to ensure current data
- When SearchMemories results seem outdated or incomplete
- After bulk file operations or imports to rebuild concept relationships
- For system maintenance to clean up abandoned sessions and optimize graph structure  
- When concepts appear disconnected but should show relationships
- To rebuild the full-text search index for improved content discovery
- After file system changes outside of Maenifold tools

## Key Features

- **Full Graph Rebuild**: Completely reconstructs concept graph from all memory files
- **WikiLink Extraction**: Automatically finds all [[concept]] references in double brackets
- **Co-occurrence Analysis**: Builds weighted relationships between concepts appearing together
- **Session Cleanup**: Identifies and marks abandoned workflow/sequential thinking sessions
- **File Content Indexing**: Updates full-text search database for SearchMemories integration
- **Relationship Quantification**: Tracks how often concepts appear together across files
- **Status Tracking**: Maintains file status information (active, completed, abandoned)
- **Performance Optimized**: Single transaction rebuild for consistency and speed

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| (none) | - | - | Sync takes no parameters - operates on entire memory system | `{}` |

## Usage Examples

### Basic Graph Synchronization
```json
{}
```
Rebuilds the complete knowledge graph from all memory files, extracting concepts and building relationships.

### After Content Creation Session
```json
{}
```
Run after creating multiple memory files to ensure all new [[concepts]] are indexed and connected in the graph.

### System Maintenance
```json
{}
```
Performs comprehensive sync including session cleanup, relationship analysis, and search index updates.

## Sync Process Details

### Phase 1: Session Cleanup
- Scans all workflow and sequential thinking files
- Identifies sessions active longer than abandonment threshold (default: 30 minutes)
- Marks stale sessions as "abandoned" with automatic timestamp
- Prevents zombie sessions from cluttering active session lists

### Phase 2: Concept Extraction  
- Processes all .md files in memory directory recursively
- Extracts [[WikiLink]] concepts using regex pattern matching
- Normalizes concepts (lowercase, spaces to hyphens) for consistency
- Counts occurrence frequency for each concept per file

### Phase 3: Graph Construction
- Creates concept nodes in SQLite database with first-seen timestamps
- Records concept mentions linked to source files with occurrence counts
- Builds co-occurrence edges between concepts appearing in same files
- Maintains source file lists for each concept relationship

### Phase 4: Content Indexing
- Stores full file content with titles and metadata
- Updates FTS5 (Full-Text Search) index for SearchMemories queries
- Preserves file status information (active, completed, abandoned)
- Maintains filesystem timestamps for change detection

## Database Schema

### Core Tables
- **concepts**: Primary concept registry with first-seen timestamps
- **concept_mentions**: Links concepts to files with occurrence counts  
- **concept_graph**: Weighted edges between co-occurring concepts
- **file_content**: Full text storage with metadata and search indexing
- **file_search**: FTS5 virtual table for fast content queries

### Graph Relationships
- Concepts connected by appearing in same files
- Edge weights based on co-occurrence frequency
- Bidirectional relationships (A-B equals B-A)
- Source file provenance for each relationship

## Common Patterns

### After Bulk Content Creation
Run Sync after creating multiple related memory files to ensure all new concept relationships are properly indexed and searchable.

### Before Graph Operations
Always sync before using BuildContext or Visualize to ensure you're working with current data reflecting recent file changes.

### Periodic Maintenance  
Run Sync periodically as part of system maintenance to clean up abandoned sessions and optimize graph structure.

### After External Changes
If memory files are modified outside Maenifold tools, run Sync to ensure graph database reflects current file state.

### Performance Optimization
Sync rebuilds indexes and optimizes database structure, improving performance of subsequent graph operations.

## Related Tools

- **BuildContext**: Uses Sync'd graph data to traverse concept relationships and build contextual information
- **Visualize**: Generates Mermaid diagrams from graph relationships created by Sync
- **SearchMemories**: Benefits from FTS index and concept graph updated by Sync
- **WriteMemory/EditMemory**: Create content that Sync processes into knowledge graph
- **SequentialThinking/Workflow**: Generate session files that Sync monitors for cleanup

## Performance Considerations

### Rebuild Strategy
Sync performs complete graph rebuild rather than incremental updates to ensure consistency and prevent drift between file system and database state.

### Transaction Management
All database operations occur within single transaction to maintain ACID properties and enable rollback on errors.

### File Processing
- Processes all .md files recursively in memory directory
- Continues processing on individual file errors to maximize data recovery
- Silent error handling prevents MCP protocol corruption

### Database Optimization
- Creates optimized indexes for concept lookups and graph traversal
- Uses FTS5 for efficient content search capabilities
- Maintains foreign key constraints for data integrity

## Troubleshooting

### Error: "Database locked" during sync
**Cause**: Another Maenifold operation is accessing the database  
**Solution**: Wait for concurrent operations to complete, then retry sync

### Warning: "No concepts found in files"
**Cause**: Memory files lack [[WikiLink]] concepts in double brackets  
**Solution**: Ensure files contain [[concept]] references for graph connectivity

### Error: "Permission denied accessing memory directory"
**Cause**: File system permissions prevent reading memory files  
**Solution**: Check directory permissions and file access rights

### Performance: Sync taking too long
**Cause**: Large number of files or complex concept relationships  
**Solution**: Normal for large knowledge bases; sync runs in background and reports progress

### Warning: "Abandoned sessions detected"  
**Cause**: Workflow or sequential thinking sessions left active too long  
**Solution**: Normal cleanup behavior; abandoned sessions marked automatically

### Error: "Failed to parse YAML frontmatter"
**Cause**: Malformed frontmatter in memory files  
**Solution**: Check file formatting; Sync continues processing other files

## Knowledge Graph Architecture

### Concept Normalization
All [[concepts]] are normalized to lowercase with spaces converted to hyphens for consistent storage and lookup.

### Relationship Weight Calculation
Co-occurrence strength determined by number of files containing both concepts, enabling relevance-based graph traversal.

### File Provenance
Every concept relationship maintains list of source files, providing traceability and context for graph connections.

### Temporal Information
Concepts track first-seen timestamps, and files maintain creation/modification times for temporal analysis.

## Ma Protocol Compliance

Sync follows Maenifold's Ma Protocol principles:
- **Simplicity**: Single responsibility for graph synchronization  
- **No Magic**: Direct SQLite operations with transparent schema
- **Minimal Complexity**: Static methods, clear database transactions
- **Real Testing**: End-to-end database and file system validation
- **Performance Focus**: Optimized for large knowledge bases with efficient indexing

This tool maintains the knowledge graph foundation that enables Maenifold's graph-augmented retrieval and relationship discovery capabilities.