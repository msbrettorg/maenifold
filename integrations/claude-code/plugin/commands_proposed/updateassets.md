---
name: ma:updateassets
description: Refreshes assets from packaged assets after maenifold upgrades
---

# UpdateAssets

Refreshes `~/maenifold/assets/` from packaged assets after maenifold upgrades.

## Parameters

- `dryRun` (bool, optional): Preview changes without modifying files. Default: `true`

## Returns

```json
{
  "filesAdded": ["workflows/new-workflow.json"],
  "filesUpdated": ["usage/tools/writemem.md"],
  "errors": []
}
```

## Example

```json
{
  "dryRun": false
}
```

## Constraints

- **Dry-run default**: Always previews first (`dryRun: true`)
- **Backup recommended**: Save custom assets before updating
- **Permission required**: Write access to `~/maenifold/assets/`

## Integration

- **GetConfig**: Check asset paths before update
- **AssetManager**: Source of packaged assets
