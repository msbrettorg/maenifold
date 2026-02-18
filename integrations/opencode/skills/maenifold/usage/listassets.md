# ListAssets

Lists available asset types or the metadata for a specific asset type.

## Parameters

- `type` (string, optional): Asset type to list. Valid values: `workflow`, `role`, `color`, `perspective`. Omit to list all available types.

## Returns

**Without type parameter** -- returns array of available asset types:
```json
["workflow", "role", "color", "perspective"]
```

**With type parameter** -- returns metadata for all assets of that type:
```json
[
  {
    "id": "design-thinking",
    "name": "Design Thinking",
    "description": "Structured ideation and prototyping methodology",
    "uri": "asset://workflows/design-thinking"
  }
]
```

## Examples

List all asset types:
```json
{}
```

List all workflows:
```json
{
  "type": "workflow"
}
```

List all roles:
```json
{
  "type": "role"
}
```

## Use Cases

- **Discover available assets**: List types first, then drill into a specific type
- **Workflow selection**: Browse workflow metadata before starting a session
- **Adopt preparation**: Find available roles, colors, or perspectives before calling Adopt

## Integration

- **ReadMcpResource**: Fetch full asset content by URI from the metadata results
- **Workflow**: Discover workflow IDs before starting a workflow session
- **Adopt**: Discover role/color/perspective IDs before adopting
