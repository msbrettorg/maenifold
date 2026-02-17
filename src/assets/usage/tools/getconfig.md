# GetConfig

Returns Maenifold system configuration for debugging and troubleshooting.

## Parameters

None - zero-parameter diagnostic tool.

## Returns

```
Maenifold Configuration:
  Memory Path: ~/maenifold/memory
  Database: ~/maenifold/memory.db
  Debounce: 150ms
  Auto Sync: True
  Decay Config:
    Grace Days (Sequential): 7
    Grace Days (Workflows): 14
    Grace Days (Default): 28
    Half-Life Days: 30
    Function: power-law
  Community Detection:
    Louvain Gamma: 1
    Sibling Min Shared Neighbors: 3
    Sibling Min Overlap: 0.4
    Sibling Max Results: 10
    Watcher Debounce: 2000ms
```

## Use Cases

- **Troubleshooting**: Verify paths when tools fail
- **Debugging**: Check auto-sync and debug flags
- **Environment validation**: Confirm config before major operations
- **Path verification**: Ensure directories resolve correctly
- **Decay tuning**: Verify grace periods and half-life before running sleep cycle
- **Community detection tuning**: Check Louvain resolution and sibling thresholds

## Integration

Pairs with:
- **MemoryStatus**: System health metrics and file counts
- **GetHelp**: Tool documentation lookup
- **Sync**: Verify auto-sync before manual sync

## Environment Variables

All configuration is read from environment variables at startup. Unset variables use the defaults shown below.

### Core

| Variable | Default | Description |
|----------|---------|-------------|
| `MAENIFOLD_ROOT` | `~/maenifold` | Root directory for all maenifold data |
| `MAENIFOLD_DATABASE_PATH` | `$MAENIFOLD_ROOT/memory.db` | Custom database file location |

### Sync & Watcher

| Variable | Default | Description |
|----------|---------|-------------|
| `MAENIFOLD_DEBOUNCE_MS` | `150` | File watcher debounce interval in milliseconds |
| `MAENIFOLD_AUTO_SYNC` | `true` | Enable/disable incremental sync on file changes |
| `MAENIFOLD_WATCHER_BUFFER` | `65536` | FileSystemWatcher internal buffer size in bytes |
| `MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY` | `40` | Run FTS5 OPTIMIZE after this many file changes |
| `MAENIFOLD_INCREMENTAL_VACUUM_MINUTES` | `720` | Auto-vacuum interval in minutes (default: 12 hours) |
| `MAENIFOLD_SYNC_LOGGING` | `true` | Enable logging of sync operations |

### Session Management

| Variable | Default | Description |
|----------|---------|-------------|
| `MAENIFOLD_SESSION_ABANDON_MINUTES` | `30` | Inactivity threshold for session abandonment detection |
| `MAENIFOLD_SESSION_CLEANUP` | `true` | Enable session cleanup during sync |

### Decay

| Variable | Default | Description |
|----------|---------|-------------|
| `MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL` | `7` | Grace period in days for `thinking/sequential/` content |
| `MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS` | `14` | Grace period in days for `thinking/workflows/` content |
| `MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT` | `28` | Grace period in days for all other memory |
| `MAENIFOLD_DECAY_HALF_LIFE_DAYS` | `30` | Half-life in days after grace period expires |
| `MAENIFOLD_DECAY_FUNCTION` | `power-law` | Decay function type: `power-law` or `exponential` |

### Community Detection

| Variable | Default | Description |
|----------|---------|-------------|
| `MAENIFOLD_LOUVAIN_GAMMA` | `1.0` | Louvain resolution parameter controlling community granularity |
| `MAENIFOLD_COMMUNITY_MIN_SHARED` | `3` | Minimum shared neighbors for community sibling inclusion |
| `MAENIFOLD_COMMUNITY_MIN_OVERLAP` | `0.4` | Minimum normalized overlap score for community siblings |
| `MAENIFOLD_COMMUNITY_MAX_SIBLINGS` | `10` | Maximum community siblings returned by BuildContext |
| `MAENIFOLD_COMMUNITY_DEBOUNCE_MS` | `2000` | DB watcher debounce for community recomputation in milliseconds |

### Diagnostics

| Variable | Default | Description |
|----------|---------|-------------|
| `MAENIFOLD_SNIPPET_LENGTH` | `1000` | RecentActivity snippet truncation length in characters |
| `MAENIFOLD_SQLITE_BUSY_TIMEOUT` | `5000` | SQLite busy timeout in milliseconds |
| `MAENIFOLD_EMBEDDING_LOGS` | `false` | Enable verbose embedding operation logging |
| `MAENIFOLD_VECTOR_LOGS` | `false` | Enable verbose vector search logging |
