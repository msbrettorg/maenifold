# MCP Everything Server — Test Fixture Profile

**Source:** <https://github.com/modelcontextprotocol/servers/tree/main/src/everything>  
**License:** MIT  
**Last verified:** 2026-05-13  
**npm package:** `@modelcontextprotocol/server-everything`

---

## What it provides

The Everything server is the official MCP reference test server. It is not intended as
a useful production server; its explicit purpose is to exercise every MCP protocol
surface so that MCP client builders can validate rendering and protocol correctness.

Tool surfaces exposed (from [upstream features doc][features]):

| Tool | Content type exercised |
|------|------------------------|
| `echo` | Plain `text` block |
| `get-annotated-message` | Annotated `text` (priority / audience metadata) |
| `get-tiny-image` | `image` content block (base64 PNG, tiny MCP logo) |
| `get-resource-links` | Multiple `resource_link` items (alternates text/blob URIs) |
| `get-resource-reference` | Inline `resource` block (embedded resource with uri + mimeType + data) |
| `get-structured-content` | `structuredContent` + backward-compatible `text` fallback |
| `gzip-file-as-resource` | Binary `resource` / `resource_link` (application/gzip, session-scoped) |
| `trigger-sampling-request` | Sampling — issues `sampling/createMessage` to the client/LLM |
| `trigger-sampling-request-async` | Bidirectional task: server sends sampling request, client executes as task |
| `trigger-long-running-operation` | Progress notifications (`notifications/progress`) over configurable steps |
| `toggle-simulated-logging` | Logging notifications (all log levels, per-session) |
| `toggle-subscriber-updates` | Resource subscription notifications (`notifications/resources/updated`) |
| `simulate-research-query` | MCP Tasks (SEP-1686): server-side task with elicitation when `ambiguous: true` |
| `trigger-elicitation-request-async` | Bidirectional task: server sends elicitation request, client executes as task |
| `get-env` | Text (diagnostic: exposes process env) |
| `get-roots-list` | Text (diagnostic: echoes last client roots list) |
| `get-sum` | Text (arithmetic: validated numeric inputs) |

Additional protocol surfaces:

- **Prompts:** `simple-prompt`, `args-prompt`, `completable-prompt` (argument auto-completion), `resource-prompt`
- **Resources:** dynamic text/blob at `demo://resource/dynamic/{text,blob}/{index}`, static docs at `demo://resource/static/document/<filename>`, session-scoped at `demo://resource/session/<name>`
- **Resource subscriptions:** opt-in per-session via `toggle-subscriber-updates`
- **MCP Tasks (SEP-1686):** advertises `tasks.list`, `tasks.cancel`, `tasks.requests.tools.call`

[features]: https://github.com/modelcontextprotocol/servers/blob/main/src/everything/docs/features.md

---

## Configuration snippet for maenifold.json

The following entry is the minimal stdio configuration. It matches the
`McpServerEntry::Stdio` variant defined in
`crates/maenifold-core/src/acp/config/types.rs` (required fields: `type`,
`name`, `command`; `args` defaults to `[]`; `env` defaults to `{}`; `gateway`
defaults to `false`).

```json
{
  "mcpServers": [
    {
      "type": "stdio",
      "name": "everything",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-everything"]
    }
  ]
}
```

> **Note on `env` field:** The Rust type is `HashMap<String, String>`, not an
> array. Omit `env` entirely (or use `{}`) when no environment overrides are
> needed; the field is skipped during serialization when empty.

---

## Manual install steps

T-303.2 (Settings UI → MCP Servers) is merged in the dev branch but may not yet
be available in every production build. Use whichever path matches your build:

### Option A — Settings UI (T-303.2 already merged in dev)

1. Open maenifold → Settings → MCP Servers.
2. Click **Add Server**.
3. Paste the JSON snippet above (or fill in the form fields: type `stdio`,
   name `everything`, command `npx`, args `["-y",
   "@modelcontextprotocol/server-everything"]`).
4. Save. The server will be registered and available on next tool call.

### Option B — Direct config file edit

1. Locate the user config file:
   - macOS: `~/Library/Application Support/maenifold/maenifold.json`
   - Linux: `~/.config/maenifold/maenifold.json`
2. Open the file in a text editor.
3. Add the entry to the `mcpServers` array:
   ```json
   {
     "mcpServers": [
       {
         "type": "stdio",
         "name": "everything",
         "command": "npx",
         "args": ["-y", "@modelcontextprotocol/server-everything"]
       }
     ]
   }
   ```
4. Save and restart maenifold (or reload config from the Settings panel if hot
   reload is available).

> **No pre-install required.** `npx -y` downloads and runs the package on first
> use; Node.js ≥ 18 must be on `PATH`.

---

## What this fixture tests

The following rows in the `apps/desktop/docs/mcp-test-apps.md` coverage matrix
currently list `Has test app? = N`. The Everything server closes those gaps:

| Coverage matrix row | Closed by Everything tool |
|---------------------|--------------------------|
| Image (base64 data URI) | `get-tiny-image` |
| Resource link (`file://` / `demo://`) | `get-resource-links` |
| Embedded resource | `get-resource-reference` |
| Structured content (`structuredContent`) | `get-structured-content` |
| Sampling request | `trigger-sampling-request` |
| Progress notifications | `trigger-long-running-operation` |
| Logging notifications | `toggle-simulated-logging` |
| Resource subscriptions / updates | `toggle-subscriber-updates` |
| Binary/download fallback | `gzip-file-as-resource` |
| Cancellation | `simulate-research-query` (MCP Tasks cancel path) |
| Elicitation request | `simulate-research-query` (ambiguous=true) / `trigger-elicitation-request-async` |

Rows **not** closed by Everything (separate fixtures needed):

| Coverage matrix row | Why not covered |
|---------------------|----------------|
| Audio (base64 data URI) | No audio tool in upstream Everything |
| Video resource blob | No video tool in upstream Everything |
| MCP Apps / chart-style UI (`_meta.ui.resourceUri`) | Requires `ext-apps` fixture, not Everything |
| Long response (>25KB) | No deterministic large-payload tool in upstream Everything |

---

## Out of scope

- **CDP UAT** — deferred to a separate task per T-303.12 acceptance criteria.
- **HTTP transport** — this profile documents stdio only; the upstream package
  also supports SSE and streamable HTTP but those are out of scope here.
- **Authentication / secrets** — the Everything server requires no credentials.
- **Production mcpServers configuration** — this document describes a developer
  test fixture; do not add the Everything server to production `maenifold.json`
  or default user configs.
