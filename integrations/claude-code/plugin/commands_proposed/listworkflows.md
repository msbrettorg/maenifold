---
name: ma:listworkflows
description: DEPRECATED - Use ListAssets and ReadMcpResource instead
---

# ListWorkflows (DEPRECATED)

**Removed.** Use MCP resource tools.

## Replacement

**List available asset types (workflows, roles, colors, perspectives):**
```json
ListAssets → returns ["workflow", "role", "color", "perspective"]
```

**List workflows only:**
```json
ListAssets → { "type": "workflow" }
```

**Get specific workflow:**
```json
ReadMcpResource → { "uri": "asset://workflows/design-thinking" }
```

## Migration

```diff
- ListWorkflows → workflow list
+ ListAssets → list asset types or type-specific metadata
+ ReadMcpResource → fetch by URI
```
