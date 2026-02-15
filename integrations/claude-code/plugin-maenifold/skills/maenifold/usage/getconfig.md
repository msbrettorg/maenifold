# GetConfig

Returns maenifold system configuration for debugging and troubleshooting.

## Parameters

None - zero-parameter diagnostic tool.

## Returns

```
maenifold Configuration:
  Memory Path: ~/maenifold/memory
  Database: ~/maenifold/memory.db
  Debounce: 150ms
  Auto Sync: True
```

## Use Cases

- **Troubleshooting**: Verify paths when tools fail
- **Debugging**: Check auto-sync and debug flags
- **Environment validation**: Confirm config before major operations
- **Path verification**: Ensure directories resolve correctly

## Integration

Pairs with:
- **MemoryStatus**: System health metrics and file counts
- **GetHelp**: Tool documentation lookup
- **Sync**: Verify auto-sync before manual sync

## Environment Variables

- `MAENIFOLD_ROOT`: Override default `~/maenifold`
- `MAENIFOLD_DATABASE_PATH`: Custom database location
- `MAENIFOLD_DEBOUNCE_MS`: File watcher debounce (default: 150)
- `MAENIFOLD_AUTO_SYNC`: Enable/disable incremental sync (default: true)
