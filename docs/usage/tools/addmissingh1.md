# AddMissingH1

One-time [[migration]] tool that prepends H1 titles (`# {title}`) to legacy [[memory]] files that lack a top-level H1 heading. Ensures all memory files follow consistent [[markdown]] structure for proper rendering and [[knowledge-graph]] integration. Safe, idempotent operation with dry-run preview mode.

## When to Use This Tool

- **Legacy Memory Migration**: Update old memory files created before H1 requirement
- **Consistency Enforcement**: Ensure all memory files have proper heading structure
- **Rendering Fixes**: Fix display issues caused by missing top-level headings
- **Pre-Export Cleanup**: Prepare memory files for export to other [[markdown]] tools
- **New Installation**: Migrate memories from older Maenifold versions
- **Quality Audit**: Identify files missing proper markdown structure

## Key Features

- **Dry Run by Default**: Preview changes without modifying files (safety first)
- **Idempotent Operation**: Re-running after migration makes no further changes
- **Thinking Session Protection**: Automatically skips [[sequential-thinking]] and [[workflow]] files
- **Title Derivation**: Uses frontmatter title or derives from filename
- **Folder Scoping**: Limit operation to specific memory subfolders
- **Batch Limiting**: Process files in controlled batches to prevent accidents
- **Modified Timestamp**: Updates frontmatter modified field to track changes
- **Safety Exclusions**: Skips .git directories and non-markdown files

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| dryRun | bool | No | Preview changes without modifying files (default: true, ALWAYS START HERE) | true |
| limit | int | No | Maximum files to modify when dryRun=false (default: 100, safety limit) | 50 |
| folder | string | No | Optional subfolder under memory root to restrict scope | "research" |

## Usage Examples

### Preview All Missing H1 Files (Default)
```json
{
  "dryRun": true
}
```
Scans entire memory directory and shows which files would be modified. NO CHANGES MADE.

### Preview Specific Folder
```json
{
  "dryRun": true,
  "folder": "research/ai"
}
```
Shows only files in research/ai subfolder that need H1 headers added.

### Apply Migration (Small Batch)
```json
{
  "dryRun": false,
  "limit": 10
}
```
⚠️ Modifies up to 10 files. ONLY run after reviewing dry run output.

### Apply Migration (Larger Batch)
```json
{
  "dryRun": false,
  "limit": 100
}
```
Processes up to 100 files. Use after testing with smaller batch first.

### Scoped Migration
```json
{
  "dryRun": false,
  "limit": 50,
  "folder": "research"
}
```
Migrates up to 50 files in research subfolder only.

## How It Works

### Detection Logic
1. Scans all .md files in scope (excluding .git directories)
2. Skips files in "thinking" directories (sequential and workflow sessions)
3. Reads file content looking for first non-empty line
4. Identifies files where first content line doesn't start with `# `

### Title Derivation
1. **Frontmatter Title**: If file has `title:` in frontmatter, uses that
2. **Filename Fallback**: Converts filename to title case (replaces `-` and `_` with spaces)

Example: `machine-learning-notes.md` → `# Machine Learning Notes`

### H1 Insertion
Prepends `# {title}\n\n` before existing content, preserving frontmatter and all content.

### Metadata Update
Updates frontmatter `modified:` timestamp to ISO 8601 UTC format.

## Common Patterns

### Standard Migration Workflow
1. **Analyze**: Run with dryRun=true to see scope
2. **Test**: Apply to small folder first with low limit
3. **Verify**: Check modified files look correct
4. **Scale**: Increase limit and process remaining files
5. **Sync**: Run Sync tool to rebuild [[knowledge-graph]]

### Incremental Migration
Migrate in batches to maintain control:
```bash
# Round 1: Test with 10 files
AddMissingH1 dryRun=false limit=10

# Round 2: Expand to 50 files
AddMissingH1 dryRun=false limit=50

# Round 3: Process remaining files
AddMissingH1 dryRun=false limit=100
```

### Folder-by-Folder Migration
Process specific folders independently:
```bash
# Migrate research files
AddMissingH1 dryRun=false limit=100 folder="research"

# Migrate projects separately
AddMissingH1 dryRun=false limit=100 folder="projects"
```

## Related Tools

- **Sync**: Run after migration to rebuild [[knowledge-graph]] indices
- **MemoryStatus**: Check overall memory file statistics before/after migration
- **SearchMemories**: Find specific files to verify H1 headers added correctly
- **ReadMemory**: Read migrated files to verify structure

## Troubleshooting

### Error: "Folder must be inside memory root"
**Cause**: Provided folder parameter is outside memory directory structure
**Solution**: Use relative paths under memory root, e.g., "research" not "/absolute/path/research"

### Error: "Folder not found"
**Cause**: Specified folder doesn't exist
**Solution**: Verify folder name spelling and existence with ListMemories tool

