# MCP ext-apps Examples — Test Fixture Profile

**Source:** <https://github.com/modelcontextprotocol/ext-apps>  
**License:** MIT / Apache-2.0 transition (same dual-license preamble as official MCP repos)  
**Last verified:** 2026-05-13  
**npm package pattern:** `@modelcontextprotocol/server-<name>` (one package per example)

---

## What it provides

The ext-apps repository is the official MCP Apps extension project. It defines the
`MCP Apps` specification (`_meta.ui.resourceUri` / `ui://` scheme), the SDK packages
(`@modelcontextprotocol/ext-apps`), and a gallery of Node.js example servers that emit
`ui://` resources — driving a sandboxed iframe in any compliant host.

In maenifold the consuming side is `apps/desktop/src/components/McpAppFrame.vue`, which
reads `_meta.ui.resourceUri` from a completed tool call, fetches the resource from the
MCP server, and renders it in a sandboxed iframe via
`@modelcontextprotocol/ext-apps/app-bridge`.

Example servers documented as test fixtures:

| Server | npm package | UI focus | Primary tool |
|--------|-------------|----------|--------------|
| `cohort-heatmap-server` | `@modelcontextprotocol/server-cohort-heatmap` | Retention heatmap (React) | `get-cohort-data` |
| `customer-segmentation-server` | `@modelcontextprotocol/server-customer-segmentation` | Scatter/bubble chart (Chart.js) | `get-customer-data` |
| `basic-server-vanillajs` | `@modelcontextprotocol/server-basic-vanillajs` | Minimal starter (Vanilla JS) | varies |
| `video-resource-server` | `@modelcontextprotocol/server-video-resource` | Video player (binary blob) | `play_video` |
| `pdf-server` | `@modelcontextprotocol/server-pdf` | PDF viewer (chunked, PDF.js) | `open_pdf` |

> **`--stdio` is required.** Every ext-apps Node.js example defaults to HTTP mode.
> Pass `--stdio` explicitly (unlike `@modelcontextprotocol/server-everything`).

> **Python examples** (`qr-server`, `say-server`) use `uv run` — not documented here.

---

## Configuration snippet for maenifold.json

Matches the `McpServerEntry::Stdio` variant in
`crates/maenifold-core/src/acp/config/types.rs` (required: `type`, `name`, `command`;
`args` defaults to `[]`; `env` defaults to `{}`; `gateway` defaults to `false`).

### Cohort Heatmap (recommended primary fixture)

```json
{
  "mcpServers": [
    {
      "type": "stdio",
      "name": "cohort-heatmap",
      "command": "npx",
      "args": ["-y", "--silent", "@modelcontextprotocol/server-cohort-heatmap", "--stdio"]
    }
  ]
}
```

### Basic VanillaJS (minimal protocol compliance fixture)

```json
{
  "mcpServers": [
    {
      "type": "stdio",
      "name": "basic-vanillajs",
      "command": "npx",
      "args": ["-y", "--silent", "@modelcontextprotocol/server-basic-vanillajs", "--stdio"]
    }
  ]
}
```

For other fixtures, substitute the package name from the table above into the same
`args` shape (replacing `@modelcontextprotocol/server-cohort-heatmap`).

> **No pre-install required.** `npx -y` downloads the package on first use;
> Node.js ≥ 18 must be on `PATH`.
>
> The upstream READMEs also include `--registry=https://registry.npmjs.org/` to avoid
> private-registry mismatches — add it if `npx` resolves incorrectly in your env.

---

## Manual install steps

### Option A — Settings UI (T-303.2 merged in dev)

1. Open maenifold → Settings → MCP Servers → **Add Server**.
2. Paste one of the JSON snippets above (or fill in: type `stdio`, name as shown,
   command `npx`, args array as shown).
3. Save. The server is available on the next tool call.

### Option B — Direct config file edit

1. Locate: macOS `~/Library/Application Support/maenifold/maenifold.json`,
   Linux `~/.config/maenifold/maenifold.json`.
2. Add the desired entry to the `mcpServers` array and save.
3. Restart maenifold (or reload config from Settings if hot-reload is available).

---

## What this fixture tests

The ext-apps examples close the **MCP Apps / chart-style UI** coverage row — the row
that `@modelcontextprotocol/server-everything` explicitly marks N/A and defers here.

| Coverage matrix row | Closed | Server |
|---------------------|--------|--------|
| MCP Apps / chart-style UI (`_meta.ui.resourceUri`) | Y | `cohort-heatmap-server` or `customer-segmentation-server` |
| Video resource blob | Y | `video-resource-server` |
| Binary/download fallback | Y | `pdf-server` |
| All other rows | N | ext-apps is UI-focused; use Everything for protocol coverage |

Specifically for `McpAppFrame.vue`, these fixtures exercise:

- `_meta.ui.resourceUri` extraction and `ui://` prefix guard in `agent-capabilities-helpers.ts`.
- `resources/read` fetch triggered by `McpAppFrame.vue`.
- `AppBridge` + `PostMessageTransport` handshake and iframe lifecycle.
- `buildAllowAttribute` from `appUi.permissions` metadata.
- Sandboxed iframe sizing via resize messages.
- Scoped tool-call enforcement (app may only call tools from its originating server).
- `mcp-app-sandbox://` privileged scheme + CSP in `apps/desktop/electron/main.ts`.

---

## Out of scope

- **CDP UAT** — deferred to a separate task per T-303.13 acceptance criteria.
- **HTTP transport** — stdio only; upstream also supports `npm run start:http`.
- **Authentication / secrets** — none of the documented servers require credentials.
- **Production config** — test fixture only; do not add to production `maenifold.json`.
