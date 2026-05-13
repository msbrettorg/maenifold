# MCP Sampling Tool — Test Fixture Profile

**Source repository:** <https://github.com/modelcontextprotocol/typescript-sdk>  
**Source commit:** `2c0c481cb9dbfd15c8613f765c940a5f5bace94d`  
**Source URL:** <https://github.com/modelcontextprotocol/typescript-sdk/blob/2c0c481cb9dbfd15c8613f765c940a5f5bace94d/examples/server/src/toolWithSampleServer.ts>  
**License:** MIT (`examples/server/package.json`)  
**Attribution:** Official Model Context Protocol TypeScript SDK example server,
copied from `modelcontextprotocol/typescript-sdk` with the same lifted-from
comment style used by the in-tree elicitation fixtures.

> **Source path note:** The original backlog item named
> `examples/server/toolWithSampleServer.ts`. At the pinned source commit, the
> file exists at `examples/server/src/toolWithSampleServer.ts`.

---

## What it does

The server registers a stdio MCP tool named `summarize`. When called with
input text, the tool sends `sampling/createMessage` back to the connected MCP
client with a summarization prompt and `maxTokens: 500`. The tool then returns
the assistant text content from the sampling response.

Context7 grounding for the upstream TypeScript SDK confirmed the same local
stdio pattern: construct an `McpServer`, attach a tool, create a
`StdioServerTransport`, and call `server.connect(transport)`. It also documents
server-initiated sampling through `server.createMessage`.

## Expected client behavior

Today, without the T-303.4.3 approval path fully exercised, maenifold rejects
the upstream `sampling/createMessage` request through the deny/error path in
`crates/maenifold-core/src/mcp/client_handler.rs`: without `samplingAgent` it
returns a protocol error, and an explicit user denial returns
`User rejected sampling request`. The task note calls this the deny-stub path;
in this worktree it is represented by explicit protocol errors before any
allowed dispatch. This is honest behavior for this fixture and tests that
branch instead of silently dispatching an invisible LLM call.

Post-T-303.4.3, the intended fully exercised path is: the sampling approval
modal opens, the user picks allow or deny, and an allowed request dispatches an
LLM call that appears in the Sessions UI with token streaming and stop control.

## Manual run command

```bash
cd apps/desktop/test-fixtures/mcp-sampling-tool
npx tsx toolWithSampleServer.ts
```

The copied upstream example is a stdio server, so maenifold should spawn it
directly instead of connecting to an HTTP URL.

## `maenifold.json` stdio launch shape

The `McpServerEntry::Stdio` variant in
`crates/maenifold-core/src/acp/config/types.rs` is tagged with `type` and
requires `name`, `command`, and optional `args`, `timeout`, `env`, and
`gateway` fields.

```json
{
  "mcpServers": [
    {
      "type": "stdio",
      "name": "sampling-tool-fixture",
      "command": "npx",
      "args": [
        "tsx",
        "<absolute path to apps/desktop/test-fixtures/mcp-sampling-tool/toolWithSampleServer.ts>"
      ]
    }
  ]
}
```

## When end-to-end smoke test becomes possible

An end-to-end smoke test requires the daemon-side path that resolves the
sampling approval modal `allow_once` decision into completed dispatch and then
returns the response to the upstream MCP server. T-303.4.3 landed that intended
flow; see `done/T-303.4.3.md`. This fixture provides the upstream stdio server
needed to exercise it.
