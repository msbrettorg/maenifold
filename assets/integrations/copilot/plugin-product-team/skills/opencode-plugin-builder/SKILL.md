---
name: opencode-plugin-builder
description: Build OpenCode plugins that extend the AI coding agent with custom hooks, tools, and integrations. Use this skill whenever the user wants to create, debug, or publish an OpenCode plugin — including hooking into events (session, tool, file, permission, compaction), creating custom tools with Zod schemas, injecting environment variables, controlling the TUI, or integrating with the OpenCode SDK. Also use when the user mentions ".opencode/plugins", "opencode plugin", "@opencode-ai/plugin", plugin hooks, or wants to extend OpenCode's behavior in any way. If the user is working with an existing OpenCode plugin and needs help understanding hooks, fixing type errors, or adding new functionality, use this skill.
---

# OpenCode Plugin Builder

Build plugins that extend OpenCode — the open-source AI coding agent. Plugins hook into OpenCode's event system to customize behavior, add tools the AI can call, inject context, and integrate with external services.

## What is an OpenCode Plugin?

A plugin is a **TypeScript or JavaScript module** that exports one or more named functions. Each function receives a context object and returns an object containing hook implementations, event handlers, and/or custom tool definitions.

```typescript
import type { Plugin } from "@opencode-ai/plugin"

export const MyPlugin: Plugin = async ({ project, client, $, directory, worktree }) => {
  return {
    // Event handlers, hooks, and tools go here
  }
}
```

The context object provides:
- `project` — current project metadata
- `directory` — current working directory
- `worktree` — git worktree path
- `client` — OpenCode SDK client (full API access)
- `$` — Bun's shell API for running commands

## Plugin Locations

Plugins load from two places:
- **Project-level**: `.opencode/plugins/` — scoped to this project
- **Global**: `~/.config/opencode/plugins/` — available everywhere

They can also be installed from npm via `opencode.json`:
```json
{
  "$schema": "https://opencode.ai/config.json",
  "plugin": ["my-published-plugin", "@my-org/custom-plugin"]
}
```

Local plugins are loaded directly. npm plugins are auto-installed via Bun at startup and cached in `~/.cache/opencode/node_modules/`.

## Core Concepts

### Hooks (Before/After Interceptors)

Hooks let you intercept and modify tool execution. They receive an `input` (what was requested) and an `output` (what will happen), and can mutate `output` to change behavior.

**`tool.execute.before`** — runs before any tool executes. Throw to block execution.

```typescript
"tool.execute.before": async (input, output) => {
  // input.tool — the tool name (e.g., "read", "write", "bash")
  // output.args — the arguments that will be passed to the tool
  if (input.tool === "read" && output.args.filePath.includes(".env")) {
    throw new Error("Reading .env files is not allowed")
  }
}
```

**`tool.execute.after`** — runs after a tool finishes. Can modify the output.

```typescript
"tool.execute.after": async (input, output) => {
  // input.tool — which tool ran
  // output.output — the tool's result (mutable)
  if (input.tool === "task") {
    output.output += "\n\n[Plugin: task completed]"
  }
}
```

### Events (Observe-Only)

Events fire when things happen. Subscribe with a single `event` handler:

```typescript
event: async ({ event }) => {
  if (event.type === "session.idle") {
    await $`osascript -e 'display notification "Done!" with title "opencode"'`
  }
}
```

See `references/events.md` for the complete event catalog.

### Custom Tools

Add tools the AI can invoke during a session. Use the `tool` helper from `@opencode-ai/plugin`:

```typescript
import { tool } from "@opencode-ai/plugin"

return {
  tool: {
    lookup_user: tool({
      description: "Look up a user by email address",
      args: {
        email: tool.schema.string("The user's email address"),
      },
      async execute(args, context) {
        const { directory, worktree } = context
        const result = await fetchUser(args.email)
        return JSON.stringify(result)
      },
    }),
  },
}
```

The `tool.schema` helper creates Zod schemas:
- `tool.schema.string(description?)` — string argument
- `tool.schema.number(description?)` — number argument
- `tool.schema.boolean(description?)` — boolean argument
- `tool.schema.enum(values[], description?)` — enum argument
- `tool.schema.optional(schema)` — make any schema optional

Custom tools appear alongside built-in tools and the AI can call them naturally.

### Shell Environment Injection

