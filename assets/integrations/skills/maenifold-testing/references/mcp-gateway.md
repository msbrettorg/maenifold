# MCP and Gateway Testing

Use this reference for MCP/Gateway calls, gate status, GatewayExecute, GatewayIndex, raw curl testing, and jq parsing failures.

## Discovery before execution

Use `GatewayIndex` before `GatewayExecute`. Do not infer domains, command names, or parameters.

```text
GatewayIndex parameters={"domain":"maenifold.gate","includeSchemas":true}
GatewayExecute domain="maenifold.gate" command="<gate_*>" parameters={...}
```

## Raw response discipline

MCP responses may have `.result` on success or `.error` on failure. Inspect raw responses before applying `jq` filters:

```bash
jq -e '.error'
jq '.result'
```

Only read `.result` after confirming there is no `.error`.

## Shell quoting

For bearer-token curl calls, use double quotes around headers containing shell variables:

```bash
-H "Authorization: Bearer $TOKEN"
```

Single quotes send the literal `$TOKEN`.

## CLI fallback

If MCP is unavailable and a CLI fallback is needed, run:

```bash
maenifold --help
```

Do not assume fallback command names from old notes. If docs and binary disagree, file/update the testing lesson instead of treating the fallback as healthy.

## Sampling capability-gate smoke test

MCP sampling is server-initiated and client-completed: the server sends
`sampling/createMessage`, the client reviews/modifies the request, the client
samples from an LLM, the client reviews/modifies the completion, and the client
returns the result to the server. A test that stops before the client-side LLM
completion does NOT prove sampling works.

For MCP sampling tool-capability regressions, use the focused ignored Rust test
`stdio_ts_sdk_sampling_tools_request_reaches_create_message_handler`. It spawns
the real `apps/desktop/test-fixtures/mcp-sampling-tool/multiTurnToolLoopServer.ts`
stdio server via fixture-local `tsx`, calls `chain-of-tools` with `tools` and
`toolChoice`, and distinguishes two pre-completion failure layers:

- `Client does not support sampling tools capability` means the TS SDK rejected
	the initialize capabilities before sending `sampling/createMessage`.
- `CallToolResult { is_error: Some(true), ... "No sampling agent configured" }`
	means the TS SDK accepted `capabilities.sampling.tools` and reached
	Maenifold's `create_message` handler. This is only a capability-gate smoke
	test; the remaining proof is client-side approval + real LLM sampling + result
	return to the server.

## In-process serialization tests do NOT prove cross-implementation interop (iter-14 lesson)

Integration tests that use `tokio::io::duplex` or any in-process channel prove ONE thing: the local serializer emits the bytes you expect. They do NOT prove:

- A foreign-language MCP server (e.g., `@modelcontextprotocol/sdk` TS server, or a Python MCP server) will accept those bytes
- The transport-layer code path used by production (stdio child process, WebSocket relay) matches the one exercised by the duplex test
- Spec-version negotiation, optional-field tolerance, or capability-shape strictness on the remote end

T-326.4.1 shipped a wire-format test (`initialize_request_wire_format_includes_sampling_capability`) that proves rmcp serializes `params.capabilities.sampling.tools` correctly. That test passes. But three CDP UAT attempts against a real `@modelcontextprotocol/sdk` TS server still returned `"Client does not support sampling tools capability"`. Either the TS SDK has stricter validation than checking the key exists, OR the running daemon hadn't actually been replaced by the freshly-built binary at UAT time (see `pipeline-workflow-drivers.md` daemon-redeploy verification). Both possibilities require a real end-to-end test — spawning the TS server as a stdio child of the daemon and asserting `createMessage` round-trips — before claiming the wire format works in practice.

Rule: when a test name promises end-to-end behavior (`*_works`, `*_succeeds`, `*_round_trips`), the test must exercise the same transport + the same peer implementation production uses. Otherwise the test name is overclaiming.
