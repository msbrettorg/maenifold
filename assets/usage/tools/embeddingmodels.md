# EmbeddingModels

Manages embedding models used by the vector index.

## Parameters

- `action` (string, required): `status`, `list`, `download`, `activate`,
  `delete`, or `regenerate`.
- `modelId` (string, optional): Catalog ID such as `bge-small-en-v1.5`.
  Required for `activate` and `delete`.
- `trustExternalCatalog` (bool, optional): Explicitly trust an external
  `{assets_path}/models/catalog.json` override.
- `allowRestrictedLicense` (bool, optional): Allow download/activation for
  non-MIT/Apache model licenses.
- `learn` (bool, optional): Return help instead of executing.

## Examples

```json
{ "action": "status" }
```

```json
{ "action": "activate", "modelId": "bge-small-en-v1.5" }
```

## Domain

MCP clients invoke this as `GatewayExecute` with domain `maenifold.system` and
command `EmbeddingModels`.
