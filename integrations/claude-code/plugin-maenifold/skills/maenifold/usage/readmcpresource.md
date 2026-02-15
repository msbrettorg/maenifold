# ReadMcpResource

Reads MCP resource content by URI. Provides CLI-accessible access to the asset catalog and individual asset definitions (workflows, roles, colors, perspectives).

## Parameters

- `uri` (string, required): Resource URI. Supported formats:
  - `asset://catalog` — full catalog of all asset types and their metadata
  - `asset://workflows/{id}` — a specific workflow definition (JSON)
  - `asset://roles/{id}` — a specific role definition (JSON)
  - `asset://colors/{id}` — a specific color definition (JSON)
  - `asset://perspectives/{id}` — a specific perspective definition (JSON)

## Returns

JSON content of the requested resource. For `asset://catalog`, returns a combined overview of all available assets. For individual assets, returns the full JSON definition.

## Examples

### Read the full asset catalog

```json
{
  "uri": "asset://catalog"
}
```

### Read a specific workflow

```json
{
  "uri": "asset://workflows/agentic-research"
}
```

### Read a specific role

```json
{
  "uri": "asset://roles/product-manager"
}
```

## Common Patterns

- **Inspect before run**: `ReadMcpResource { "uri": "asset://workflows/think-tank" }` → review steps → `Workflow { "workflowId": "think-tank" }`
- **Discovery flow**: `ListAssets { "type": "workflow" }` → pick an ID → `ReadMcpResource { "uri": "asset://workflows/{id}" }` → read full definition
- **Catalog overview**: `ReadMcpResource { "uri": "asset://catalog" }` → browse all available assets in one call

## Errors

- `URI is required` → provide a non-empty `uri` parameter
- `Invalid resource URI format: X` → URI must match `asset://type/id` or `asset://catalog`
- `Unknown resource type: X` → valid types in URI path are `workflows`, `roles`, `colors`, `perspectives`

## Integration

- **ListAssets**: Discover asset IDs before reading full content
- **Adopt**: Read role/color/perspective definitions to understand what they configure
- **Workflow**: Read workflow definitions to understand step structure before starting
