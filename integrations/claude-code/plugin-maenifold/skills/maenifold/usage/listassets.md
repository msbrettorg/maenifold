# ListAssets

Lists available asset types or the metadata for a specific asset type. Use to discover workflows, roles, colors, and perspectives before calling `Adopt` or `Workflow`.

## Parameters

- `type` (string, optional): Asset type to list: `"workflow"`, `"role"`, `"color"`, `"perspective"`. Omit to list all available asset types.

## Returns

- Without `type`: JSON array of available asset type names (`["workflow", "role", "color", "perspective"]`).
- With `type`: JSON array of asset metadata objects for that type, including `id`, `name`, `description`, and type-specific fields.

## Examples

### List all asset types

```json
{}
```

Returns: `["workflow", "role", "color", "perspective"]`

### List available workflows

```json
{
  "type": "workflow"
}
```

Returns metadata for all workflows under `assets/workflows/`.

### List available roles

```json
{
  "type": "role"
}
```

Returns metadata for all roles under `assets/roles/`.

## Common Patterns

- **Discover before adopt**: `ListAssets { "type": "role" }` → find the right role → `Adopt { "type": "role", "identifier": "..." }`
- **Discover before workflow**: `ListAssets { "type": "workflow" }` → find the right workflow → `Workflow { "workflowId": "..." }`
- **Full catalog**: Use `ReadMcpResource { "uri": "asset://catalog" }` for a combined view of all asset types.

## Errors

- `Unknown asset type 'X'` → valid types are `workflow`, `role`, `color`, `perspective`

## Integration

- **Adopt**: Discover roles/colors/perspectives before adopting
- **Workflow**: Discover workflow IDs before starting
- **ReadMcpResource**: Read full asset content by URI after discovering via ListAssets
