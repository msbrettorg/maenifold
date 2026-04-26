# T-226.1 Decision Memo: Electron MCP SDK Streamable HTTP Compatibility

**Date:** 2026-04-25
**Recommendation:** **GO**

## Executive Summary

`@modelcontextprotocol/sdk` v1.29.0's `StreamableHTTPClientTransport` is **fully compatible** with the Electron renderer context. It uses exclusively browser-standard Web APIs (`fetch`, `ReadableStream`, `TextDecoderStream`, `TransformStream`) with zero Node.js-only dependencies in the client transport path. The SDK is already a transitive dependency via `@agentclientprotocol/sdk`, so promoting it to a direct dependency introduces **zero new packages**. The ~609 LOC of custom transport code can be replaced.

## 1. Node.js-Only Dependencies

**No blockers.** The `StreamableHTTPClientTransport` imports:

| Import | Node.js deps? | Notes |
|--------|--------------|-------|
| `shared/transport.js` | None | Pure functions using `Headers`, `fetch` |
| `types.js` | None | Zod schemas and type guards |
| `client/auth.js` | None | Uses `pkce-challenge` and `jose` (pure JS/WebCrypto) |
| `eventsource-parser/stream` | None | `EventSourceParserStream extends TransformStream` |

The `cross-spawn` dependency (Node.js-only) is imported ONLY in `client/stdio.js`. As long as imports target `@modelcontextprotocol/sdk/client/streamableHttp`, `cross-spawn` is never loaded.

Electron renderer config (`contextIsolation: true, nodeIntegration: false, sandbox: true`) is compatible.

## 2. SSE Streaming

**Compatible.** The SDK does NOT use browser `EventSource`. Instead:

1. `fetch()` GET with `Accept: text/event-stream`
2. `response.body` as `ReadableStream`
3. Pipe through `TextDecoderStream` -> `EventSourceParserStream`

All Web Streams API — available in Electron's Chromium renderer. Superior to our custom `parseSseJsonRpcResponse` which reads full body and splits on `data:` lines.

## 3. Session Header Handling

**Built-in.** The SDK's `_commonHeaders()` sets `mcp-session-id` and `mcp-protocol-version` automatically. Matches our `useMcpClient.ts` `buildHeaders()` behavior.

## 4. Dependency Impact

**Zero new packages.** The SDK is already resolved in `package-lock.json` as a transitive dep of `@agentclientprotocol/sdk`. Promoting to direct adds a line to `package.json` but no new `node_modules/`.

## 5. Files That Can Be Replaced

| File | LOC | Replacement |
|------|-----|-------------|
| `apps/desktop/src/composables/mcpTransport.ts` | 195 | SDK's `StreamableHTTPClientTransport` |
| `apps/desktop/src/composables/useMcpClient.ts` | 414 | SDK's `Client` + thin Vue composable wrapper (~50-80 LOC) |
| **Total** | **609 LOC removed** | **~80 LOC new** |

## 6. Implementation Caveats

1. **Import paths matter.** Use `@modelcontextprotocol/sdk/client/streamableHttp`, not barrel export, to avoid pulling in `stdio.js` -> `cross-spawn`.
2. **Vue wrapper still needed.** SDK's `Client` is not reactive — wrap `connected`, `error`, `sessionId` as Vue `ref()`s.
3. **Reconnect-on-404.** Our custom code reconnects on 404 (session expired). SDK doesn't — preserve in wrapper.
4. **Notification routing.** `onNotification()` maps to `client.setNotificationHandler()` — different API, same semantics.
5. **Protocol version.** SDK supports `2025-03-26` (our version). Negotiation is automatic.
