# Backlog — maenifold-rs

For completed work (Sprints 1–3, 9–15, 20–22), see [CHANGELOG.md](CHANGELOG.md).

---

## Sprint 16 — Storage Abstraction + PostgreSQL Backend

### T-16.1: Define operation-oriented `trait DbBackend` and `SqliteBackend` wrapper | RTM: FR-22

Introduce `trait DbBackend` in `crates/maenifold-core/src/db/backend.rs` using an **operation-oriented** design — trait methods map to logical data operations (e.g., `upsert_file_content()`, `query_concepts()`, `vector_search()`, `graph_traverse()`), NOT to connection handles. Callers never see a `Connection` or `Pool` — they call domain operations and get results. This ensures a third backend (e.g., Cosmos DB, in-memory, remote API) is a clean trait impl addition, not a rewrite.

**Design principles:**

- Methods grouped by domain: file content, graph, vector search, sync, maintenance
- All methods take `&self` + domain-specific params, return `Result<DomainType>`
- No `connection()` method — backend owns its connection lifecycle internally
- `SqliteBackend` wraps existing `rusqlite` code internally, exposing only domain operations. Zero behavioral change.
- `PgBackend` wraps `sqlx::PgPool` internally behind the same trait surface.
- `ToolContext` carries `Arc<dyn DbBackend>` instead of `database_path: PathBuf`

**Files**: `crates/maenifold-core/src/db/backend.rs` (new), `crates/maenifold-core/src/db/mod.rs`, `crates/maenifold-core/src/tools/context.rs`

- **Status**: pending

### T-16.2: Migrate 27 call sites from `open_connection()` to `DbBackend` trait methods | RTM: FR-22

Replace all 27 inline `db::open_connection(&context.config.database_path)` calls + raw SQL with calls to `context.backend.method()` domain operations. Each call site invokes a trait method (e.g., `backend.query_file_content()`, `backend.upsert_concept()`) instead of opening a connection and writing SQL. The `SqliteBackend` impl produces identical results. No behavioral change — domain logic preserved, SQL encapsulated inside backend impls.

- **Files**: `benchmark/mod.rs`, `benchmark/render.rs`, `community/persist.rs`, `decay/metadata.rs`, `graph/analyzer.rs`, `graph/build_context.rs`, `memory/listing.rs`, `memory/support.rs`, `repair/semantics.rs`, `search/api.rs`, `sessions/cleanup.rs`, `similar/mod.rs`, `sync/leader.rs`, `sync/mod.rs`, `sync/watchers.rs`, `tools/diagnostics.rs`, `tools/recent_activity.rs`
- **Status**: pending
- **Depends on**: T-16.1

### T-16.3: Implement `PgBackend` with sqlx + pgvector | RTM: FR-22

Implement `PgBackend` behind `--features postgres`. Uses `sqlx::PgPool` for connection management. Async pool operations wrapped with `tokio::task::block_in_place` to preserve sync `ToolFn` signature. Requires `sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "json", "time"] }` and `pgvector = { version = "0.4", features = ["sqlx"] }`.

- **Files**: `crates/maenifold-core/src/db/pg_backend.rs` (new), `Cargo.toml`
- **Status**: pending
- **Depends on**: T-16.1

### T-16.4: PostgreSQL schema — relational tables | RTM: FR-22

Create PG schema migration for: `file_content`, `concepts`, `concept_mentions`, `concept_graph`, `concept_communities`, `model_metadata`, `sync_leader_lease`. Use `sqlx::migrate!()` macro. Map SQLite types: `INTEGER PRIMARY KEY` → `SERIAL`, `TEXT` → `TEXT`, `REAL` → `DOUBLE PRECISION`, `BLOB` → `BYTEA`. Add `tsvector` generated column on `file_content` for future FTS.

- **Files**: `crates/maenifold-core/migrations/` (new directory), `crates/maenifold-core/src/db/pg_backend.rs`
- **Status**: pending
- **Depends on**: T-16.3

### T-16.5: PostgreSQL schema — pgvector tables + HNSW indexes | RTM: FR-22

Replace vec0 virtual tables with: `vec_concepts (concept_name TEXT PK, embedding VECTOR(N))` and `vec_memory_files (file_path TEXT PK, embedding VECTOR(N))`. Create HNSW indexes with `vector_cosine_ops`. Rewrite 6 `vec_distance_cosine()` query sites to use `<=>` operator.

