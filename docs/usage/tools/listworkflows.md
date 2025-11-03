# ListWorkflows (DEPRECATED)

**Removed.** Use MCP resource tools.

## Replacement

**List all assets (workflows, roles, colors, perspectives):**
```json
ListMcpResources → returns catalog with metadata
```

**Get specific workflow:**
```json
ReadMcpResource → { "uri": "asset://workflows/design-thinking" }
```

## Migration

```diff
- ListWorkflows → workflow list
+ ListMcpResources → full asset catalog
+ ReadMcpResource → fetch by URI
```
