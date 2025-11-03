# RecentActivity

Track recent activity across your Maenifold knowledge system with time-based filtering and session monitoring. RecentActivity provides a comprehensive view of recent memory files, thinking sessions, and workflow progress, enabling effective progress monitoring and work discovery patterns.

## When to Use This Tool

- **Progress Monitoring**: Track recent work across Sequential Thinking sessions and Workflows
- **Session Discovery**: Find active or recent thinking sessions to resume work
- **Memory Activity Tracking**: See recent memory file creation and modification patterns
- **Work Context Recovery**: Rediscover what you were working on after a break
- **Activity Filtering**: Focus on specific types of work (thinking vs memory operations)
- **Time-Based Queries**: Find work from specific time periods (last 24 hours, past week)
- **Session Status Monitoring**: Check completion status of workflows and thinking sessions
- **Knowledge Base Growth**: Monitor how your memory system is expanding over time

## Key Features

- **Dual Activity Tracking**: Monitors both memory file operations and thinking session activity
- **Time-Based Filtering**: Configurable timespan filtering with positive duration validation
- **Activity Type Filtering**: Separate "thinking", "memory", or view "all" activity types
- **Session Status Tracking**: Shows "active", "completed", or "cancelled" status for thinking sessions
- **Smart Content Extraction**: Automatically extracts meaningful snippets from content
- **Progress Indicators**: Displays thought counts for Sequential sessions, step counts for Workflows
- **Hierarchical Organization**: Groups activity by session ID with detailed progress information
- **Snippet Configuration**: Configurable snippet length (default 1000 chars, via MAENIFOLD_SNIPPET_LENGTH)

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| limit | int | No | Maximum results to return (default 10, negative values become 0) | 20 |
| filter | string | No | Activity type filter: "thinking", "memory", or "all" (default "all") | "thinking" |
| timespan | TimeSpan | No | Time period to filter by - MUST be positive duration | "24.00:00:00" |

## Usage Examples

### Basic Recent Activity Query
```json
{
  "limit": 10
}
```
Shows the 10 most recent activities across all types (thinking sessions, memory files).

### Focus on Recent Thinking Sessions  
```json
{
  "limit": 15,
  "filter": "thinking"
}
```
Shows recent Sequential Thinking sessions and Workflows, excluding memory file operations.

### Time-Based Activity Search
```json
{
  "limit": 25,
  "timespan": "48.00:00:00"
}
```
Shows activities from the last 48 hours, useful for weekly progress reviews.

### Memory-Only Activity Filter
```json
{
  "filter": "memory",
  "limit": 20
}
```
Shows only memory file creation and modifications, excluding thinking sessions.

### Combined Filtering
```json
{
  "limit": 10,
  "filter": "thinking", 
  "timespan": "7.00:00:00"
}
```
Shows thinking sessions from the last 7 days for focused session discovery.

## Activity Types and Display Formats

### Sequential Thinking Sessions
```
**session-abc123** (sequential)
  Modified: 2024-08-31 14:30
  Thoughts: 7
  Status: active
  First: "Analyzing the core architecture patterns..."
  Last: "Need to explore the dependency relationships..."
```

### Workflow Sessions  
```
**workflow-xyz789** (workflow)
  Modified: 2024-08-31 15:45
  Steps: 4
  Status: completed
  First: "Starting design thinking process for..."
  Current: "Final validation shows successful implementation..."
```

### Memory Files
```
**knowledge-document** (memory)
  Modified: 2024-08-31 16:20
  Title: Machine Learning Implementation Notes
  Sections: 3
  First: "Implementing GraphRAG requires understanding..."
  Last: "Key insights from testing phase include..."
```

## Common Patterns

### Progress Monitoring Workflow
Use RecentActivity to track active sessions, then use ReadMemory to dive deeper into specific sessions for continuation or review.

### Session Recovery Pattern
After a break, run RecentActivity with "thinking" filter to find your most recent active sessions and resume where you left off.

### Knowledge Base Growth Tracking
Run periodic RecentActivity queries with memory filter to see how your knowledge base is expanding over time.

### Time-Based Work Reviews
Use timespan filtering for daily/weekly reviews: `{"timespan": "24.00:00:00"}` for daily progress, `{"timespan": "168.00:00:00"}` for weekly summaries.

### Active Session Discovery
Filter by "thinking" to find sessions with "active" status that need completion or have pending thoughts.

## Related Tools

