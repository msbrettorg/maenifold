# Claude Code + Maenifold Integration

Restore semantic context from your knowledge graph on every Claude Code session.

## What It Does

Every session start:
1. Queries recent activity from Maenifold
2. Extracts top `[[WikiLinks]]` like [[authentication]], [[database-schema]] from your work
3. Builds graph context with relationships
4. Injects ~5K tokens of semantic knowledge

## Setup

### Shell Alias (Recommended)

Add a `pm` alias to your shell profile (`~/.zshrc` or `~/.bashrc`) to launch Claude Code as the Maenifold Product Manager:

```bash
alias pm='claude --allow-dangerously-skip-permissions --plugin-dir /path/to/ralph-loop --agent maenifold-product-team:product-manager'
```

This gives you a single command to start a fully-equipped PM session with:
- The `maenifold-product-team:product-manager` agent role
- Subagent orchestration (SWE, red-team, blue-team, researcher)
- TDD workflow with requirements traceability (PRD.md → RTM.md → TODO.md)
- Sprint lifecycle management

Usage:
```bash
pm                          # Start PM session in current directory
pm "run sprint planning"    # Start with an initial prompt
```

### Session Start Hook

Auto-restore knowledge graph context on every session:

```bash
# Copy hook
cp ~/maenifold/assets/integrations/claude-code/hooks/session_start.sh ~/.claude/hooks/
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

Edit `~/.claude/hooks/session_start.sh` (top of file):

```bash
GRAPH_DEPTH=2              # How many hops in the graph (1-3)
MAX_ENTITIES=10            # Max related concepts per hop (3-20)
INCLUDE_CONTENT=false      # Include content previews (true/false)
MAX_TOKENS=5000            # Approximate token budget
MAX_CONCEPTS=10            # Max top concepts to process
```

**Recommendations:**
- `INCLUDE_CONTENT=false` shows more graph structure (recommended)
- `INCLUDE_CONTENT=true` shows content snippets but covers less graph
- Higher `GRAPH_DEPTH` and `MAX_ENTITIES` = broader context, more tokens
- Adjust `MAX_TOKENS` to control total context size

## Troubleshooting

```bash
# Test manually
echo '{"session_id":"test"}' | ~/.claude/hooks/session_start.sh

# Check Maenifold
maenifold --tool MemoryStatus
```

## See Also

- [QUICK_START.md](QUICK_START.md) - 1-minute setup
- [hooks/](hooks/) - Additional hook examples
- [Claude Code Hooks Documentation](https://docs.claude.com/en/docs/claude-code/hooks) - Official hooks reference