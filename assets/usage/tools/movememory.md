# MoveMemory

Relocates/renames knowledge files in `~/maenifold/memory/` while preserving `[[WikiLinks]]` and updating metadata timestamps.

## Parameters

- `source` (string, required): Source FILE identifier - `memory://uri` or exact title
- `destination` (string, required): Destination FILE name or path. Examples: `"new-name"` or `"folder/new-name.md"`

## Returns

```json
{
  "moved": "memory://old/path → memory://new/path"
}
```

## Example

```json
{
  "source": "memory://research/ml-basics",
  "destination": "ai/deep-learning/neural-networks"
}
```

## Constraints

- **Destination must not exist**: Operation fails if target file exists (no overwrite)
- **Path escape prevention**: Destination must stay within memory root
- **Automatic slugification**: Filename normalized to lowercase-with-hyphens
- **Extension handling**: `.md` appended automatically if missing

## Integration

- **ReadMemory**: Verify source content before move
- **SearchMemories**: Find files for batch reorganization
- **ListMemories**: Explore folder structure for planning
- **EditMemory**: Update cross-references after reorganization
- **Sync**: Run after bulk moves to rebuild graph indices
