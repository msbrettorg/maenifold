# ReadMcpResource

Reads MCP resource content by URI. Provides CLI-accessible access to asset resources (workflows, roles, colors, perspectives).

## Parameters

- `uri` (string, required): Resource URI. Format: `asset://type/id` or `asset://catalog`.

## Returns

Full asset content as JSON. Structure varies by asset type.

**Workflow example:**
```json
{
  "id": "design-thinking",
  "name": "Design Thinking",
  "steps": [ ... ],
  "metadata": { ... }
}
```

**Catalog (`asset://catalog`):**
```json
{
  "workflows": [ ... ],
  "roles": [ ... ],
  "colors": [ ... ],
  "perspectives": [ ... ]
}
```

## Examples

Read full asset catalog:
```json
{
  "uri": "asset://catalog"
}
```

Read a specific workflow:
```json
{
  "uri": "asset://workflows/design-thinking"
}
```

Read a specific role:
```json
{
  "uri": "asset://roles/researcher"
}
```

## URI Format

```
asset://catalog              -- full catalog of all asset types
asset://workflows/{id}       -- specific workflow by ID
asset://roles/{id}           -- specific role by ID
asset://colors/{id}          -- specific color by ID
asset://perspectives/{id}    -- specific perspective by ID
```

## Integration

- **ListAssets**: Discover asset IDs and URIs before reading
- **Workflow**: Read workflow JSON to understand steps before starting
- **Adopt**: Read role/color/perspective content for inspection
