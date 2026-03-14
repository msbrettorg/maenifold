# OpenCode Plugin Events

Complete catalog of events plugins can subscribe to via the `event` handler.

## Subscribing to Events

```typescript
export const MyPlugin: Plugin = async (ctx) => {
  return {
    event: async ({ event }) => {
      switch (event.type) {
        case "session.idle":
          // Session finished processing
          break
        case "file.edited":
          // A file was modified
          break
      }
    },
  }
}
```

The `event` object always has a `type` string property. Additional properties vary by event type.

---

## Event Catalog

### Session Events

| Event | Fires When |
|-------|-----------|
| `session.created` | New session started |
| `session.updated` | Session metadata changed |
| `session.deleted` | Session removed |
| `session.idle` | Session finished processing (no pending work) |
| `session.status` | Session status changed (running, idle, etc.) |
| `session.error` | Session encountered an error |
| `session.compacted` | Session context was compacted |
| `session.diff` | Session diff updated |

**`session.idle`** is the most commonly used — it signals the AI has finished its current task. Great for notifications, post-processing, or cleanup.

### Message Events

| Event | Fires When |
|-------|-----------|
| `message.updated` | Message content changed |
| `message.removed` | Message deleted |
| `message.part.updated` | Part of a message updated |
| `message.part.removed` | Part of a message removed |

### File Events

| Event | Fires When |
|-------|-----------|
| `file.edited` | A file was created, modified, or deleted |
| `file.watcher.updated` | File watcher detected external changes |

### Tool Events

| Event | Fires When |
|-------|-----------|
| `tool.execute.before` | Before a tool runs (interceptor, not event) |
| `tool.execute.after` | After a tool finishes (interceptor, not event) |

Note: Tool events are actually **hooks** (before/after interceptors), not passive events. They use a different subscription pattern — see SKILL.md for hook syntax.

### Permission Events

| Event | Fires When |
|-------|-----------|
| `permission.asked` | OpenCode is requesting user permission |
| `permission.replied` | User responded to a permission request |

### Command Events

| Event | Fires When |
|-------|-----------|
| `command.executed` | A slash command was run |

### Server Events

| Event | Fires When |
|-------|-----------|
| `server.connected` | Client connected to server |

### Installation Events

| Event | Fires When |
|-------|-----------|
| `installation.updated` | OpenCode installation changed |

### LSP Events

| Event | Fires When |
|-------|-----------|
| `lsp.client.diagnostics` | Language server reported diagnostics |
| `lsp.updated` | Language server state changed |

### Todo Events

| Event | Fires When |
|-------|-----------|
| `todo.updated` | Session todo list changed |

### Shell Events

| Event | Fires When |
|-------|-----------|
| `shell.env` | Shell environment being prepared (interceptor — can inject env vars) |

Note: `shell.env` is an interceptor like tool hooks. Use it to inject environment variables.

### TUI Events

| Event | Fires When |
|-------|-----------|
| `tui.prompt.append` | Text appended to the TUI prompt |
| `tui.command.execute` | TUI command executed |
| `tui.toast.show` | Toast notification displayed |

---

## Event vs Hook

**Events** (`event` handler) are observe-only — you receive notification that something happened but cannot change the outcome.

**Hooks** (`tool.execute.before`, `tool.execute.after`, `shell.env`, `experimental.session.compacting`) are interceptors — you receive mutable `output` objects and can modify behavior or throw to block execution.

Both are registered in the same return object from your plugin function, just at different keys.
