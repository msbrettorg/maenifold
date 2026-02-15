# DeleteMemory

Permanently removes memory files from disk with confirmation safety.

## Parameters

- `identifier` (string, required): Memory FILE identifier - URI (`memory://path/file`) or title
- `confirm` (bool, optional, default: false): MUST be `true` to execute deletion

## Returns

```json
{
  "uri": "memory://experiments/test-workflow",
  "deleted": true,
  "message": "Memory file deleted successfully"
}
```

## Example

```json
{
  "identifier": "Obsolete Research Notes",
  "confirm": true
}
```

## Constraints

- **Irreversible**: No recovery mechanism - file deleted immediately from disk
- **Confirmation required**: Operation fails if `confirm != true`
- **Graph lag**: Concepts removed from graph on next `Sync`, not immediately
- **Orphan concepts**: Deleting sole source of `[[WikiLinks]]` creates orphans

## Integration

- **ReadMemory**: Verify content before deletion
- **ExtractConceptsFromFile**: Check which `[[WikiLinks]]` will be affected
- **BuildContext**: Assess relationship impact before deletion
- **Sync**: Run after deletion to clean orphaned concepts from graph
- **SearchMemories**: Find files to delete by pattern

## Common Patterns

**Pre-deletion verification:**
```
1. ReadMemory → confirm correct file
2. ExtractConceptsFromFile → assess concept impact
3. BuildContext → check relationship dependencies
4. DeleteMemory with confirm=true
5. Sync → clean graph
```

**Cleanup workflow:**
```json
{
  "identifier": "memory://thinking/sequential/abandoned-session-12345",
  "confirm": true
}
```

## Errors

- `"Must set confirm=true to delete"` → Add `"confirm": true` to payload
- `"Memory file not found"` → Verify identifier with SearchMemories or ReadMemory
