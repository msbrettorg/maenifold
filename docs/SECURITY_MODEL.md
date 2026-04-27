# Security Model

maenifold is a local-only, single-user context engineering infrastructure built in Rust. It exposes two interfaces: an MCP server over stdio and a CLI. There is no network listener, no authentication, and no multi-user access control. The trust boundary is the OS user account.

This document describes what the security model **is**, not what it should be.

## Data Flow Diagram

```
External Entities                             maenifold Process                          Persistent Storage
─────────────────                             ─────────────────                          ──────────────────

[npm Registry] ──┐
                 ├──► binary + native libs + ONNX model ─► (no code signing)
[Homebrew /     ─┘
 GitHub Rel.] ────► binary + SHA-256 checksums ─────────► (no SLSA provenance)

[MCP Client] ─────► JSON-RPC via stdio ────► [MCP Server] ──┐
                                                            │
[CLI User]   ─────► tool <name> <payload> ──────► [CLI Parser] ──┤
                                                            ▼
[Env Vars    ─────► paths, config (no ──► (consumed by process; no validation
 MAENIFOLD_*]       validation beyond     beyond type-safe parsing)
                    type-safe parsing)
                                                  │
                                                  ▼
                                          [ToolRegistry — 28 tools]
                                                  │
              ┌───────────────────┬───────────────┼──────────────┬──────────────┐
              ▼                   ▼               ▼              ▼              ▼
       [MarkdownIO]       [SearchEngine]    [DecayCalc]   [graph queries  parameterized]
        read/write +      FTS5 + vector +                       │
        path validation   RRF fusion                            ▼
              │                   │                       ┌─────────────┐
              ▼                   ▼                       │  SQLite     │
       ┌──────────────┐    queries to ─────────────────► │  memory.db  │
       │ Markdown     │                                  │  (concepts, │
       │ Files        │    [IncrementalSync] ──watch──► │  file_search│
       │ ~/maenifold/ │◄──────┘   │                      │  FTS5,      │
       │ memory/      │           ├──upsert content──►  │  graph,     │
       └──────────────┘           │   + concepts         │  vec_*)     │
                                  ▼                      └─────────────┘
                              [ONNX Runtime] ──384-dim──► (above)
                                  ▲                          ▲
                          loaded in-process              loaded as native
                          (no integrity check)           ext (no integrity)
                                  │                          │
                          ┌───────┴───────┐           ┌──────┴──────┐
                          │ ONNX Model    │           │ sqlite-vec  │
                          │ all-MiniLM-   │           │ vec0.{dylib,│
                          │ L6-v2.onnx    │           │ so, dll}    │
                          └───────────────┘           └─────────────┘

       [AssetWatcher] ─watch *.json──► ~/maenifold/assets/
                     ─resource notifications──► [MCP Server]
```

Trust-boundary highlights visible in the flow above:

- **Distribution → process**: binaries arrive via npm or Homebrew with no code signing and no SLSA provenance.
- **MCP client / CLI user → process**: requests cross stdio or shell args; there is no authentication.
- **Process → filesystem**: `MarkdownIO` validates paths before reading or writing markdown.
- **Process → SQLite**: all queries are parameterized; embeddings and graph data live in `memory.db`.
- **Process → native deps**: ONNX model and `sqlite-vec` are loaded in-process with no integrity verification.

### Threat Flow Table

| Category | From | To | Security Note |
|----------|------|-----|---------------|
| Distribution | npm Registry | maenifold process (Binary + native libs + ONNX model) | No code signing |
| Distribution | Homebrew / GitHub Releases | maenifold process (Binary + SHA-256 checksums) | No SLSA provenance |
| User Environment | Environment Variables | maenifold process (Paths, config) | Type-safe parsing only |
| User Environment | MCP Client (Claude, Desktop) | MCP Server (stdio JSON-RPC) | No authentication |
| User Environment | CLI User (shell) | CLI Parser (tool \<name\> \<payload\> args) | No authentication |
| Internal Process | MCP Server | ToolRegistry (Dispatch to 28 tools) | — |
| Internal Process | CLI Parser | ToolRegistry (Dispatch to 28 tools) | — |
| Internal Process | ToolRegistry | MarkdownIO (Read/write operations) | Path validated |
| Internal Process | ToolRegistry | SearchEngine (FTS5 + vector queries) | — |
| Internal Process | ToolRegistry | SQLite memory.db (Graph queries) | Parameterized |
| Internal Process | ToolRegistry | DecayCalculator (Decay calculations) | — |
| File System | MarkdownIO | Markdown Files (~/maenifold/memory/, Read/write .md) | Path validated |
| File System | IncrementalSync | Markdown Files (Watch *.md changes) | — |
| File System | IncrementalSync | SQLite memory.db (Upsert content + concepts) | — |
| File System | IncrementalSync | ONNX Runtime (Embedding generation) | — |
| File System | AssetWatcher | Asset Files (~/maenifold/assets/, Watch *.json changes) | — |
| File System | AssetWatcher | MCP Server (Resource notifications) | — |
| Search & Storage | SearchEngine | SQLite memory.db (FTS5 + vector queries) | — |
| Search & Storage | ONNX Runtime | SQLite memory.db (384-dim embeddings) | — |
| Native Dependencies | ONNX Model (all-MiniLM-L6-v2.onnx) | ONNX Runtime (Loaded in-process) | No integrity check |
| Native Dependencies | sqlite-vec (vec0.dylib/so/dll) | SQLite (Loaded as native extension) | No integrity check |

