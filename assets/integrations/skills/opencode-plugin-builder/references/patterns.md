# OpenCode Plugin Patterns

Production-tested patterns drawn from the community ecosystem. Each pattern includes a complete working example.

---

## Desktop Notifications

Alert the user when the AI finishes working:

```typescript
import type { Plugin } from "@opencode-ai/plugin"

export const NotifyPlugin: Plugin = async ({ $ }) => {
  return {
    event: async ({ event }) => {
      if (event.type === "session.idle") {
        // macOS
        await $`osascript -e 'display notification "Session completed!" with title "opencode"'`
        // Linux (requires notify-send)
        // await $`notify-send "opencode" "Session completed!"`
      }
    },
  }
}
```

---

## File Protection

Prevent the AI from reading or writing sensitive files:

```typescript
import type { Plugin } from "@opencode-ai/plugin"

const PROTECTED = [".env", ".env.local", ".env.production", "secrets.json"]

export const FileGuard: Plugin = async () => {
  return {
    "tool.execute.before": async (input, output) => {
      const filePath = output.args?.filePath || output.args?.path || ""
      const isProtected = PROTECTED.some((p) => filePath.endsWith(p))
      
      if (isProtected && (input.tool === "read" || input.tool === "write" || input.tool === "edit")) {
        throw new Error(`Blocked: ${input.tool} on protected file ${filePath}`)
      }
    },
  }
}
```

---

## Environment Variable Injection

Make env vars available to all shell commands the AI runs:

```typescript
import type { Plugin } from "@opencode-ai/plugin"

export const EnvInjector: Plugin = async () => {
  return {
    "shell.env": async (input, output) => {
      // Available in all bash tool calls and user terminals
      output.env.NODE_ENV = "development"
      output.env.PROJECT_ROOT = input.cwd
      
      // Forward secrets from your local env
      if (process.env.DATABASE_URL) {
        output.env.DATABASE_URL = process.env.DATABASE_URL
      }
    },
  }
}
```

---

## Custom Tool: External API

Give the AI access to an external service:

```typescript
import { type Plugin, tool } from "@opencode-ai/plugin"

export const JiraPlugin: Plugin = async () => {
  const baseUrl = process.env.JIRA_URL
  const token = process.env.JIRA_TOKEN

  return {
    tool: {
      jira_search: tool({
        description: "Search Jira issues by JQL query",
        args: {
          jql: tool.schema.string("JQL query string"),
          maxResults: tool.schema.optional(tool.schema.number("Max results to return")),
        },
        async execute(args) {
          const response = await fetch(`${baseUrl}/rest/api/3/search`, {
            method: "POST",
            headers: {
              Authorization: `Bearer ${token}`,
              "Content-Type": "application/json",
            },
            body: JSON.stringify({
              jql: args.jql,
              maxResults: args.maxResults ?? 10,
            }),
          })
          return JSON.stringify(await response.json())
        },
      }),
    },
  }
}
```

---

## Compaction Context Injection

Ensure important context survives session compaction:

```typescript
import type { Plugin } from "@opencode-ai/plugin"

export const CompactionContext: Plugin = async ({ directory }) => {
  return {
    "experimental.session.compacting": async (_input, output) => {
      // Read project-specific context to inject
      const file = Bun.file(`${directory}/.opencode/compaction-context.md`)
      if (await file.exists()) {
        const context = await file.text()
        output.context.push(context)
      }
    },
  }
}
```

---

## Command Logging / Audit Trail

Log every tool invocation for debugging or compliance:

```typescript
import type { Plugin } from "@opencode-ai/plugin"

export const AuditPlugin: Plugin = async ({ client }) => {
  return {
    "tool.execute.after": async (input, output) => {
      await client.app.log({
        body: {
          service: "audit",
          level: "info",
          message: `Tool executed: ${input.tool}`,
          extra: {
            tool: input.tool,
            args: JSON.stringify(output.args).slice(0, 500),
            timestamp: new Date().toISOString(),
          },
        },
      })
    },
  }
}
```

---

## Dynamic Context Pruning

Reduce token usage by cleaning up stale tool outputs:

```typescript
import type { Plugin } from "@opencode-ai/plugin"

export const PruningPlugin: Plugin = async ({ client }) => {
  return {
    "experimental.session.compacting": async (_input, output) => {
      output.context.push(
        "When summarizing this session, omit intermediate tool outputs that were " +
        "superseded by later corrections. Keep only the final state of each file."
      )
    },
  }
}
```

---

## Toast Notifications via SDK

Show in-app notifications from your plugin:

```typescript
import type { Plugin } from "@opencode-ai/plugin"

export const ToastPlugin: Plugin = async ({ client }) => {
  return {
    event: async ({ event }) => {
      if (event.type === "session.error") {
        await client.tui.showToast({
          body: { message: "Session encountered an error", variant: "error" },
        })
      }
    },
  }
}
```

---

## Combining Multiple Hooks

A single plugin can implement any combination of hooks, events, and tools:

```typescript
import { type Plugin, tool } from "@opencode-ai/plugin"

export const KitchenSink: Plugin = async ({ client, $ }) => {
  await client.app.log({
    body: { service: "kitchen-sink", level: "info", message: "Loaded" },
  })

  return {
    // Event handler
    event: async ({ event }) => {
      if (event.type === "session.idle") {
        await $`echo "Session done" >> /tmp/opencode.log`
      }
    },

    // Tool interceptors
    "tool.execute.before": async (input, output) => {
      if (input.tool === "bash" && output.args.command?.includes("rm -rf /")) {
        throw new Error("Blocked dangerous command")
      }
    },

    // Environment injection
    "shell.env": async (_input, output) => {
      output.env.OPENCODE_PLUGIN = "kitchen-sink"
    },

    // Compaction hook
    "experimental.session.compacting": async (_input, output) => {
      output.context.push("Remember: the kitchen-sink plugin is active.")
    },

    // Custom tools
    tool: {
      ping: tool({
        description: "Return pong",
        args: {},
        async execute() {
          return "pong"
        },
      }),
    },
  }
}
```

---

## Publishing to npm

Structure your package for distribution:

```
my-opencode-plugin/
├── src/
│   └── index.ts      # Named export(s)
├── package.json
├── tsconfig.json
└── README.md
```

**package.json essentials:**
```json
{
  "name": "opencode-my-plugin",
  "version": "1.0.0",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "files": ["dist"],
  "peerDependencies": {
    "@opencode-ai/plugin": "^0.1.0"
  },
  "scripts": {
    "build": "bun build src/index.ts --outdir dist --target node"
  }
}
```

Users install by adding the package name to their config:
```json
{
  "plugin": ["opencode-my-plugin"]
}
```
