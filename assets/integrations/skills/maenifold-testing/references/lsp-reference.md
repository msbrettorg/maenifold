# LSP Reference

Three LSP servers are configured in `.github/lsp.json` (repo-level) and `~/.copilot/lsp-config.json` (user-level). Config reference: [Adding LSP servers for Copilot CLI](https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/add-lsp-servers).

## Monorepo Layout

The TS/Vue frontend lives in `apps/desktop/`, not at the git root. Both servers use `rootUri: "apps/desktop"` to point at the subdirectory containing `tsconfig.json`. Without this, the servers start in the git root, find no tsconfig, and fail silently.

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

## TypeScript (`typescript-language-server`)

**Status:** File-level operations work. `workspaceSymbol` is unreliable.

| Operation | Works | Notes |
|---|---|---|
| `documentSymbol` | ✅ | Full symbol tree for any `.ts` file |
| `hover` | ✅ | Type signatures, docs |
| `goToDefinition` | ✅ | Cross-file, into `node_modules` |
| `findReferences` | ✅ | Cross-file |
| `rename` | ✅ | |
| `workspaceSymbol` | ⚠️ | Returns empty until tsserver finishes async project loading |

**Known limitation:** `typescript-language-server` loads the project asynchronously. `workspace/symbol` returns empty until initialization completes (signaled by `$/progress` which Copilot CLI does not surface). See [typescript-language-server#948](https://github.com/typescript-language-server/typescript-language-server/issues/948). File-level operations work immediately.

**Workaround:** Use `documentSymbol` on the target file, or start from a known import and use `goToDefinition` to navigate. `findReferences` from any known symbol gives the cross-file view.

## Vue (`vue-language-server`)

**Status:** Requires warm-up time. Large projects may exceed default timeout.

| Operation | Works | Notes |
|---|---|---|
| `documentSymbol` | ⚠️ | Works after project loads; may timeout on first request |
| `hover` | ⚠️ | Same warm-up caveat |
| `goToDefinition` | ⚠️ | Same warm-up caveat |
| `findReferences` | ⚠️ | Same warm-up caveat |
| `workspaceSymbol` | ⚠️ | Same async loading limitation as TS |

Config includes `requestTimeoutMs: 180000` (3 min). The `@vue/language-server` version must match `vue-tsc` — both pinned at `3.2.7`.

## Config Fields

| Field | What it does | Where documented |
|---|---|---|
| `rootUri` | Workspace root relative to git root. Required for monorepos. | [Copilot CLI LSP docs](https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/add-lsp-servers) |
| `initializationOptions.tsserver.path` | TS location for `typescript-language-server`. Relative to `rootUri`. | [typescript-language-server config](https://github.com/typescript-language-server/typescript-language-server/blob/main/docs/configuration.md) |
| `initializationOptions.typescript.tsdk` | TS location for `vue-language-server`. Relative to `rootUri`. | Vue Language Server docs |
| `requestTimeoutMs` | Per-request timeout in ms. Default: 90000. | Copilot CLI LSP docs |
| `initializationTimeoutMs` | Server startup timeout. | Copilot CLI LSP docs |
| `fileExtensions` | Maps extensions to language IDs. | Copilot CLI LSP docs |

## Agent Guidance

1. **Prefer file-level operations.** `documentSymbol`, `hover`, `findReferences`, `goToDefinition` are reliable across all three servers.
2. **Use `workspaceSymbol` only for Rust.** For TS/Vue, navigate from known entry points.
3. **If an LSP call times out**, the server is still loading. Retry next turn — don't fall back to grep unless the server is genuinely broken.
4. **Manage with `/lsp`.** `/lsp show` to check status, `/lsp reload` after config changes, `/lsp test <server>` to verify.
