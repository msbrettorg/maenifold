# Claude Code + Maenifold Quick Start

## 1-Minute Setup

```bash
# Run the installer
cd ~/maenifold/docs/integrations/claude-code
./install.sh

# That's it! Start a new Claude Code session
```

## What You Get

### SessionStart Hook
Every Claude Code session now starts with:
- 🧠 Knowledge graph context restoration
- 📊 Top concepts from recent work
- 🔗 Semantic relationships with weights
- 📝 Actual content from relevant files
- ⚡ ~5K tokens of intelligent context

### Task Augmentation Hook
When using Task tool with [[concepts]]:
- 🎯 Auto-detects [[concepts]] in prompts
- 📊 Enriches with graph context automatically
- 🔗 Adds semantic relationships
- ⚡ "Concept-as-protocol" pattern enabled

## How It Works

### SessionStart Mode
```
Session Start → Query Graph → Extract Top Concepts → Build Context → Inject Knowledge
```

### Task Augmentation Mode
```
Task Prompt → Extract [[concepts]] → Build Context → Augment Prompt → Execute Task
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

Edit `~/.claude/hooks/graph_rag.sh`:

```bash
# SessionStart mode

# Change time window (default: 24 hours)
timespan: "48.00:00:00"  # Line 142

# Adjust graph depth and entities
depth: 2, maxEntities: 5  # Line 93

# Adjust concept limits
enrich_concepts "$TOP_CONCEPTS" 10 1500  # Line 175 (max=10, chars=1500)

# Task augmentation mode

# Adjust concept limits for task prompts
enrich_concepts "$CONCEPTS" 8 1000  # Line 268 (max=8, chars=1000)
```

## Testing

1. Create some memory files with [[concepts]]:
```bash
~/maenifold/bin/osx-x64/maenifold --tool WriteMemory \
  --payload '{"title":"Test","content":"Testing [[authentication]] with [[jwt]]"}'
```

2. Start new Claude Code session
3. You should see the context restoration!

## Troubleshooting

### No context appears?
```bash
# Check Maenifold is accessible
maenifold --tool MemoryStatus --payload '{}'
# or
$MAENIFOLD_ROOT/src/bin/Release/net9.0/maenifold --tool MemoryStatus --payload '{}'

# Check hooks are registered
cat ~/.claude/settings.json | jq '.hooks.SessionStart, .hooks.PreToolUse'

# Test SessionStart hook manually
echo '{"session_id":"test","cwd":"."}' | ~/.claude/hooks/graph_rag.sh session_start

# Test Task augmentation hook manually
echo '{"tool_name":"Task","tool_input":{"prompt":"Fix [[auth]] bug"}}' | ~/.claude/hooks/graph_rag.sh task_augment
```

### Too much/little context?
Adjust `max_concepts` and `char_limit_per_concept` parameters in the hook:
- SessionStart: line 175
- Task augmentation: line 268

### Maenifold not detected?
Set the `MAENIFOLD_ROOT` environment variable:
```bash
export MAENIFOLD_ROOT="$HOME/maenifold"
# Add to ~/.bashrc or ~/.zshrc to persist
```

## Advanced Features

### Multi-Project Support
```bash
# Set MAENIFOLD_ROOT per project in .envrc (with direnv):
export MAENIFOLD_ROOT="$PWD"

# Or in graph_rag.sh, add project detection:
if [[ "$CWD" == *"project-a"* ]]; then
  MAENIFOLD_CLI="~/project-a/maenifold/src/bin/Release/net9.0/maenifold"
fi
```

### Concept-as-Protocol Pattern
```bash
# Embed [[concepts]] in Task prompts to trigger auto-enrichment:

# Instead of:
"Fix the authentication bug in the login module"

# Use:
"Fix [[authentication]] bug in [[login]] module with [[jwt]]"

# The hook auto-injects graph context for all three concepts!
```

### Conditional Loading
```bash
# Only load for specific projects
if [[ "$REPO_NAME" == "critical-project" ]]; then
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