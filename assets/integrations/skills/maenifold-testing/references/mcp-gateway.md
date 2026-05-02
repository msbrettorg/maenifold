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
