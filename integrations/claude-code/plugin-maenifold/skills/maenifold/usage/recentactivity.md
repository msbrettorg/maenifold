# RecentActivity

Monitors system activity with time-based filtering for memory files and thinking sessions.

## Parameters

- `limit` (int, optional): Max results. Default: 10
- `filter` (string, optional): `"thinking"`, `"memory"`, or `"all"`. Default: `"all"`
- `timespan` (TimeSpan, optional): Format: `"[days.]hours:minutes:seconds"`. Must be positive.
- `includeContent` (bool, optional): Full content vs headers only. Default: false

## Returns

Chronological activity list with timestamps, types, session status, and snippets.

### Sequential Thinking
```
session-abc123 (sequential)
  Modified: 2024-08-31 14:30
  Thoughts: 7, Status: active
  First: "Analyzing core architecture..."
```

### Workflows
```
workflow-xyz789 (workflow)
  Modified: 2024-08-31 15:45
  Steps: 4, Status: completed
  First: "Starting design thinking..."
```

### Memory Files
```
knowledge-document (memory)
  Modified: 2024-08-31 16:20
  Title: ML Implementation Notes
```

## Examples

### Recent Thinking Sessions
```json
{"limit": 15, "filter": "thinking"}
```

### Time-Based Activity
```json
{"limit": 25, "timespan": "48.00:00:00"}
```

### Combined Filtering
```json
{"limit": 10, "filter": "thinking", "timespan": "7.00:00:00"}
```

## TimeSpan Format

- `"01:30:00"` - 1.5 hours
- `"24.00:00:00"` - 1 day
- `"7.00:00:00"` - 7 days

## Constraints

- **Sync required**: Run `Sync` first to create database
- **Positive timespan**: Negative values rejected
- **Snippet length**: `MAENIFOLD_SNIPPET_LENGTH` env var (default: 1000)

## Status Values

- `active`: In progress, can continue
- `completed`: Finished successfully
- `cancelled`: Explicitly cancelled
- `abandoned`: Session timed out after prolonged inactivity

## Integration

- **ReadMemory**: Use session IDs from results for full content
- **SequentialThinking/Workflow**: Sessions tracked with status
- **Sync**: Must run first to create indices

## Errors

**"No database found"**: Run `Sync` first
**"timespan must be positive"**: Use `"24.00:00:00"` format
**"No recent activity found"**: Expand timespan or use `"all"` filter
