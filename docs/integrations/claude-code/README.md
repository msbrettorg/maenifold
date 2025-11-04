# Claude Code + Maenifold Integration

Restore semantic context from your knowledge graph on every Claude Code session.

## Quick Install

```bash
cd ~/maenifold/docs/integrations/claude-code
./install.sh
```

## What It Does

Every session start:
1. Queries recent activity from Maenifold
2. Extracts top [[concepts]] from your work
3. Builds graph context with relationships
4. Injects ~5K tokens of semantic knowledge

## Manual Setup

```bash
# Copy hook
cp hooks/session_start.sh ~/.claude/hooks/
chmod +x ~/.claude/hooks/session_start.sh

# Update ~/.claude/settings.json
{
  "hooks": {
    "SessionStart": [{
      "matcher": "*",
      "hooks": [{
        "type": "command",
        "command": "~/.claude/hooks/session_start.sh"
      }]
    }]
  }
}
```


## Example Output

```markdown
🧠 **Knowledge Graph Context Restoration**

### [[authentication]]
Context: JWT implementation with refresh tokens
Related: oauth (12 files), security (8 files)
Content: "Implementing RS256 token signing..."

### [[database-migration]]
Context: Schema versioning strategy
Related: postgresql (15 files), migrations (10 files)
```

## Configuration

Edit `~/.claude/hooks/session_start.sh`:

```bash
MAX_TOKENS=5000         # Token budget
depth: 1                # Graph traversal depth
maxEntities: 3          # Concepts per level
timespan: "24.00:00:00" # Activity window
grep -v '^concept'      # Filter patterns
```

## Troubleshooting

```bash
# Test manually
echo '{"session_id":"test"}' | ~/.claude/hooks/session_start.sh

# Check Maenifold
~/maenifold/bin/osx-x64/Maenifold --tool MemoryStatus
```

## See Also

- [QUICK_START.md](QUICK_START.md) - 1-minute setup
- [hooks/](hooks/) - Additional hook examples
- `install.sh` - Automated installation