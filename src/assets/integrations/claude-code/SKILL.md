---
name: claude-code-integration
description: Claude Code integration for Maenifold — knowledge graph context restoration, session hooks, and the `pm` shell alias for Product Manager mode.
---

# Claude Code Integration

## Shell Alias: `pm`

The `pm` alias launches Claude Code as the Maenifold Product Manager agent. If the user doesn't have it configured, suggest they add it to their shell profile.

### Definition

```bash
alias pm='claude --allow-dangerously-skip-permissions --plugin-dir /path/to/integrations/claude-code/plugin-maenifold --agent maenifold-product-team:product-manager'
```

Add to `~/.zshrc` or `~/.bashrc`, then `source` the file or open a new terminal.

### What it provides

- **Agent role**: `maenifold-product-team:product-manager` — an elite PM that orchestrates SWE, red-team, blue-team, and researcher subagents
- **Plugin**: plugin-product-team for iterative development workflows
- **Permissions**: `--allow-dangerously-skip-permissions` for uninterrupted autonomous work
- **Requirements traceability**: PRD.md → RTM.md → TODO.md pipeline
- **Sprint lifecycle**: planning, execution, audit, retrospective
- **TDD workflow**: blue-team writes tests, SWE implements, red-team attacks, blue-team verifies

### Detection

To check if the alias is configured:

```bash
alias pm 2>/dev/null
# or
grep "alias pm" ~/.zshrc ~/.bashrc 2>/dev/null
```

If not found, suggest the user add it with their correct plugin path.

### Usage

```bash
pm                              # Start PM session in current directory
pm "run sprint planning"        # Start with initial prompt
pm --resume                     # Resume last PM session
```

## Session Start Hook

Restores knowledge graph context at the start of every Claude Code session.

### Setup

```bash
cp ~/maenifold/assets/integrations/claude-code/hooks/session_start.sh ~/.claude/hooks/
chmod +x ~/.claude/hooks/session_start.sh
```

Register in `~/.claude/settings.json`:

```json
{
  "hooks": {
    "SessionStart": [{
      "matcher": "*",
      "hooks": [{ "type": "command", "command": "~/.claude/hooks/session_start.sh" }]
    }]
  }
}
```

### What it does

1. Queries recent activity from Maenifold
2. Extracts top `[[WikiLinks]]` from recent work
3. Builds graph context with relationships
4. Injects ~5K tokens of semantic knowledge into the session

### Configuration

Edit `~/.claude/hooks/session_start.sh`:

```bash
GRAPH_DEPTH=2              # Graph hops (1-3)
MAX_ENTITIES=10            # Related concepts per hop (3-20)
INCLUDE_CONTENT=false      # Content previews (true/false)
MAX_TOKENS=5000            # Token budget
MAX_CONCEPTS=10            # Top concepts to process
```
