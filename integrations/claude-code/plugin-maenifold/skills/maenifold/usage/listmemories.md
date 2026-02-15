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
📄 quick-notes.md (2.1 KB)
```

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
Check structure before WriteMemory with folder path:
```json
{"path": "projects"}
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
- Paths relative to memory root: `"projects"` not `"~/maenifold/memory/projects"`
- Non-existent paths return error with directory list suggestion

## Integration

- **WriteMemory**: Use findings to choose optimal folder paths
- **SearchMemories**: When folders contain many files, search within specific areas
- **ReadMemory**: Access specific files discovered through listing
- **MoveMemory**: Understand destination structure before reorganization
- **RecentActivity**: Combine structure view with activity patterns
