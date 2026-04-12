# Pipeline

Creates and manages pipeline sessions composed of DAG nodes.

## Parameters

- `name` (string, optional): Name prefix for the pipeline session ID. Format: `{name}-{epoch}`. Defaults to `pipeline`. Must start with a letter; alphanumeric and hyphens only. Use to categorize runs (e.g. `sleep`, `benchmark`, `research`).
- `sessionId` (string, optional): Existing pipeline session identifier for lifecycle, topology, or inspection operations.
- `definition` (object, optional): Pipeline definition to create. Must include `nodes` and `edges`.
- `maxConcurrency` (integer, optional, default: `8`): Maximum number of nodes the scheduler may activate concurrently.
- `prompt` (string, optional): Ambient prompt passed to all agent-dispatched nodes during autonomous execution.
- `inputs` (object, optional): Optional runtime inputs visible to pipeline execution. Runtime values override persisted definition inputs.
- `list` (bool, default: false): Return all pipeline sessions with status summaries.
- `forceComplete` (bool, default: false): Admin override to mark a pipeline completed. The pipeline must not be actively running.
- `learn` (bool, default: false): Return this help text.

## Create

Provide `definition` to validate the directed graph, create a persistent pipeline session, and pre-create one workflow session for every node.

Use `name` to set a prefix for the session ID: `{name}-{epoch}`. This enables natural run versioning — e.g. `ls sleep-*` to list all sleep cycle runs.

### Definition shape

```json
{
  "templateId": "research-template-v1",
  "nodes": [
    { "id": "root", "type": "workflow", "displayLabel": "Root", "workflowId": "workflow-a" },
    { "id": "child", "type": "workflow", "displayLabel": "Child", "workflowId": "workflow-b", "acpClient": "agent-b" },
    {
      "id": "draft",
      "type": "prompt",
      "displayLabel": "Draft",
      "prompt": "Summarize {{input}}",
      "agent": "agent-c",
      "toolMode": "disabled",
      "agentConfig": {
        "cwd": "/workspace",
        "model": "gpt-5.4-mini",
        "mode": "prompt",
        "thoughtLevel": "medium",
        "configOptions": {
          "temperature": "0.2"
        }
      }
    },
    {
      "id": "analyze",
      "type": "sequentialThinking",
      "displayLabel": "Analyze",
      "topic": "Evaluate tradeoffs",
      "agent": "agent-d",
      "totalThoughts": 5,
      "agentConfig": {
        "cwd": "/workspace",
        "model": "gpt-5.4-mini",
        "mode": "think",
        "thoughtLevel": "high",
        "configOptions": {
          "temperature": "0.1"
        }
      }
    },
    {
      "id": "publish",
      "type": "tool",
      "displayLabel": "Publish",
      "toolName": "pipeline",
      "toolArgs": {},
      "agentConfig": {
        "cwd": "/workspace",
        "model": "gpt-5.4-mini",
        "mode": "prompt",
        "thoughtLevel": "low",
        "configOptions": {
          "temperature": "0.0"
        }
      }
    }
  ],
  "edges": [
    { "source": "root", "target": "child", "type": "success" },
    { "source": "child", "target": "draft", "type": "success" },
    { "source": "draft", "target": "analyze", "type": "success" },
    { "source": "analyze", "target": "publish", "type": "success" }
  ]
}
```

Optional definition fields:

- `templateId` (string, optional): Template identifier preserved across pipeline round-trips.

Supported node shapes:

- Workflow: `{ "id": "...", "type": "workflow", "displayLabel": "...", "workflowId": "...", "acpClient": "..." }`
- Prompt: `{ "id": "...", "type": "prompt", "displayLabel": "...", "prompt": "...", "agent": "...", "toolMode": "disabled", "agentConfig": { "cwd": "...", "model": "...", "mode": "...", "thoughtLevel": "...", "configOptions": {} } }`
- SequentialThinking: `{ "id": "...", "type": "sequentialThinking", "displayLabel": "...", "topic": "...", "agent": "...", "totalThoughts": 5, "agentConfig": { "cwd": "...", "model": "...", "mode": "...", "thoughtLevel": "...", "configOptions": {} } }`
- Tool: `{ "id": "...", "type": "tool", "displayLabel": "...", "toolName": "...", "toolArgs": {}, "agentConfig": { "cwd": "...", "model": "...", "mode": "...", "thoughtLevel": "...", "configOptions": {} } }`

`displayLabel` is optional on all node types and is preserved for presentation-only UI surfaces.

`agentConfig` is optional on Prompt, SequentialThinking, and Tool nodes. Supported fields:

- `cwd` (string, optional): Working directory for the agent session.
- `model` (string, optional): Model override for the agent session.
- `mode` (string, optional): Session mode override for the agent session.
- `thoughtLevel` (string, optional): Thought level override for the agent session.
- `configOptions` (object, optional): Arbitrary config options forwarded to the agent session.

### Returns

- `pipelineId`: Created pipeline session identifier.
- `ready`: Initial ready node IDs.
- `nodes`: Array of `{ nodeId, workflowId, sessionId }`.

## Lifecycle operations

- `sessionId + run=true (+ prompt)`: Start autonomous execution for an existing pipeline session. When provided, `prompt` is passed through as ambient context for agent-dispatched nodes.
- `list=true`: Return all pipeline sessions with status summaries.
- `sessionId + cancel=true`: Cancel a running pipeline and prevent further work from starting.
- `sessionId + view=true`: Inspect the current pipeline state.
- `sessionId + dispatch=true`: Claim ready nodes as active and return them.
- `sessionId + nodeId + restart=true`: Recreate an abandoned or stuck node session.
- `sessionId + nodeId + status`: Update a node to `completed` or `abandoned`.
- `sessionId + forceComplete=true`: Mark the pipeline completed without executing additional nodes. The pipeline must not be actively running.

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
- Running pipelines with active nodes cannot be deleted until they are cancelled.

## Errors

- Empty node lists
- Duplicate node IDs
- Unknown edge references
- Cycles in the pipeline graph
- Missing workflow definitions
- Invalid `maxConcurrency` values
- Invalid `name` (must start with a letter, alphanumeric and hyphens only)
- Invalid lifecycle/topology parameter combinations
