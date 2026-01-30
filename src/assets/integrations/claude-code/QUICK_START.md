# Claude Code + Maenifold Quick Start

## 1-Minute Setup

```bash
# Copy the hook
cp ~/maenifold/assets/integrations/claude-code/hooks/session_start.sh ~/.claude/hooks/
chmod +x ~/.claude/hooks/session_start.sh

# Add to ~/.claude/settings.json:
# {
#   "hooks": {
#     "SessionStart": [{
#       "matcher": "*",
#       "hooks": [{ "type": "command", "command": "~/.claude/hooks/session_start.sh" }]
#     }]
#   }
# }

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

Edit `~/.claude/hooks/session_start.sh` (top of file):

```bash
# Graph traversal (default: depth 2, 10 entities, no content)
GRAPH_DEPTH=2              # How many hops (1-3)
MAX_ENTITIES=10            # Related concepts per hop (3-20)
INCLUDE_CONTENT=false      # Show content previews? (true/false)

# Token budget (default: 5000)
MAX_TOKENS=5000            # Approximate limit

# Concept selection (default: 10)
MAX_CONCEPTS=10            # Top concepts to process
```

**Why defaults work:**
- `INCLUDE_CONTENT=false` → See MORE of the graph (20+ concepts vs 3)
- `GRAPH_DEPTH=2` → See both direct + expanded relations
- `MAX_ENTITIES=10` → Broad context without overwhelming

## Testing

1. Create some memory files with `[[WikiLinks]]` (e.g., [[authentication]], [[database]]):
```bash
maenifold --tool WriteMemory \
  --payload '{"title":"Test","content":"Testing [[authentication]] with [[jwt]]"}'
```

2. Start new Claude Code session
3. You should see the context restoration!

## Troubleshooting

### No context appears?
```bash
# Check Maenifold is running
maenifold --tool MemoryStatus

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
# Note: If you have different Maenifold installations per project,
# ensure each is in PATH or use full paths here
if [[ "$CWD" == *"project-a"* ]]; then
  # Use project-specific Maenifold if needed
  export PATH="/path/to/project-a-maenifold:$PATH"
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

- Documentation: `~/maenifold/assets/integrations/claude-code/`
- Hooks: `~/.claude/hooks/`
- Settings: `~/.claude/settings.json`
- Claude Code Hooks: https://docs.claude.com/en/docs/claude-code/hooks

The knowledge graph remembers so Claude doesn't have to forget!