- **SequentialThinking**: Creates thinking sessions that appear in RecentActivity with thought counts and status tracking
- **Workflow**: Creates workflow sessions that RecentActivity monitors for step progress and completion status
- **ReadMemory**: Use session IDs from RecentActivity to read full session content for continuation
- **SearchMemories**: Find related knowledge when RecentActivity reveals interesting session topics
- **WriteMemory**: Memory creation operations that RecentActivity tracks for knowledge base growth monitoring
- **Sync**: Must be run first to index files - RecentActivity requires the database created by Sync

## Status Values Explained

- **active**: Session is in progress and can be continued with more thoughts/responses
- **completed**: Session finished successfully through normal completion
- **cancelled**: Session was explicitly cancelled by user before completion  
- **abandoned**: Session was left inactive (not currently used by RecentActivity)

## Troubleshooting

### Error: "No database found. Run the `Sync` command first"
**Cause**: RecentActivity requires the file_content database table created by the Sync command  
**Solution**: Run the `Sync` tool first to index your memory files and create the necessary database structure

### Error: "timespan parameter must be positive"
**Cause**: Provided timespan is negative (e.g., "-24.00:00:00")  
**Solution**: Use positive timespan values like "24.00:00:00" for 24 hours or "7.00:00:00" for 7 days

### Result: "No recent activity found"
**Cause**: No files match your filter criteria or timespan constraints  
**Solution**: Expand timespan, change filter from "thinking"/"memory" to "all", or increase limit parameter

### Empty or Limited Results
**Cause**: Very restrictive filtering or short timespan  
**Solution**: Use broader parameters: increase limit, expand timespan, or use "all" filter

### Session Content Appears Truncated
**Cause**: Content snippets are limited by MAENIFOLD_SNIPPET_LENGTH configuration (default 1000 chars)  
**Solution**: This is expected behavior for readability - use ReadMemory to get full session content

### Status Shows as "active" for Old Sessions  
**Cause**: Sessions not explicitly completed or cancelled maintain "active" status  
**Solution**: Complete old sessions using SequentialThinking or Workflow tools with appropriate status parameters

## Activity Filtering Logic

### "thinking" Filter
- Includes: Sequential Thinking sessions (`memory://thinking/sequential/`)
- Includes: Workflow sessions (`memory://thinking/workflow/`)  
- Excludes: Regular memory files

### "memory" Filter  
- Includes: All memory files outside thinking directory
- Excludes: Sequential Thinking and Workflow sessions
- Shows: Knowledge files, documentation, research notes

### "all" Filter (Default)
- Includes: Everything - thinking sessions and memory files
- Provides: Complete activity overview across your Maenifold system

## Time Format Specifications

TimeSpan format follows .NET standard: `[days.]hours:minutes:seconds[.fractional_seconds]`

**Common Examples**:
- `"01:30:00"` - 1 hour 30 minutes
- `"24.00:00:00"` - 24 hours (1 day)
- `"7.00:00:00"` - 7 days (1 week)  
- `"30.00:00:00"` - 30 days (approximately 1 month)

## Performance Considerations

- **Database Dependency**: Requires Sync to create database indices for fast queries
- **Snippet Length**: Configurable via MAENIFOLD_SNIPPET_LENGTH environment variable
- **Query Optimization**: Uses indexed last_modified timestamps for efficient time-based filtering
- **Memory Usage**: Content snippets are truncated to prevent excessive memory usage with large result sets

## Integration with Maenifold Ecosystem

RecentActivity serves as a **discovery and monitoring hub** for your Maenifold system:

1. **Knowledge Graph Integration**: Files tracked by RecentActivity contain [[concepts]] that build your knowledge graph
2. **Session Continuity**: Provides session IDs needed to continue Sequential Thinking and Workflow sessions  
3. **Memory System Overview**: Shows how WriteMemory, EditMemory, and other memory operations are growing your knowledge base
4. **Quality Assurance**: Helps identify abandoned sessions that need completion or cleanup
5. **Work Pattern Analysis**: Reveals your thinking and knowledge creation patterns over time

Use RecentActivity as your **starting point** for understanding what's happening in your Maenifold system, then use other tools to dive deeper into specific sessions, memory files, or knowledge areas.

## Ma Protocol Compliance

RecentActivity follows Maenifold's Ma Protocol principles:
- **Single Responsibility**: Focused purely on activity discovery and monitoring
- **No Magic**: Direct SQLite queries with transparent filtering logic
- **Simple Parameters**: Clear, validated inputs with helpful error messages  
- **Real Data**: Shows actual file activity, modification times, and session states
- **Minimal Complexity**: Static methods, straightforward SQL queries, no caching complexity

This tool provides essential visibility into your Maenifold activity patterns while maintaining the simplicity and directness that characterizes the Ma Protocol approach.