- **Files**: `crates/maenifold-core/migrations/`, `crates/maenifold-core/src/search/vector.rs`, `crates/maenifold-core/src/similar/vector_query.rs`, `crates/maenifold-core/src/db/pg_backend.rs`
- **Status**: pending
- **Depends on**: T-16.4

### T-16.6: Tests for storage abstraction | RTM: FR-22, NFR-5

Boundary tests: SqliteBackend produces identical results to current code. Feature-gated PG tests using `testcontainers-rs` with PostgreSQL image. Verify: schema init, CRUD on all tables, vector search, concurrent access (NFR-6).

- **Files**: `crates/maenifold-core/tests/backend_tests.rs` (new)
- **Status**: pending
- **Depends on**: T-16.2, T-16.5

## Sprint 17 — Streamable HTTP Transport

### T-17.1: Add rmcp HTTP transport feature + axum dependency | RTM: FR-23

Update workspace `Cargo.toml`: add `transport-streamable-http-server` to rmcp features (gated behind `http` feature flag). Add `axum = "0.7"` and `tower-http = { version = "0.6", features = ["cors"] }` dependencies.

- **Files**: `Cargo.toml`, `crates/maenifold-core/Cargo.toml`
- **Status**: pending

### T-17.2: Implement `serve_http()` in MCP server | RTM: FR-23

Add `MaenifoldMcpServer::serve_http(port: u16)` alongside existing `serve_stdio()`. Uses `StreamableHttpService` from rmcp mounted on axum router. CORS permissive. Session tracking via `Mcp-Session-Id` headers. Same `MaenifoldHandler` used for both transports.

- **Files**: `crates/maenifold-core/src/mcp/http.rs` (new), `crates/maenifold-core/src/mcp/mod.rs`
- **Status**: pending
- **Depends on**: T-17.1

### T-17.3: CLI `--http` flag and `--port` option | RTM: FR-23

Add `--http` and `--port <PORT>` CLI arguments. `--http` starts `serve_http()`. `--mcp` unchanged. Mutually exclusive flags. Default port 8080.

- **Files**: `crates/maenifold-cli/src/main.rs` or `crates/maenifold-core/src/cli.rs`
- **Status**: pending
- **Depends on**: T-17.2

### T-17.4: Polling-based watcher fallback | RTM: FR-26

Add polling mode to watcher: scan `memory_path` on interval using `walkdir` + mtime comparison. Activated by `MAENIFOLD_WATCHER_POLL_SECONDS` env var (default: 0 = disabled, use native). When set, replaces `notify::recommended_watcher` with a timer-driven scan loop. Same `sync_single_file()` incremental logic.

- **Files**: `crates/maenifold-core/src/sync/watchers.rs`
- **Status**: pending

### T-17.5: HTTP transport tests | RTM: FR-23

Integration tests: start HTTP server on random port, send MCP initialize + tool/call requests via reqwest, verify JSON-RPC responses. Test session creation and cleanup.

- **Files**: `crates/maenifold-core/tests/http_transport_tests.rs` (new)
- **Status**: pending
- **Depends on**: T-17.2

## Sprint 18 — Entra ID Authentication

### T-18.1: RFC 9728 protected resource metadata endpoint | RTM: FR-24

Add `GET /.well-known/oauth-protected-resource` route to axum router. Returns JSON with `resource` (server URL), `authorization_servers` (Entra ID tenant URL), `scopes_supported` (["maenifold:tools"]), `bearer_methods_supported` (["header"]). Configurable via env vars: `MAENIFOLD_AUTH_TENANT_ID`, `MAENIFOLD_AUTH_SERVER_URL`.

- **Files**: `crates/maenifold-core/src/auth/metadata.rs` (new), `crates/maenifold-core/src/mcp/http.rs`
- **Status**: pending
- **Depends on**: T-17.2

### T-18.2: JWT validation middleware | RTM: FR-24

Axum middleware layer that validates Bearer tokens from Authorization header. Uses `jsonwebtoken` crate with Entra ID JWKS endpoint (`https://login.microsoftonline.com/{tenant}/discovery/v2.0/keys`). Validates: issuer, audience, expiration, `maenifold:tools` scope. Returns 401 with `WWW-Authenticate` header on failure. Cached JWKS with 1-hour refresh.

