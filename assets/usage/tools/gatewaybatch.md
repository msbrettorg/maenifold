# GatewayBatch

Submits multiple exact gateway commands sequentially or reads bounded in-memory
batch status.

## Parameters

- `operation` (string, required): Batch operation. Supported operations are
  determined by the running gateway implementation.
- `calls` (array, optional): Commands to execute. Each item contains
  `domain`, `command`, and `parameters`.
- `continueOnError` (bool, optional): Whether execution continues after an item
  fails.
- `batchId` (string, optional): Existing batch identifier for status-style
  operations.

## Item shape

```json
{
  "domain": "maenifold.memory",
  "command": "SearchMemories",
  "parameters": { "query": "authentication", "mode": "Hybrid" }
}
```

## Notes

`GatewayBatch` is deterministic only. It batches exact commands; it does not
route natural language and does not replace `GatewayIntent`.
