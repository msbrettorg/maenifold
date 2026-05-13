# MCP sampling tool fixture

## Source

Official sampling example copied from
`modelcontextprotocol/typescript-sdk@2c0c481cb9dbfd15c8613f765c940a5f5bace94d`.
The upstream repository is MIT licensed.

## What it does

It serves a `summarize` tool over stdio. When the tool is called, the server
issues `sampling/createMessage` back to the MCP client so the client can
delegate the summarization prompt to an LLM and return the generated text.

## Run

```bash
npx tsx toolWithSampleServer.ts
```

See `apps/desktop/docs/mcp-test-fixtures/sampling.md` for source attribution,
expected client behavior, and the `maenifold.json` stdio snippet.
