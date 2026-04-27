# AddMcpServer

Adds a configured MCP server entry.

## Parameters

- `entry` (object, required): MCP server entry.

## Entry shapes

```json
{
  "type": "stdio",
  "name": "example",
  "command": "node",
  "args": ["server.js"],
  "gateway": true
}
```

```json
{
  "type": "http",
  "name": "example",
  "url": "http://localhost:8081/mcp"
}
```