- **Files**: `crates/maenifold-core/src/auth/jwt.rs` (new), `crates/maenifold-core/src/auth/mod.rs` (new)
- **Status**: pending
- **Depends on**: T-18.1

### T-18.3: PG managed identity authentication | RTM: FR-24

Use `azure_identity::ManagedIdentityCredential` to acquire token for `https://ossrdbms-aad.database.windows.net/.default`. Pass as password in sqlx `PgConnectOptions`. Handle token refresh (pool `max_lifetime` < 1 hour).

- **Files**: `crates/maenifold-core/src/db/pg_backend.rs`
- **Status**: pending
- **Depends on**: T-16.3

### T-18.4: `--no-auth` bypass flag | RTM: FR-24

Skip JWT validation when `--no-auth` is passed. For local development and testing only. Log warning when active.

- **Files**: `crates/maenifold-core/src/auth/mod.rs`, `crates/maenifold-core/src/cli.rs`
- **Status**: pending
- **Depends on**: T-18.2

### T-18.5: Auth tests | RTM: FR-24

Test: valid token accepted, expired token rejected, wrong audience rejected, missing token returns 401, `--no-auth` bypasses validation, RFC 9728 metadata returned correctly.

- **Files**: `crates/maenifold-core/tests/auth_tests.rs` (new)
- **Status**: pending
- **Depends on**: T-18.2

## Sprint 19 — Deployment + SRE Agent Integration

### T-19.1: Dockerfile | RTM: FR-25

Multi-stage Dockerfile: Rust builder image → debian-slim runtime. Copy binary + assets + ONNX model. Expose port 8080. Health check endpoint. ENV defaults for PG connection, auth tenant, memory path.

- **Files**: `Dockerfile` (new)
- **Status**: pending
- **Depends on**: T-17.2

### T-19.2: Bicep/azd infrastructure templates | RTM: FR-25

Bicep templates for: ACA environment + container app (1 replica, 1 vCPU, 2GB), Azure PG Flexible Server (B1ms, pgvector extension enabled), Azure Files share (mounted at /maenifold/memory), managed identity (Contributor on PG, Storage File Data SMB Share Contributor on Files). azd integration for `azd up` deployment.

- **Files**: `deploy/main.bicep` (new), `deploy/modules/` (new), `azure.yaml` (new)
- **Status**: pending

### T-19.3: SRE Agent connector configuration | RTM: FR-25

Sample YAML for Azure SRE Agent connector registration. Subagent configuration with `mcp_tools: [maenifold-mcp/*]` wildcard. Documentation for connector setup in SRE Agent portal.

- **Files**: `deploy/sre-agent/connector.yaml` (new), `deploy/sre-agent/subagent.yaml` (new), `docs/SRE_AGENT.md` (new)
- **Status**: pending
- **Depends on**: T-19.1

### T-19.4: CI container build workflow | RTM: FR-25

GitHub Actions workflow: trigger on `v*` tags. Build container image with `--features postgres,http`. Push to GitHub Container Registry (ghcr.io). Optionally push to Azure Container Registry if configured.

- **Files**: `.github/workflows/container.yml` (new)
- **Status**: pending
- **Depends on**: T-19.1

### T-19.5: End-to-end deployment test | RTM: FR-25, NFR-6

Deploy to a test ACA environment. Verify: container starts, PG schema initializes, Sync completes, tool calls succeed over HTTP, Entra ID auth works, 5 concurrent clients work.

- **Files**: `deploy/test/e2e.sh` (new)
- **Status**: pending
- **Depends on**: T-19.1, T-19.2, T-18.2


---

## Sprint TBD — ACP Client Orchestration

### T-23.1: Add `agent-client-protocol` crate dependency behind `acp` feature flag | RTM: FR-27 (RTM-162)

Add `agent-client-protocol` to `Cargo.toml` behind `--features acp`. Verify `cargo build` without the feature produces unchanged binary. Wire tokio async runtime (already present).

- **Status**: pending
- **Depends on**: none

### T-23.2: Implement `MaenifoldClient` struct + ACP Client trait | RTM: FR-27 (RTM-159, RTM-160)

