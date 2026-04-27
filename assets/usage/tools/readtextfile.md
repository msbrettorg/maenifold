# ReadTextFile

Reads a UTF-8 text file from an absolute path or a `cwd`-relative path.

## Parameters

- `path` (string, required): File path to read.
- `cwd` (string, optional): Current working directory for resolving relative
  paths.

## Example

```json
{
  "path": "README.md",
  "cwd": "/Users/brett/src/ma-collective/maenifold-rs"
}
```

## Domain

MCP clients invoke this as `GatewayExecute` with domain `maenifold.system` and
command `ReadTextFile`.
