# MemoryStatus

Returns system statistics: file counts, graph metrics, storage usage.

## Parameters

None required.

## Returns

```markdown
# Memory System Status

### Files
- Total files: 1234
- Total folders: 56
- Total size: 45.67 MB

### Graph Database
- Concepts: 789
- Relations: 2345
- Mentions: 6789
- Database size: 12.34 MB
```

## Example

```json
{}
```

## Use Cases

- **Health check before bulk operations**: Validate baseline before Sync or batch WriteMemory
- **Performance diagnosis**: Compare metrics over time to identify bottlenecks
- **Capacity planning**: Monitor storage growth and graph density trends
- **Post-operation validation**: Verify Sync rebuilt graph correctly

## Metrics Explained

**File Statistics**
- `Total files`: All .md files in memory system
- `Total folders`: Directory count including nested structures
- `Total size`: Combined storage in MB

**Graph Database**
- `Concepts`: Unique `[[WikiLink]]` nodes extracted from files
- `Relations`: Connections between concepts
- `Mentions`: Total concept occurrences across content
- `Database size`: SQLite storage in MB

**Health Indicators**
- High concept-to-file ratio → rich interconnectivity
- Balanced relations-to-concepts → healthy graph density
- Growing mentions → active knowledge reuse

## Troubleshooting

**"Not initialized (run sync first)"**
→ Database doesn't exist. Run Sync to build graph.

**Zero concepts despite files**
→ Files lack `[[WikiLinks]]` or Sync hasn't run. Add WikiLinks, then Sync.

**Large database vs. small file count**
→ Normal for rich graphs with many relationships and vectors.

**Performance degradation with growth**
→ Use metrics to identify bottleneck (file I/O vs database queries).

**Inconsistent metrics after changes**
→ Stale database. Run Sync to refresh.

## Integration

- **Sync**: Validate graph rebuild impact
- **SearchMemories**: Understand search performance context
- **ListMemories**: Complement with directory structure details
- **RecentActivity**: Cross-reference activity with health metrics