## Trust Boundaries

| Boundary | Components | Trust Level | Controls |
|---|---|---|---|
| Distribution to Machine | npm, Homebrew, GitHub Releases | Implicit trust in registry | SHA-256 checksums (Homebrew only). No code signing, no SLSA provenance |
| Environment to Process | Env vars (`MAENIFOLD_ROOT`, `MAENIFOLD_DATABASE_PATH`) | Any local process can set | No validation beyond type-safe parsing |
| MCP Client to Process | stdio pipe | Parent process (1:1) | No auth (by MCP spec). JSON depth limit=32 |
| CLI to Process | Shell args | Any local user | No auth. JSON depth limit=32 |
| Process to Filesystem | `MarkdownIO` | OS file permissions | Path traversal: `UriToPath` prefix check, `ValidatePathSecurity`, `SanitizeUserInput` (URL-decode first), `Slugify` |
| Process to SQLite | Parameterized queries | In-process | Prepared statements throughout. WAL mode. Foreign keys. Read-only connections for queries |
| Process to ONNX | In-process native code | Bundled model | No integrity verification. Model path from assets directory |
| Process to sqlite-vec | Native extension loading | Bundled native lib | No integrity verification. `load_extension_enable()` |

## STRIDE Threat Analysis

### Noteworthy (document, don't panic)

**1. Indirect Prompt Injection via Memory Content**

Memory content flows into LLM context. This is architecturally inherent to all MCP tools -- the `session_start` hook auto-injects recent memory. Mitigation: memory content is as trusted as the user who wrote it. This is the design intent, not a vulnerability.

**2. ONNX Model Integrity**

No hash verification when loading the `.onnx` model. ONNX models can contain custom operators that execute arbitrary code. Bounded risk: the model file lives in the assets directory (the installed binary directory), not in user-writable `~/maenifold/`. Exploiting this requires write access to the installation directory.

**3. sqlite-vec Native Extension Integrity**

Same situation as the ONNX model. The native library (`vec0.dylib`/`so`/`dll`) is loaded from the binary directory without verification. Same bounded risk: requires write access to the installation directory.

**4. MAENIFOLD_ROOT Redirection**

The `MAENIFOLD_ROOT` environment variable redirects all data operations. Any process running as the user can set this. Bounded risk: if an attacker controls your environment variables, they already control your shell.

### Accepted Risks (by design)

- **No authentication.** Single-user local tool. The OS handles access control.
- **No encryption at rest.** User's home directory, user's data. Volume-level encryption is the user's choice.
- **No audit logging.** Single-user system. Nothing to audit.
- **No network exposure.** stdio only. No listening sockets.

### Well-Mitigated

- **Path traversal:** 4-layer defense -- URL-decode, sanitize, validate, canonicalize+prefix-check.
- **SQL injection:** Parameterized queries throughout. No string concatenation of user input into SQL.
- **JSON DoS:** JSON depth limit of 32 enforced on all deserialization.
- **WikiLink regex:** Negative lookbehind/lookahead rejects malformed triple/quadruple brackets. Markdig depth limit as fallback.

## Security Controls Reference

| Control | Location |
|---|---|
| JSON depth limits | `crates/maenifold-core/src/util/json.rs` |
| `sanitize_user_input` (URL-decode, strip dangerous chars, collapse traversal) | `crates/maenifold-core/src/memory/` |
| `validate_path_security` (block absolute paths, dot sequences) | `crates/maenifold-core/src/util/paths.rs` |
| URI-to-path traversal check (canonicalize + prefix match) | `crates/maenifold-core/src/markdown/` |
| Sanitize frontmatter (strip embedding fields) | `crates/maenifold-core/src/markdown/` |
| Parameterized queries (rusqlite named parameters) | `crates/maenifold-core/src/db/` |
| WikiLink regex hardening (negative lookaround) | `crates/maenifold-core/src/markdown/` |
| Checksum concurrency guard | `crates/maenifold-core/src/markdown/` |
| `MoveMemory` boundary check (canonicalize + prefix match) | `crates/maenifold-core/src/memory/` |
| `slugify` (strip non-alphanumeric, collapse hyphens) | `crates/maenifold-core/src/markdown/` |
| Read-only SQLite connections (`query_only=ON`) | `crates/maenifold-core/src/db/` |

## Design Philosophy

maenifold's security model follows from what it is: a local tool that runs as you, reads your files, and talks to your LLM client over a pipe.

**The OS is the security boundary.** There is no authentication because there is no one to authenticate. If you can run the binary, you are the user. If you are the user, you own the data. Adding auth would be security theater.

**Markdown files are the source of truth.** SQLite is a regenerable cache built from the markdown. The security controls on filesystem access (path traversal prevention) matter more than the database controls, because the files are the durable state.

**Defense-in-depth where it counts.** Path traversal gets four layers of defense because that is the one boundary where user-supplied input meets the filesystem. Everything else either has no user-facing surface (ONNX loading) or is handled by parameterized queries (SQLite).

**No encryption because the user owns the disk.** maenifold stores data in `~/maenifold/`. If you want encryption, encrypt the volume. Implementing application-level encryption for a single-user local tool would be complexity without security gain.

**This is not a server.** No listening sockets, no multi-tenancy, no shared state, no network protocol. The threat model is commensurately narrow.
