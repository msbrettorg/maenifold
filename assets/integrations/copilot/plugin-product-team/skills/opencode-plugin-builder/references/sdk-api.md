# OpenCode SDK API Reference

The `client` object available in plugin context is a type-safe SDK client for the OpenCode server. This reference covers the APIs most useful for plugin development.

## Standalone SDK Usage

Outside of plugins, install and create a client:

```typescript
import { createOpencode } from "@opencode-ai/sdk"

// Start server + client
const { client } = await createOpencode({ port: 4096 })

// Or connect to existing server
import { createOpencodeClient } from "@opencode-ai/sdk"
const client = createOpencodeClient({ baseUrl: "http://localhost:4096" })
```

Types are importable directly:
```typescript
import type { Session, Message, Part } from "@opencode-ai/sdk"
```

---

## API Surface

### Global

| Method | Description |
|--------|-------------|
| `client.global.health()` | Server health check. Returns `{ healthy, version }` |

### App

| Method | Description |
|--------|-------------|
| `client.app.log({ body })` | Write structured log entry |
| `client.app.agents()` | List all available agents |

**Logging** is the most common SDK use in plugins:
```typescript
await client.app.log({
  body: {
    service: "my-plugin",
    level: "info",    // debug | info | warn | error
    message: "Something happened",
    extra: { key: "value" },
  },
})
```

### Sessions

| Method | Description |
|--------|-------------|
| `client.session.list()` | List all sessions |
| `client.session.get({ path: { id } })` | Get session by ID |
| `client.session.create({ body: { title? } })` | Create new session |
| `client.session.delete({ path: { id } })` | Delete session |
| `client.session.abort({ path: { id } })` | Abort running session |
| `client.session.messages({ path: { id } })` | List messages in session |
| `client.session.prompt({ path, body })` | Send prompt and get response |
| `client.session.command({ path, body })` | Execute slash command |
| `client.session.shell({ path, body })` | Run shell command |
| `client.session.share({ path: { id } })` | Share session |
| `client.session.summarize({ path, body })` | Summarize session |

**Sending a prompt:**
```typescript
const result = await client.session.prompt({
  path: { id: sessionId },
  body: {
    model: { providerID: "anthropic", modelID: "claude-sonnet-4-5" },
    parts: [{ type: "text", text: "Hello!" }],
  },
})
```

**Injecting context without triggering a response:**
```typescript
await client.session.prompt({
  path: { id: sessionId },
  body: {
    noReply: true,
    parts: [{ type: "text", text: "Context: you are working on project X." }],
  },
})
```

### Structured Output

Request validated JSON from the model:
```typescript
const result = await client.session.prompt({
  path: { id: sessionId },
  body: {
    parts: [{ type: "text", text: "Analyze this code" }],
    format: {
      type: "json_schema",
      schema: {
        type: "object",
        properties: {
          issues: { type: "array", items: { type: "string" } },
          severity: { type: "string", enum: ["low", "medium", "high"] },
        },
        required: ["issues", "severity"],
      },
    },
  },
})
// Access: result.data.info.structured_output
```

### Files

| Method | Description |
|--------|-------------|
| `client.find.text({ query: { pattern } })` | Search for text pattern |
| `client.find.files({ query: { query, type?, limit? } })` | Find files by name |
| `client.find.symbols({ query: { query } })` | Find workspace symbols |
| `client.file.read({ query: { path } })` | Read file content |
| `client.file.status()` | Get tracked file statuses |

### TUI Control

| Method | Description |
|--------|-------------|
| `client.tui.appendPrompt({ body: { text } })` | Add text to prompt |
| `client.tui.submitPrompt()` | Submit current prompt |
| `client.tui.clearPrompt()` | Clear prompt |
| `client.tui.showToast({ body: { message, variant } })` | Show notification |
| `client.tui.executeCommand({ body: { command } })` | Run a command |

Toast variants: `"success"`, `"error"`, `"info"`, `"warning"`

### Events (SSE)

```typescript
const events = await client.event.subscribe()
for await (const event of events.stream) {
  console.log(event.type, event.properties)
}
```

### Auth

```typescript
await client.auth.set({
  path: { id: "anthropic" },
  body: { type: "api", key: "sk-..." },
})
```

### Config

| Method | Description |
|--------|-------------|
| `client.config.get()` | Get current config |
| `client.config.providers()` | List providers and default models |

### Project

| Method | Description |
|--------|-------------|
| `client.project.list()` | List all projects |
| `client.project.current()` | Get current project |

---

## Error Handling

```typescript
try {
  await client.session.get({ path: { id: "invalid" } })
} catch (error) {
  console.error("Failed:", (error as Error).message)
}
```

The SDK throws on network errors and server errors. Always wrap SDK calls in try/catch within plugins — an unhandled exception in a plugin can crash the host.