### Result: "Found 0 files missing a top-level H1"
**Cause**: All files already have H1 headers (idempotent - this is success!)
**Solution**: No action needed - files are already in correct format

### Files Modified But Graph Unchanged
**Cause**: Forgot to run Sync after migration
**Solution**: Run Sync tool to rebuild [[knowledge-graph]] with updated file structure

### Some Files Skipped
**Cause**: Files in "thinking" directories are automatically excluded for safety
**Solution**: This is expected behavior - thinking session files have different structure

### Frontmatter Title vs Filename Title
**Cause**: Tool uses frontmatter title if present, otherwise derives from filename
**Solution**: This is correct behavior - frontmatter title takes precedence

## Output Structure

### Dry Run Output
```
Scan complete. Found 23 files missing a top-level H1.
  • research/machine-learning/transformers.md
  • research/machine-learning/attention-mechanisms.md
  • projects/2024/ai-infrastructure.md
  • projects/2024/deployment-strategy.md
  … and 19 more

Run with dryRun=false to apply changes. Suggested next step: Sync
```

### Actual Run Output
```
  ✓ Updated: research/machine-learning/transformers.md
  ✓ Updated: research/machine-learning/attention-mechanisms.md
  ✓ Updated: projects/2024/ai-infrastructure.md

Migration complete. Updated 3 file(s). Errors: 0. Remaining without H1: 20.

Run 'Sync' to rebuild the knowledge graph.
```

### Error Output
```
  ✓ Updated: research/file-1.md
  ✗ Error: research/file-2.md — File is locked
  ✓ Updated: research/file-3.md

Migration complete. Updated 2 file(s). Errors: 1. Remaining without H1: 18.
```

## Before and After Examples

### Before Migration
```markdown
---
title: Machine Learning Fundamentals
created: 2024-01-15
---

This document covers [[neural-networks]] and [[deep-learning]] concepts.

## Key Topics
Understanding [[backpropagation]] is essential.
```

### After Migration
```markdown
---
title: Machine Learning Fundamentals
created: 2024-01-15
modified: 2024-10-24T18:30:00Z
---

# Machine Learning Fundamentals

This document covers [[neural-networks]] and [[deep-learning]] concepts.

## Key Topics
Understanding [[backpropagation]] is essential.
```

## Safety Guarantees

### What is Protected
- **Thinking Sessions**: Files in `thinking/sequential/` and `thinking/workflow/` are never modified
- **Git Directories**: Files in `.git/` folders are excluded from scanning
- **Frontmatter**: Preserved exactly as-is except for `modified:` timestamp update
- **Content**: Existing content unchanged except for H1 prepending
- **WikiLinks**: All [[concept]] links preserved perfectly

### What is Modified
- **H1 Header**: Adds `# {title}` at top of content (after frontmatter)
- **Modified Timestamp**: Updates or adds `modified:` field in frontmatter
- **File Write**: Writes modified content back to same file path

### Idempotency
Running multiple times is safe - files already having H1 headers are skipped. No duplicate headers created.

## Example Migration Session

### Step 1: Check Scope
```json
{
  "dryRun": true
}
```
Output: `Found 147 files missing a top-level H1`

### Step 2: Test on Subset
```json
{
  "dryRun": false,
  "limit": 5
}
```
Output: `Updated 5 file(s). Errors: 0. Remaining without H1: 142`

### Step 3: Verify Results
Use ReadMemory to check a few migrated files - confirm H1 headers look correct.

### Step 4: Scale Up
```json
{
  "dryRun": false,
  "limit": 50
}
```
Output: `Updated 50 file(s). Errors: 0. Remaining without H1: 92`

### Step 5: Complete Migration
```json
{
  "dryRun": false,
  "limit": 100
}
```
Output: `Updated 92 file(s). Errors: 0. Remaining without H1: 0`

### Step 6: Rebuild Graph
```json
{
  "tool": "Sync"
}
```
Rebuilds [[knowledge-graph]] with updated file structure.

## When NOT to Use This Tool

- **Modern Installations**: New memory files already have H1 headers via WriteMemory
- **Custom Structured Files**: Files with intentional non-H1 structure (rare)
- **Thinking Sessions**: Tool automatically skips these anyway
- **Non-Memory Markdown**: Tool only processes files under memory root

## Ma Protocol Compliance

AddMissingH1 follows Maenifold's Ma Protocol principles:
- **Safety First**: Dry run by default, batch limiting, thinking session protection
- **Idempotent**: Safe to run multiple times without creating duplicates
- **Transparent**: Shows exactly which files will be modified before changing anything
- **Real Files**: Modifies actual source files, not database abstractions
- **Minimal Scope**: Single responsibility - adds missing H1 headers, nothing more
- **No Magic**: Simple file scanning and text prepending with clear logic

This tool represents Ma Protocol's emphasis on **gentle correction** - fixing legacy structure issues without disrupting existing content or forcing unnecessary complexity.
