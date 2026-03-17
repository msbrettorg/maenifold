# Pipeline

Creates and manages pipeline sessions composed of workflow nodes.

## Parameters

- `definition` (object, optional): Pipeline definition to create. Must include `nodes` and `edges`.
- `maxConcurrency` (integer, optional, default: `8`): Maximum number of nodes the scheduler may activate concurrently.
- `learn` (bool, default: false): Return this help text.

## Create

Provide `definition` to validate the directed graph, create a persistent pipeline session, and pre-create one workflow session for every node.

### Definition shape

```json
{
  "nodes": [
    { "id": "root", "workflowId": "workflow-a" },
    { "id": "child", "workflowId": "workflow-b", "acpClient": "agent-b" }
  ],
  "edges": [
    { "source": "root", "target": "child", "type": "success" }
  ]
}
```

### Returns

- `pipelineId`: Created pipeline session identifier.
- `ready`: Initial ready node IDs.
- `nodes`: Array of `{ nodeId, workflowId, sessionId }`.

## Errors

- Empty node lists
- Duplicate node IDs
- Unknown edge references
- Cycles in the pipeline graph
- Missing workflow definitions
- Invalid `maxConcurrency` values

## Example

```json
{
  "definition": {
    "nodes": [
      { "id": "root", "workflowId": "workflow-a" },
      { "id": "child", "workflowId": "workflow-b" }
    ],
    "edges": [
      { "source": "root", "target": "child" }
    ]
  },
  "maxConcurrency": 4
}
```
