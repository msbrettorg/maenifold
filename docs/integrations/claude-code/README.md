# Claude Code + Maenifold Integration

Restore semantic context from your knowledge graph on every Claude Code session with automatic task augmentation.

## Quick Install

```bash
cd ~/maenifold/docs/integrations/claude-code
./install.sh
```

## What It Does

The `graph_rag.sh` hook provides two Graph-RAG modes:

### 1. SessionStart Mode (FLARE-style proactive retrieval)

Every session start:
1. Queries recent activity from Maenifold
2. Extracts top [[concepts]] from your work
3. Builds graph context with relationships via `BuildContext` and `FindSimilarConcepts`
4. Injects ~5K tokens of graph-derived semantic knowledge into the new Claude session

### 2. Task Augmentation Mode (Concept-as-Protocol)

When you use the Task tool with embedded [[concepts]]:
1. Detects [[concepts]] in your task prompt
2. Auto-enriches with graph context via `BuildContext` and `FindSimilarConcepts`
3. Injects semantic relationships before task execution
4. Enables "concept-as-protocol" pattern: embed [[concepts]] for automatic context retrieval

**Example:**
```markdown
Create a feature for [[authentication]] using [[jwt]] and [[oauth]]
```

The hook auto-injects graph context for all three concepts before the task executes.

## Graph-RAG Patterns

This integration implements **Graph-RAG** and **FLARE** from `docs/search-and-scripting.md`:

- **Graph-RAG**: Concept-centric retrieval over the Maenifold graph, not just flat file text
- **FLARE**: Proactive, forward-looking retrieval at session start, before any user prompt
- **Concept-as-Protocol**: Embed [[concepts]] to trigger automatic graph context injection

## Manual Setup

```bash
# Copy hook
cp hooks/graph_rag.sh ~/.claude/hooks/
chmod +x ~/.claude/hooks/graph_rag.sh

# Update ~/.claude/settings.json
{
  "hooks": {
    "SessionStart": [{
      "matcher": "*",
      "hooks": [{
        "type": "command",
        "command": "~/.claude/hooks/graph_rag.sh session_start"
      }]
    }],
    "PreToolUse": [{
      "matcher": "Task",
      "hooks": [{
        "type": "command",
        "command": "~/.claude/hooks/graph_rag.sh task_augment"
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

Edit `~/.claude/hooks/graph_rag.sh`:

### SessionStart Mode
```bash
depth: 2                # Graph traversal depth - extended network (line 93)
maxEntities: 5          # Concepts per level (line 93)
timespan: "24.00:00:00" # Activity window (line 142)
max_concepts=5          # Max concepts to enrich (line 175)
char_limit_per_concept=1000  # Chars per concept (line 175)
semantic_threshold=0.5  # Min score for repo context (line 197)
```

### Task Augmentation Mode
```bash
max_concepts=5          # Max concepts from prompt (line 276)
char_limit_per_concept=800  # Chars per concept (line 276)
# Concepts are frequency-ranked (repeated concepts processed first)
```

### CLI Detection
The hook auto-detects Maenifold via:
1. `PATH` lookup (if `maenifold` is installed globally)
2. `$MAENIFOLD_ROOT` environment variable (points to repo root)

Set `MAENIFOLD_ROOT` for portable detection:
```bash
export MAENIFOLD_ROOT="$HOME/maenifold"
```

## Troubleshooting

```bash
# Test SessionStart mode manually
echo '{"session_id":"test","cwd":"/path/to/repo"}' | ~/.claude/hooks/graph_rag.sh session_start

# Test Task augmentation mode manually
echo '{"tool_name":"Task","tool_input":{"prompt":"Fix [[authentication]] bug"}}' | ~/.claude/hooks/graph_rag.sh task_augment

# Check Maenifold CLI
maenifold --tool MemoryStatus --payload '{}'
# or with MAENIFOLD_ROOT
$MAENIFOLD_ROOT/src/bin/Release/net9.0/maenifold --tool MemoryStatus --payload '{}'
```

## See Also

- [QUICK_START.md](QUICK_START.md) - 1-minute setup
- [hooks/](hooks/) - Additional hook examples
- `install.sh` - Automated installation
- `../../search-and-scripting.md` - RAG/search patterns this integration implements