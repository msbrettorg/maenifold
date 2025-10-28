---
layout: ../../../layouts/DocsLayout.astro
---

# UpdateAssets Tool

## Overview
Updates persistent assets from packaged assets on upgrades with dry-run capability.

## Purpose
Select when asset changes (workflows, documentation, models) need refreshing after maenifold upgrades.

## Key Features
- Dry-run mode to preview changes before applying
- Detailed summary of modifications
- Explicit refresh mechanism after deployment
- Error reporting if refresh encounters issues

## Parameters

### dryRun (optional)
- **Type**: boolean
- **Default**: true
- **Description**: Preview changes without modifying files. Set to false to apply updates.

## Return Value
Returns `UpdateAssetsResult` with summary of added/updated files.

## Usage Examples

### CLI

**Dry Run (Preview Changes)**
```bash
Maenifold --tool UpdateAssets --payload '{"dryRun":true}'
```

**Execute Update**
```bash
Maenifold --tool UpdateAssets --payload '{"dryRun":false}'
```

**Default (Dry Run)**
```bash
Maenifold --tool UpdateAssets --payload '{}'
```

### MCP

**Dry Run (Preview)**
```json
{
  "dryRun": true
}
```

**Execute Update**
```json
{
  "dryRun": false
}
```

## Common Patterns

### Safe Update Workflow
1. **Preview**: Run with `dryRun: true` to see what would change
2. **Review**: Check the summary of changes
3. **Apply**: Run with `dryRun: false` to update assets

### After maenifold Upgrade
```bash
# Check what assets need updating
Maenifold --tool UpdateAssets --payload '{"dryRun":true}'

# Apply updates
Maenifold --tool UpdateAssets --payload '{"dryRun":false}'
```

## Integration Points
- Integrates with `AssetManager` for asset initialization
- Works with workflows, documentation, and model assets
- Called after maenifold upgrades to refresh persistent assets

## Best Practices
1. **Always dry-run first**: Check what will change before applying
2. **Review changes**: Examine the summary before executing
3. **After upgrades**: Run after updating maenifold to get latest assets
4. **Version control**: Consider backing up custom assets before updating

## Troubleshooting

### No Changes Detected
- Assets may already be up to date
- Check that packaged assets exist in the installation

### Permission Errors
- Ensure write permissions to asset directories
- Check file locks on existing assets

### Partial Updates
- Review error messages in the result
- Some files may have updated while others failed
- Re-run to attempt failed updates

## Related Tools
- **GetConfig**: Check asset paths and configuration
- **MemoryStatus**: Verify asset counts after update

## RTM Compliance
- **UseStructuredContent**: true
- **Destructive**: false (dry-run), true (execute)
- **ReadOnly**: false
- **Title**: "Update Assets"