Create `crates/maenifold-core/src/acp/client.rs`. Implement `Client` trait: `request_permission` (deny-all default), `session_update` (buffer + validate WikiLinks). Struct carries Config, session reference, permission policy.

- **Status**: pending
- **Depends on**: T-23.1

### T-23.3: Agent subprocess spawner + connection lifecycle | RTM: FR-27 (RTM-157, RTM-158, RTM-161, RTM-163)

Create `crates/maenifold-core/src/acp/process.rs`. Spawn agent as child process with `--acp --stdio`. Wire stdin/stdout to `ClientSideConnection`. Implement `initialize()` + `new_session()` handshake. Kill on shutdown. Verify no MCP server passed in `mcpServers`.

- **Status**: pending
- **Depends on**: T-23.2

### T-23.4: CLI `--orchestrate` mode entry point | RTM: FR-27 (RTM-157, RTM-163)

Add `--orchestrate --agent <program>` to `cli.rs`. Parse args, spawn agent, enter orchestration loop. Single prompt + response for Phase 0 spike.

- **Status**: pending
- **Depends on**: T-23.3

### T-23.5: Permission policy engine | RTM: FR-29 (RTM-170 through RTM-174), NFR-7 (RTM-180)

Create `crates/maenifold-core/src/acp/permissions.rs`. Step-scoped allowlists for fs/read, fs/write, terminal. Memory root writes blocked. All decisions logged. Default deny-all.

- **Status**: pending
- **Depends on**: T-23.2

### T-23.6: Response capture pipeline (3-layer) | RTM: FR-30 (RTM-175 through RTM-178)

Create `crates/maenifold-core/src/acp/capture.rs`. Layer 1: JSONL event log. Layer 2: workflow state updates. Layer 3: semantic compaction with WikiLinks → memory:// notes → incremental sync.

- **Status**: pending
- **Depends on**: T-23.2

### T-23.7: Workflow-driven agent execution loop | RTM: FR-27 (RTM-160), FR-28 (RTM-164 through RTM-169, RTM-181, RTM-191, RTM-192), NFR-7 (RTM-179, RTM-189)

Create `crates/maenifold-core/src/acp/orchestrator.rs`. For each workflow step: assemble graph context, build prompt envelope, send via `conn.prompt()`, capture response, validate WikiLinks, persist, advance state machine. Pause on failure.

- **Status**: pending
- **Depends on**: T-23.3, T-23.5, T-23.6

### T-23.8: Red-team audit — ACP Client traceability + security | RTM: all FR-27 through FR-30, NFR-7

Audit all ACP Client code for: T-\* traceability, permission bypass vectors, recursive MCP loop prevention, agent crash isolation, memory root write blocking.

- **Status**: pending
- **Depends on**: T-23.4, T-23.5, T-23.6, T-23.7

### T-23.9: Blue-team validation — integration tests + end-to-end | RTM: FR-27 through FR-30, NFR-7

Write and run integration tests for all RTM-157 through RTM-180. End-to-end test: workflow with 3 steps, mock ACP agent, verify complete capture pipeline.

- **Status**: pending
- **Depends on**: T-23.4, T-23.7

### T-24.1: Add OpenCode shared runtime config helper + loader-safe exports | RTM: FR-31 (RTM-195 through RTM-200)

Implement `assets/integrations/opencode/plugins/lib/config.ts` and wire `persistence.ts`, `confession.ts`, and `compaction.ts` to consume it under the runtime plugin tree used by the `~/.config/opencode/plugins` symlink.
AC-1: `getMaenifoldPluginEnv(name)` returns `process.env[name]` when explicitly set
AC-2: Unset env values fall back to `~/.config/opencode/opencode.json` → `mcp.maenifold.environment`
AC-3: `getMaenifoldPluginFlag(name)` parses string booleans and returns `true`, `false`, or `undefined` without throwing
AC-4: Missing or malformed OpenCode config fails open and preserves plugin execution
AC-5: `persistence.ts` and `confession.ts` expose only callable plugin factories at module top level while keeping test seams reachable via attached properties
AC-6: `compaction.ts`, `confession.ts`, and `persistence.ts` all resolve shared helper imports successfully from the repo-backed plugin symlink path
AC-7: Focused OpenCode plugin tests pass from `assets/integrations/opencode/tests/plugins`

- **Status**: pending
- **Depends on**: none
