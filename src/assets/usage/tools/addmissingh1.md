# AddMissingH1

One-time migration tool prepending `# {title}` headers to legacy memory files lacking top-level H1. Ensures consistent markdown structure for rendering and graph integration. Idempotent with dry-run preview.

## Parameters

- `dryRun` (bool, optional): Preview changes without modifying files. Default: `true`. ALWAYS START HERE.
- `limit` (int, optional): Maximum files to modify when `dryRun=false`. Default: `100`.
- `folder` (string, optional): Subfolder under memory root to restrict scope. Example: `"research"`
- `createBackups` (bool, optional): Create `.bak` backup files before modification. Default: `false`.

## Returns

```json
{
  "foundCount": 23,
  "updatedCount": 0,
  "errorCount": 0,
  "remainingCount": 23,
  "files": ["research/file1.md", "research/file2.md"],
  "message": "Run with dryRun=false to apply changes. Suggested next step: Sync"
}
```

## Example

```json
{
  "dryRun": false,
  "limit": 10,
  "folder": "research"
}
```

**Workflow:**
1. Dry run to preview: `{"dryRun": true}`
2. Test small batch: `{"dryRun": false, "limit": 5}`
3. Verify results with ReadMemory
4. Scale up: `{"dryRun": false, "limit": 100}`
5. Rebuild graph: Run Sync

## Constraints

- **Dry run default**: Safety first - explicit `dryRun=false` required for changes
- **Thinking sessions excluded**: Automatically skips `thinking/sequential/` and `thinking/workflow/` files
- **Idempotent**: Re-running after migration makes no changes - files with H1 skipped
- **Title derivation**: Uses frontmatter `title:` or derives from filename
- **Metadata update**: Sets frontmatter `modified:` to ISO 8601 UTC timestamp

## Integration

- **Sync**: REQUIRED after migration to rebuild graph indices
- **MemoryStatus**: Check file statistics before/after
- **ReadMemory**: Verify migrated files
- **ListMemories**: Explore folder structure for scoping
