# MCP and Gateway Testing

Use this reference for MCP/Gateway calls, gate status, GatewayExecute, GatewayIndex, raw curl testing, and jq parsing failures.

## Discovery before execution

Use `GatewayIndex` before `GatewayExecute`. Do not infer domains, command names, or parameters.

```text
GatewayIndex parameters={"domain":"maenifold.gate","includeSchemas":true}
GatewayExecute domain="maenifold.gate" command="<gate_*>" parameters={...}
```

## Execution-path chain

Always reach gateway commands in this exact order. There is no curl-to-`/mcp` step. No `mae_mcp` helper exists; do not reintroduce one.

1. **MCP tools (primary).** Use `maenifold-GatewayExecute`, `maenifold-GatewayIndex`, etc. Discover with `GatewayIndex` first; execute with `GatewayExecute`. Examples:

   ```text
   GatewayExecute domain="maenifold.gate" command="gate_Sync" parameters={"root":"/abs/path","rebuild":true}
   GatewayExecute domain="maenifold.gate" command="gate_Pass" parameters={"root":"/abs/path","taskId":"T-X.Y","agent":"ralph-pm","notes":"...","expectedStage":"red-team"}
   GatewayExecute domain="maenifold.memory" command="WriteMemory" parameters={"title":"...","folder":"mae-auto/iterations","content":"..."}
   ```

2. **Start the daemon if MCP tools aren't exposed in your session.** On macOS: `launchctl kickstart -k gui/$(id -u)/com.ma-collective.maenifold`. For a full release rebuild + restart: `./rebuild.sh` from the repo root. Wait a few seconds for the HTTP listener, then re-check the tool inventory.

3. **CLI fallback (if MCP still unreachable).** Use the in-process CLI:

   ```bash
   maenifold tool GatewayExecute '{"domain":"maenifold.gate","command":"gate_Status","parameters":{"root":"/abs/path"}}'
   maenifold tool WriteMemory '{"title":"...","folder":"mae-auto/iterations","content":"..."}'
   ```

   The CLI loads tools in-process, so it works even when the HTTP daemon is down. List available tools with `maenifold tool --help` and `maenifold --help`.

4. **Fail fast.** If neither MCP tools nor the CLI binary work, stop and report the exact failure to the user (the commands you tried and what they returned). Do not curl `/mcp`. Do not hand-roll an MCP HTTP envelope. Do not invent a fourth path.

## Raw response discipline

MCP / CLI responses use a `{ "success": bool, "data": ..., "error": ... }` envelope (or `{ "result": ..., "isError": bool }` for raw JSON-RPC). Always check `error` / `isError` before reading `data` / `result`. If you receive JSON, parse it before composing follow-up calls.

## Sampling capability-gate smoke test

MCP sampling is server-initiated and client-completed: the server sends
`sampling/createMessage`, the client reviews/modifies the request, the client
samples from an LLM, the client reviews/modifies the completion, and the client
returns the result to the server. A test that stops before the client-side LLM
completion does NOT prove sampling works.

	The previous capability-gate smoke test depended on a TS fixture under
	`apps/desktop/test-fixtures/mcp-sampling-tool/` that was removed when the
	front-end was ripped. If sampling tool-capability regressions need verifying,
	add a new fixture (e.g. under `crates/maenifold-core/tests/fixtures/`) that
	spawns a real `@modelcontextprotocol/sdk` TS server as a stdio child of the
	daemon and asserts the two distinguishable failure layers:

	- `Client does not support sampling tools capability` — the TS SDK rejected
	  the initialize capabilities before sending `sampling/createMessage`.
	- `CallToolResult { is_error: Some(true), ... "No sampling agent configured" }`
	  — the TS SDK accepted `capabilities.sampling.tools` and reached
	  Maenifold's `create_message` handler. This is only a capability-gate smoke
	  test; the remaining proof is client-side approval + real LLM sampling + result
	  return to the server.

	## In-process serialization tests do NOT prove cross-implementation interop (iter-14 lesson)

	Integration tests that use `tokio::io::duplex` or any in-process channel prove ONE thing: the local serializer emits the bytes you expect. They do NOT prove:

	- A foreign-language MCP server (e.g., `@modelcontextprotocol/sdk` TS server, or a Python MCP server) will accept those bytes
	- The transport-layer code path used by production (stdio child process) matches the one exercised by the duplex test
	- Spec-version negotiation, optional-field tolerance, or capability-shape strictness on the remote end

	T-326.4.1 shipped a wire-format test (`initialize_request_wire_format_includes_sampling_capability`) that proves rmcp serializes `params.capabilities.sampling.tools` correctly. That test passes. But three end-to-end attempts against a real `@modelcontextprotocol/sdk` TS server still returned `"Client does not support sampling tools capability"`. Either the TS SDK has stricter validation than checking the key exists, OR the running daemon hadn't actually been replaced by the freshly-built binary at verification time (see `pipeline-workflow-drivers.md` daemon-redeploy verification). Both possibilities require a real end-to-end test — spawning the TS server as a stdio child of the daemon and asserting `createMessage` round-trips — before claiming the wire format works in practice.

	Rule: when a test name promises end-to-end behavior (`*_works`, `*_succeeds`, `*_round_trips`), the test must exercise the same transport + the same peer implementation production uses. Otherwise the test name is overclaiming.
