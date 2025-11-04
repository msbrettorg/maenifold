# Claude Code + Maenifold Quick Start

## 1-Minute Setup

```bash
# Run the installer
cd ~/maenifold/docs/integrations/claude-code
./install.sh

# That's it! Start a new Claude Code session
```

## What You Get

Every Claude Code session now starts with:
- 🧠 Knowledge graph context restoration
- 📊 Top concepts from recent work
- 🔗 Semantic relationships with weights
- 📝 Actual content from relevant files
- ⚡ ~5K tokens of intelligent context

## How It Works

```
Session Start → Query Graph → Extract Concepts → Build Context → Inject Knowledge
```

## The Magic

Instead of starting cold, Claude sees:
```markdown
🧠 **Knowledge Graph Context Restoration**

### [[authentication]]
Context: JWT implementation with refresh tokens
Related: oauth (12 files), security (8 files)
Content: "Implementing RS256 token signing..."

### [[database-migration]]
Context: Schema versioning strategy
Related: postgresql (15 files), migrations (10 files)
Content: "Using Flyway for version control..."

[More concepts with actual context...]
```

## Customization

Edit `~/.claude/hooks/session_start.sh`:

```bash
# Adjust token budget (default: 5000)
MAX_TOKENS=3000

# Change time window (default: 24 hours)
timespan":"48.00:00:00"

# Filter concepts (add patterns to exclude)
grep -v '^concept' | grep -v '^test'

# Adjust graph depth (default: 1)
depth: 2, maxEntities: 5
```

## Testing

1. Create some memory files with [[concepts]]:
```bash
~/maenifold/bin/osx-x64/Maenifold --tool WriteMemory \
  --payload '{"title":"Test","content":"Testing [[authentication]] with [[jwt]]"}'
```

2. Start new Claude Code session
3. You should see the context restoration!

## Troubleshooting

### No context appears?
```bash
# Check Maenifold is running
~/maenifold/bin/osx-x64/Maenifold --tool MemoryStatus

# Check hook is registered
cat ~/.claude/settings.json | jq '.hooks.SessionStart'

# Test hook manually
echo '{"session_id":"test","cwd":"."}' | ~/.claude/hooks/session_start.sh
```

### Too much/little context?
Adjust MAX_TOKENS in the hook (line 32)

### Wrong Maenifold path?
Update MAENIFOLD_CLI in the hook (line 18)

## Advanced Features

### Multi-Project Support
```bash
# In session_start.sh, add project detection:
if [[ "$CWD" == *"project-a"* ]]; then
  MAENIFOLD_CLI="~/project-a/maenifold/bin/osx-x64/Maenifold"
fi
```

### Priority Concepts
```bash
# Prioritize certain patterns
grep -E '\[\[(auth|security|database)[^]]*\]\]' | head -5
```

### Conditional Loading
```bash
# Only load for specific projects
if [[ "$PROJECT_NAME" == "critical" ]]; then
  # Run full restoration
else
  exit 0  # Skip
fi
```

## Philosophy

> "The graph is the context window"

Traditional: Files → Text → Context
Maenifold: Knowledge Graph → Semantic Relations → Understanding

## Support

- Documentation: `~/maenifold/docs/integrations/claude-code/`
- Hooks: `~/.claude/hooks/`
- Settings: `~/.claude/settings.json`

The knowledge graph remembers so Claude doesn't have to forget!