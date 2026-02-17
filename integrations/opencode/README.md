# OpenCode Integration

Bridges [OpenCode](https://opencode.ai) with maenifold's knowledge graph. Two TypeScript plugins hook into OpenCode's session lifecycle to ensure the graph grows through compaction events and conversation context persists across sessions.

## Plugins

### `plugins/compaction.ts`

Hooks into `experimental.session.compacting`. Injects WikiLink tagging guidelines into the compaction prompt so the LLM produces graph-friendly summaries with proper `[[WikiLink]]` syntax. This ensures every compaction event contributes concept nodes to the knowledge graph rather than producing flat prose that the graph cannot index.

Guidelines injected include: banned low-signal terms, normalization rules (lowercase-with-hyphens), anti-patterns, and examples.

### `plugins/persistence.ts`

Hooks into the `session.compacted` event to persist compaction summaries via maenifold's SequentialThinking tool. Maintains a per-project session map in memory (resets on OpenCode restart), caps summaries at 32K chars and total payloads at 50KB, and invokes the CLI with a 30-second timeout.

On each compaction:
1. Extracts the summary message from the compacted session.
2. Sanitizes text (NFKC normalize, strip control chars).
3. Wraps the summary as a SequentialThinking thought in the project's session chain.
4. Parses the `--json` CLI response to track the session ID for subsequent thoughts.

If no WikiLinks are present in the summary, it prepends `[[opencode]] [[compaction]]` to ensure minimum graph connectivity.

## Prerequisites

- **OpenCode CLI** installed and configured.
- **maenifold binary** in `PATH` (`which maenifold` should resolve).
- maenifold MCP server configured in `opencode.json` (separate from these plugins).

## Configuration

Register both plugins in your OpenCode configuration:

```typescript
import { CompactionPlugin } from "./plugins/compaction"
import { PersistencePlugin } from "./plugins/persistence"

export default {
  plugins: [CompactionPlugin, PersistencePlugin],
}
```

The persistence plugin reads these defaults (no configuration required):

| Constant | Value | Purpose |
|----------|-------|---------|
| Summary max chars | 32,000 | Cap on extracted summary text |
| Payload max chars | 50,000 | Cap on total SequentialThinking JSON payload |
| CLI timeout | 30s | Kill maenifold process if unresponsive |

The plugin degrades gracefully: if maenifold is unreachable or the CLI times out, errors are logged and a toast notification is shown. The plugin never throws into OpenCode's event loop.
