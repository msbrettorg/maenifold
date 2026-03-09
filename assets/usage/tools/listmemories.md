# ListMemories

Explores memory system folder structure with file counts and sizes for navigation and organization planning.

## Parameters

- `path` (string, optional): Directory path relative to memory root. Default: `"/"` (root)

## Returns

```
### Folders
📁 projects/ (15 files)
📁 research/ (8 files)

### Files
📄 ai-philosophy.md (12.3 KB)
   created: 2024-01-15
   last_accessed: 2024-01-20
   decay_weight: 0.85
📄 quick-notes.md (2.1 KB)
   created: 2024-01-18
   last_accessed: 2024-01-18
   decay_weight: 1.00
```

**Decay metadata:**
- `created`: File creation date (from frontmatter)
- `last_accessed`: Last read via ReadMemory (updates decay reference)
- `decay_weight`: Current decay weight (0.0-1.0). Higher = more recent/relevant

## Examples

```json
{}
```
Root directory - shows top-level folders and files.

```json
{
  "path": "projects"
}
```
Specific folder - explores subdirectory structure.

```json
{
  "path": "projects/ai-research/experiments"
}
```
Deep navigation - nested directory exploration.

## Usage Patterns

**Pre-write exploration:**
```json
{"path": "projects"}
// Check structure before WriteMemory with folder path
```

**Progressive navigation:**
```json
{}                                    // Root overview
{"path": "projects"}                  // Narrow focus
{"path": "projects/current"}          // Deep dive
```

**Organization assessment:**
- High file counts → active knowledge areas
- Sparse folders → consolidation candidates
- Deep nesting → potential flattening targets

## Constraints

- Shows only `.md` files (maenifold knowledge files)
- Paths relative to memory root: `"projects"` not `"/Users/you/maenifold/memory/projects"`
- Non-existent paths return error with directory list suggestion

## Decay Weight Interpretation

- **1.00**: Within grace period (7-14 days depending on folder)
- **0.50-0.99**: Active decay, content still relevant
- **0.25-0.49**: Aging content, may need refresh
- **< 0.25**: Stale content, consider updating or archiving

Files in `thinking/sequential/` have a 7-day grace period; all other memory has 14 days.

## Integration

- **WriteMemory**: Use findings to choose optimal folder paths
- **SearchMemories**: When folders contain many files, search within specific areas
- **ReadMemory**: Access specific files discovered through listing (also boosts last_accessed)
- **MoveMemory**: Understand destination structure before reorganization
- **RecentActivity**: Combine structure view with activity patterns
