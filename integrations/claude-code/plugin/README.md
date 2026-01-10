# maenifold Plugin for Claude Code

CLI integration for maenifold knowledge graph and reasoning infrastructure. This plugin exposes maenifold's full toolkit through slash commands and specialized agents.

## Overview

**maenifold** is a test-time adaptive reasoning infrastructure that enables:
- Persistent knowledge graphs with 384-dimensional vector embeddings
- 30+ structured workflows for systematic problem-solving
- Sequential thinking sessions with revision and branching
- Hybrid search (semantic + full-text) via Reciprocal Rank Fusion

This plugin bridges maenifold's CLI tools into Claude Code's command system.

## CLI Detection

The plugin automatically detects the maenifold binary in this order:
1. System PATH
2. `$MAENIFOLD_ROOT/src/bin/Release/net9.0/maenifold`
3. `$MAENIFOLD_ROOT/src/bin/Debug/net9.0/maenifold`

Set `MAENIFOLD_ROOT` environment variable to customize the installation path (default: `~/maenifold`).

## Commands

All commands use the `/ma:` prefix. The plugin provides 25+ commands organized by layer:

### Memory Layer

| Command | Description |
|---------|-------------|
| `/ma:write` | Create knowledge file with WikiLinks |
| `/ma:read` | Retrieve file by URI or title |
| `/ma:edit` | Modify existing file (append, prepend, find_replace) |
| `/ma:delete` | Remove file permanently |
| `/ma:move` | Relocate or rename file |
| `/ma:search` | Search with Hybrid/Semantic/FullText modes |
| `/ma:extract` | Extract concepts from file |
| `/ma:sync` | Rebuild knowledge graph |
| `/ma:list` | Explore folder structure |

### Graph Layer

| Command | Description |
|---------|-------------|
| `/ma:context` | Multi-hop concept traversal |
| `/ma:similar` | Semantic similarity search |
| `/ma:visualize` | Generate Mermaid diagrams |

### Reasoning Layer

| Command | Description |
|---------|-------------|
| `/ma:think` | Sequential thinking session |
| `/ma:workflow` | Orchestrate structured methodology |

### Session Layer

| Command | Description |
|---------|-------------|
| `/ma:recent` | Monitor system activity |
| `/ma:assume` | Declare and track assumptions |
| `/ma:status` | System statistics |
| `/ma:config` | View configuration |
| `/ma:help` | Tool documentation |
| `/ma:adopt` | Load role/color/perspective |

### Asset Layer

| Command | Description |
|---------|-------------|
| `/ma:assets` | Browse available assets |
| `/ma:resource` | Read MCP resource |
| `/ma:update` | Refresh packaged assets |
| `/ma:repair` | Consolidate concept variants |
| `/ma:analyze-corruption` | Identify graph corruption |

## Agents

The plugin includes four specialized agents with full maenifold integration:

### researcher
Full maenifold stack with write access. 4-phase methodology:
1. BuildContext on task concepts
2. SearchMemories in relevant domains
3. Execute investigation
4. WriteMemory with findings

**Use cases**: Deep analysis, documentation synthesis, knowledge creation

### blue-team
Read-only defensive security agent. 6-step incident response:
1. BuildContext on security concepts
2. Search for known issues
3. Analyze defensive posture
4. Document findings
5. Recommend mitigations
6. Update threat model

**Use cases**: Security reviews, incident analysis, defensive validation

### red-team
Read-only offensive security agent. 5-phase adversarial testing:
1. BuildContext on attack surface
2. Search for vulnerabilities
3. Design exploitation scenarios
4. Document attack vectors
5. Recommend remediations

**Use cases**: Security audits, threat modeling, vulnerability discovery

### swe
Read-only implementation agent with frontend aesthetics focus. 5-step workflow:
1. BuildContext on implementation patterns
2. Search for existing solutions
3. Implement with quality standards
4. Self-review against requirements
5. Report completion

**Use cases**: Code implementation, bug fixes, feature development

## Concept-as-Protocol

All agents support automatic context enrichment via WikiLink concepts:
- Use `[[concept]]` syntax in prompts
- Triggers BuildContext + FindSimilarConcepts automatically
- Injects graph relationships into agent context
- Enables knowledge graph navigation without explicit commands

**Example**:
```
"Analyze the [[authentication]] system for [[security-vulnerabilities]]"
```

Automatically fetches:
- Direct relationships to authentication and security-vulnerabilities
- Similar concepts via semantic search
- Related files from knowledge graph
- Co-occurring concepts for expanded context

## Usage Examples

### Knowledge Creation
```
/ma:write "GraphRAG Implementation" "Implementing [[GraphRAG]] requires [[vector-embeddings]] and [[concept-extraction]]..."
/ma:sync
/ma:build-context "GraphRAG" --depth 2
```

### Research Workflow
```
/ma:search "transformer attention mechanisms" --mode Semantic
/ma:read memory://research/transformers/attention
/ma:extract memory://research/transformers/attention
```

### Sequential Thinking
```
/ma:think --session session-12345 --thought 0 --total 5 "Analyzing [[memory-system]] architecture..."
/ma:think --session session-12345 --thought 1 --total 5 "Identified [[SQLite]] as persistence layer..."
```

### Graph Navigation
```
/ma:find-similar "neural-network" --max 20
/ma:visualize "machine-learning" --depth 2
/ma:recent --filter thinking --timespan "24.00:00:00"
```

## Installation

1. Clone this repository to `~/.claude/plugins/maenifold/`
2. Ensure maenifold CLI is installed and accessible via PATH or `$MAENIFOLD_ROOT`
3. Restart Claude Code to load the plugin
4. Verify with `/ma:status`

## Configuration

Set environment variables to customize behavior:
- `MAENIFOLD_ROOT` - Installation directory (default: ~/maenifold)
- `MAENIFOLD_CLI_TIMEOUT` - Command timeout in seconds (default: 30)

## Architecture

```
~/.claude/plugins/maenifold/
├── .claude-plugin/
│   └── plugin.json         # Plugin manifest
├── commands/
│   ├── ma-write.md         # /ma:write command
│   ├── ma-search.md        # /ma:search command
│   └── ...                 # 25+ total commands
├── agents/
│   ├── researcher.md       # Full-stack research agent
│   ├── blue-team.md        # Defensive security agent
│   ├── red-team.md         # Offensive security agent
│   └── swe.md              # Implementation agent
├── hooks/
│   └── hooks.json          # Hook configuration
├── skills/
│   └── maenifold-cli/
│       └── SKILL.md        # Unified CLI documentation
└── README.md               # This file
```

## CLI Invocation Pattern

All commands internally use:
```bash
maenifold --tool ToolName --payload '{"param":"value"}'
```

JSON responses are parsed and formatted for Claude Code display.

## Troubleshooting

**"maenifold binary not found"**
→ Set `MAENIFOLD_ROOT` or add to PATH

**"Database not found"**
→ Run `/ma:sync` to create knowledge graph

**"No results from search"**
→ Ensure files contain `[[WikiLink]]` concepts and run `/ma:sync`

**Command timeout**
→ Increase `MAENIFOLD_CLI_TIMEOUT` for large operations

## Resources

- [maenifold Documentation](https://maenifold.dev)
- [GitHub Repository](https://github.com/ma-collective/maenifold)
- [CLI Scripting Guide](https://maenifold.dev/scripting)
- [Integration Patterns](https://maenifold.dev/integrations)

## License

MIT License - See repository for details
