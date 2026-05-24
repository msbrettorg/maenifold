# LSP Reference

One LSP server is configured in `.github/lsp.json` (repo-level) and `~/.copilot/lsp-config.json` (user-level): `rust-analyzer`. Config reference: [Adding LSP servers for Copilot CLI](https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/add-lsp-servers).

## Rust (`rust-analyzer`)

**Status:** Fully operational. All LSP operations work including cross-crate `workspaceSymbol`.

| Operation | Works | Notes |
|---|---|---|
| `workspaceSymbol` | ✅ | Finds symbols across all crates |
| `documentSymbol` | ✅ | |
| `goToDefinition` | ✅ | Cross-crate |
| `findReferences` | ✅ | Cross-crate |
| `hover` | ✅ | Full type + size info |
| `incomingCalls` | ✅ | |
| `outgoingCalls` | ✅ | |
| `rename` | ✅ | |

## Config Fields

| Field | What it does | Where documented |
|---|---|---|
| `rootUri` | Workspace root relative to git root. | [Copilot CLI LSP docs](https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/add-lsp-servers) |
| `requestTimeoutMs` | Per-request timeout in ms. Default: 90000. | Copilot CLI LSP docs |
| `initializationTimeoutMs` | Server startup timeout. | Copilot CLI LSP docs |
| `fileExtensions` | Maps extensions to language IDs. | Copilot CLI LSP docs |

## Agent Guidance

1. **Prefer LSP over grep** for symbol navigation. `documentSymbol`, `hover`, `findReferences`, `goToDefinition`, `workspaceSymbol` are all reliable in rust-analyzer.
2. **If an LSP call times out**, the server is still loading. Retry next turn — don't fall back to grep unless the server is genuinely broken.
3. **Manage with `/lsp`.** `/lsp show` to check status, `/lsp reload` after config changes, `/lsp test rust-analyzer` to verify.
