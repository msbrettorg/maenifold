# MCP Elicitation Servers — Test Fixture Profile

**Source repository:** <https://github.com/modelcontextprotocol/typescript-sdk>  
**Source commit:** `2c0c481cb9dbfd15c8613f765c940a5f5bace94d`  
**License:** MIT (`examples/server/package.json`)  
**Attribution:** Official Model Context Protocol TypeScript SDK example servers,
copied from `modelcontextprotocol/typescript-sdk` with a one-line lifted-from
comment prepended to each local fixture file.

> **Source path note:** The requested upstream URL used `src/examples/server`.
> The files exist in the current upstream tree at `examples/server/src/`.

---

## Fixture: form elicitation

**Local fixture:** `apps/desktop/test-fixtures/mcp-elicitation-form/elicitationFormExample.ts`  
**Source URL:** <https://github.com/modelcontextprotocol/typescript-sdk/blob/2c0c481cb9dbfd15c8613f765c940a5f5bace94d/examples/server/src/elicitationFormExample.ts>  
**Upstream file SHA:** `e059e8452d6907a0fbc71f496fc1725d3de9bb61`

### What it does

The server registers three tools that call `elicitation/create` with
`mode: "form"`:

| Tool | Form requested |
|------|----------------|
| `register_user` | Username, email, password, optional newsletter checkbox |
| `create_event` | Two-step event details flow: title/description, then date/time/duration |
| `update_shipping_address` | Recipient name, street, city, state, ZIP, optional phone |

### Expected client behavior

When one of the tools runs, maenifold should open an elicitation modal rendered
from the supplied JSON Schema. The user fills the structured fields and submits
or declines. The client returns the accepted form content (or decline/cancel
action) to the server, and the tool result reflects what the server received.

### Manual run command

```bash
cd apps/desktop/test-fixtures/mcp-elicitation-form
npx tsx elicitationFormExample.ts
```

The copied upstream example is a Streamable HTTP server and prints its MCP URL
as `http://localhost:3000/mcp`.

### `maenifold.json` HTTP launch shape

The `McpServerEntry::Http` variant in
`crates/maenifold-core/src/acp/config/types.rs` requires `type` and `url`.
Use this when the upstream example is run as a Streamable HTTP server.

```json
{
  "mcpServers": [
    {
      "type": "http",
      "name": "elicitation-form",
      "url": "http://localhost:3000/mcp"
    }
  ]
}
```

> **Run order:** start the upstream HTTP server first
> (`npx tsx elicitationFormExample.ts`) so `http://localhost:3000/mcp` is
> reachable before maenifold connects.

---

## Fixture: URL elicitation

**Local fixture:** `apps/desktop/test-fixtures/mcp-elicitation-url/elicitationUrlExample.ts`  
**Source URL:** <https://github.com/modelcontextprotocol/typescript-sdk/blob/2c0c481cb9dbfd15c8613f765c940a5f5bace94d/examples/server/src/elicitationUrlExample.ts>  
**Upstream file SHA:** `93b59152f8b3697bedeebfc83ff30c28a2e8d993`

### What it does

The server registers two tools that require URL elicitation:

| Tool | URL requested |
|------|---------------|
| `payment-confirm` | A local payment confirmation URL containing session and elicitation IDs |
| `third-party-auth` | A demo third-party authorization URL |

The fixture also includes the upstream `inMemoryEventStore.ts` helper used by
the URL example's resumable Streamable HTTP transport.

### Expected client behavior

When one of the tools runs, maenifold should open an elicitation modal that
presents the URL prompt and lets the user open or acknowledge the requested URL.
After the user completes the prompted action, the client reports completion to
the server and the server resumes the tool flow.

### Manual run command

```bash
cd apps/desktop/test-fixtures/mcp-elicitation-url
npx tsx elicitationUrlExample.ts
```

The copied upstream example starts an MCP HTTP endpoint on
`http://localhost:3000/mcp` and an auxiliary auth endpoint on
`http://localhost:3001`.

### `maenifold.json` HTTP launch shape

```json
{
  "mcpServers": [
    {
      "type": "http",
      "name": "elicitation-url",
      "url": "http://localhost:3000/mcp"
    }
  ]
}
```

> **Run order:** start the upstream HTTP server first
> (`npx tsx elicitationUrlExample.ts`) so `http://localhost:3000/mcp` and
> the auxiliary auth endpoint on `http://localhost:3001` are reachable
> before maenifold connects.

---

## Sampling fixture (blocked on T-303.4.3)

The third upstream example, `toolWithSampleServer.ts`, is intentionally not
copied in this fixture set. Sampling fixture wiring is replanned to T-303.15.1
because daemon-side LLM dispatch is blocked on T-303.4.2 architectural
research and has no accepted design path yet.
