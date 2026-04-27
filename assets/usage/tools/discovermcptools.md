# DiscoverMcpTools

Discovers tools exposed by configured MCP servers.

## Parameters

No parameters.

## Returns

An array of discovery results. Gateway-managed stdio servers are reported from
the live gateway pool. Non-gateway MCP servers are probed directly with the
default discovery timeout.

## Notes

This is a public daemon tool because it needs live daemon configuration. It is
not a first-party maenifold domain command.
