# Pipeline

Creates and manages pipeline sessions composed of workflow nodes.

## Parameters

- `sessionId` (string, optional): Existing pipeline session identifier for lifecycle, topology, or inspection operations.
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

## Lifecycle operations

- `sessionId + run=true`: Start autonomous execution for an existing pipeline session.
- `sessionId + pause=true`: Pause a running pipeline after active work drains.
- `sessionId + resume=true`: Resume a paused pipeline.
- `sessionId + cancel=true`: Cancel a running pipeline and prevent further work from starting.
- `sessionId + view=true`: Inspect the current pipeline state.
- `sessionId + dispatch=true`: Claim ready nodes as active and return them.
- `sessionId + nodeId + restart=true`: Recreate an abandoned or stuck node session.
- `sessionId + nodeId + status`: Update a node to `completed` or `abandoned`.

## Topology operations

- `sessionId + rewind`: Reset the provided `nodeIds` and every downstream node.

```json
{
  "sessionId": "pipeline-1735689600000",
  "rewind": {
    "nodeIds": ["child"]
  }
}
```

- `sessionId + graft`: Add new nodes and edges to an existing pipeline.

```json
{
  "sessionId": "pipeline-1735689600000",
  "graft": {
    "nodes": [
      { "id": "cleanup", "workflowId": "workflow-cleanup" }
    ],
    "edges": [
      { "source": "child", "target": "cleanup", "type": "always" }
    ]
  }
}
```

- `sessionId + join`: Merge another pipeline into the target pipeline with linking edges.

```json
{
  "sessionId": "pipeline-target",
  "join": {
    "sourceSessionId": "pipeline-source",
    "edges": [
      { "source": "target-root", "target": "source-root" }
    ]
  }
}
```

- `sessionId + fork`: Clone the selected node and downstream subgraph into a new pipeline session. `newSessionId` is optional.

```json
{
  "sessionId": "pipeline-1735689600000",
  "fork": {
    "fromNodeId": "child",
    "newSessionId": "pipeline-forked"
  }
}
```

## Delete

- `sessionId + delete=true`: Permanently delete the pipeline session and all associated workflow sessions.
- Running pipelines with active nodes cannot be deleted until they are paused or cancelled.

## Errors

- Empty node lists
- Duplicate node IDs
- Unknown edge references
- Cycles in the pipeline graph
- Missing workflow definitions
- Invalid `maxConcurrency` values
- Invalid lifecycle/topology parameter combinations