Inject environment variables into all shell execution (AI tools and user terminals):

```typescript
"shell.env": async (input, output) => {
  output.env.MY_API_KEY = process.env.MY_API_KEY ?? ""
  output.env.PROJECT_ROOT = input.cwd
}
```

### Compaction Hooks (Experimental)

Customize what context survives when a session is compacted:

```typescript
"experimental.session.compacting": async (input, output) => {
  // Append extra context to the compaction prompt
  output.context.push("## Important\nAlways remember X when resuming.")
  
  // OR replace the compaction prompt entirely:
  // output.prompt = "Your custom compaction prompt..."
}
```

When `output.prompt` is set, it replaces the default compaction prompt entirely and `output.context` is ignored.

## Building a Plugin Step-by-Step

### 1. Create the file

```bash
mkdir -p .opencode/plugins
touch .opencode/plugins/my-plugin.ts
```

### 2. Write the plugin

Start with the typed export pattern:

```typescript
import type { Plugin } from "@opencode-ai/plugin"

export const MyPlugin: Plugin = async (ctx) => {
  const { client, $, directory } = ctx
  
  // Initialization code runs once at startup
  await client.app.log({
    body: { service: "my-plugin", level: "info", message: "Plugin loaded" }
  })
  
  return {
    // Your hooks, events, and tools
  }
}
```

### 3. Add dependencies (if needed)

For external npm packages, create a `package.json` in your config directory:

```json
{
  "dependencies": {
    "some-library": "^1.0.0"
  }
}
```

OpenCode runs `bun install` at startup to make these available.

### 4. Test it

Restart opencode (or start a new session). Check the logs for your initialization message. Trigger the hooks you've implemented.

### 5. Publish (optional)

Package as an npm module. Users install via config:
```json
{ "plugin": ["your-package-name"] }
```

## Using the SDK Client

The `client` object in your plugin context is a full OpenCode SDK client. Use it for:

- **Logging**: `client.app.log({ body: { service, level, message, extra? } })`
- **Sessions**: `client.session.list()`, `client.session.create()`, `client.session.prompt()`
- **Files**: `client.find.text()`, `client.find.files()`, `client.file.read()`
- **TUI**: `client.tui.showToast()`, `client.tui.appendPrompt()`
- **Events**: `client.event.subscribe()` for SSE stream

Read `references/sdk-api.md` for the full SDK API surface.

## Common Patterns

Read `references/patterns.md` for production-tested patterns including:
- Notification plugins (desktop alerts on session idle)
- Security plugins (.env protection, secret redaction)
- Context injection (compaction hooks, environment variables)
- Custom tool creation (external API integration)
- SDK-driven automation (programmatic session control)
- Observability (logging, metrics, tracing)

## TypeScript Setup

For typed plugins, install the type package:

```bash
bun add -d @opencode-ai/plugin
```

This gives you:
- `Plugin` type for your export function
- `tool()` helper for creating custom tools
- `tool.schema` for argument schemas

The types are generated from the server's OpenAPI spec. All API types (`Session`, `Message`, `Part`, etc.) are importable from `@opencode-ai/sdk`.

## Plugin Load Order

Plugins load in this sequence:
1. Global config npm plugins (`~/.config/opencode/opencode.json`)
2. Project config npm plugins (`opencode.json`)
3. Global plugin directory (`~/.config/opencode/plugins/`)
4. Project plugin directory (`.opencode/plugins/`)

All hooks run in sequence across all loaded plugins. Duplicate npm packages (same name + version) load once. A local plugin and an npm plugin with similar names both load.

## Debugging

- Use `client.app.log()` instead of `console.log` for structured logging
- Levels: `debug`, `info`, `warn`, `error`
- Wrap risky operations in try/catch — a throwing plugin can break opencode
- Check `~/.cache/opencode/node_modules/` if npm plugins fail to load
- Test hooks by triggering the relevant tool or event manually

## Anti-Patterns to Avoid

- **Blocking hooks**: Long-running sync work in `tool.execute.before` stalls the AI. Use async operations and timeouts.
- **Swallowing errors silently**: Log failures so you can debug later.
- **Mutating `input`**: Only mutate `output`. The `input` object is for reading what was requested.
- **Heavy initialization**: Plugin functions run at startup. Defer expensive work to first use.
- **Hardcoded paths**: Use `directory` and `worktree` from context instead of absolute paths.
