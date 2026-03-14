# Requirements Traceability Matrix — maenifold-rs

For completed requirements (RTM-001 through RTM-114, RTM-147 through RTM-156), see [CHANGELOG.md](CHANGELOG.md).

---

## Hosted Deployment — Dual-Transport, Dual-Backend

### FR-22: Storage Abstraction Layer (`db/backend.rs`)

- □ RTM-115: `trait DbBackend` exists with operation-oriented domain methods (not connection handles) | TC-22.1 | Boundary
- □ RTM-116: `SqliteBackend` wraps existing rusqlite code internally with zero behavioral change | TC-22.2 | Boundary
- □ RTM-117: `PgBackend` connects via `PgPool` with configurable pool size | TC-22.3 | Boundary
- □ RTM-118: `ToolContext` carries `Arc<dyn DbBackend>` instead of `database_path` | TC-22.4 | Boundary
- □ RTM-119: All 27 `open_connection()` call sites migrated to trait domain methods | TC-22.5 | Boundary
- □ RTM-120: PG schema creates equivalent tables: file_content, concepts, concept_mentions, concept_graph, concept_communities, model_metadata, sync_leader_lease | TC-22.6 | Boundary
- □ RTM-121: PG schema uses `pgvector` VECTOR(N) columns replacing vec0 virtual tables | TC-22.7 | Boundary
- □ RTM-122: PG schema uses HNSW index on vector columns | TC-22.8 | Boundary
- □ RTM-123: `vec_distance_cosine()` queries rewritten to pgvector `<=>` operator | TC-22.9 | Boundary
- □ RTM-124: `cargo build` (no features) produces binary identical in behavior to current | TC-22.10 | Boundary+CLI
- □ RTM-125: `cargo build --features postgres` compiles and links sqlx + pgvector | TC-22.11 | Build

### FR-23: Streamable HTTP Transport (`mcp/http.rs`)

- □ RTM-126: `maenifold --http --port 8080` starts axum HTTP server | TC-23.1 | CLI
- □ RTM-127: HTTP server exposes MCP Streamable HTTP endpoints (POST /mcp, GET /mcp, DELETE /mcp) | TC-23.2 | Integration
- □ RTM-128: `maenifold --mcp` continues to start stdio server (unchanged) | TC-23.3 | CLI
- □ RTM-129: HTTP sessions tracked via `Mcp-Session-Id` header | TC-23.4 | Integration
- □ RTM-130: All 28 tools invokable via HTTP transport | TC-23.5 | Integration

### FR-24: Entra ID Authentication (`auth/`)

- □ RTM-131: `GET /.well-known/oauth-protected-resource` returns RFC 9728 metadata | TC-24.1 | Integration
- □ RTM-132: Bearer token in Authorization header validated against Entra ID JWKS | TC-24.2 | Integration
- □ RTM-133: Requests without valid token receive 401 with WWW-Authenticate header | TC-24.3 | Integration
- □ RTM-134: Token audience validated against configured server URL | TC-24.4 | Integration
- □ RTM-135: `--no-auth` flag bypasses authentication for local development | TC-24.5 | CLI
- □ RTM-136: PG connection uses managed identity token (no stored password) | TC-24.6 | Integration

### FR-25: Azure Deployment Manifests (`deploy/`)

- □ RTM-137: Dockerfile produces runnable container image | TC-25.1 | Infrastructure
- □ RTM-138: Bicep/azd templates deploy ACA + PG Flex + Azure Files + managed identity | TC-25.2 | Infrastructure
- □ RTM-139: SRE Agent connector YAML provided with correct endpoint and auth config | TC-25.3 | Infrastructure
- □ RTM-140: CI workflow builds and pushes container image on v\* tags | TC-25.4 | Infrastructure

### FR-26: Polling-Based Watcher Fallback (`sync/watchers.rs`)

- □ RTM-141: `MAENIFOLD_WATCHER_POLL_SECONDS` env var configures polling interval (default 30) | TC-26.1 | Boundary
- □ RTM-142: Polling watcher detects new/changed/deleted .md files | TC-26.2 | Boundary
- □ RTM-143: Polling watcher triggers same incremental sync logic as native watcher | TC-26.3 | Boundary

### NFR-5: Local Mode Preservation

- □ RTM-144: `cargo build` (no postgres/http features) produces identical binary behavior | TC-NFR5.1 | Boundary+CLI

### NFR-6: Concurrent Access

- □ RTM-145: 5+ simultaneous PG connections succeed without contention timeout | TC-NFR6.1 | Load
- □ RTM-146: Concurrent WriteMemory + SearchMemories do not corrupt data | TC-NFR6.2 | Load

---

## ACP Client Orchestration

### FR-27: ACP Client Core (`acp/`)

- □ RTM-157: `maenifold --orchestrate --agent copilot` spawns agent subprocess with `--acp --stdio` | TC-27.1 | Integration
- □ RTM-158: ClientSideConnection performs initialize + new_session handshake | TC-27.2 | Integration
- □ RTM-159: `conn.prompt()` sends content blocks and receives streaming `session_update` | TC-27.3 | Integration
- □ RTM-160: WikiLinks in agent responses are validated before persistence | TC-27.4 | Boundary
- □ RTM-161: Agent process is killed on orchestrator shutdown/cancel | TC-27.5 | Boundary
- □ RTM-162: `cargo build` without `acp` feature produces unchanged binary; `agent-client-protocol` gated behind feature | TC-27.6 | Build
- □ RTM-163: Spawned agent does NOT receive maenifold as MCP server (no recursive loop) | TC-27.7 | Architecture

### FR-28: Workflow-Driven Agent Execution (`acp/orchestrator.rs`)

- □ RTM-164: Workflow step assembles prompt with graph context (FindSimilarConcepts + BuildContext + SearchMemories) | TC-28.1 | Integration
- □ RTM-165: Agent response persisted to workflow session markdown on turn completion | TC-28.2 | Integration
- □ RTM-166: Workflow advances to next step on StopReason::EndTurn | TC-28.3 | Boundary
- □ RTM-167: Workflow pauses on agent failure/cancellation for human review | TC-28.4 | Fault
- □ RTM-168: SequentialThinking submachine receives agent response for thinking steps | TC-28.5 | Integration
- □ RTM-169: Fresh context assembled per step (includes prior step outputs enriching graph) | TC-28.6 | Integration
- □ RTM-181: Prompt envelope contains step definition + graph context + prior outputs + WikiLink requirements | TC-28.7 | Boundary

### FR-29: Permission Policy Engine (`acp/permissions.rs`)

- □ RTM-170: `request_permission` evaluates against step-scoped allowlist with 3 outcomes: auto-approve, prompt-user, deny | TC-29.1 | Boundary
- □ RTM-171: `fs/read_text_file` auto-approves within workspace + memory root, denies outside | TC-29.2 | Security
- □ RTM-172: `fs/write_text_file` blocks writes to memory root (must use WriteMemory); captures pre/post diff | TC-29.3 | Security
- □ RTM-173: `terminal/create` allows only allowlisted commands, denies arbitrary shell | TC-29.4 | Security
- □ RTM-174: All permission decisions, file reads/writes, and terminal executions logged to audit trail | TC-29.5 | Audit
- □ RTM-182: Symlink paths canonicalized before allowlist evaluation | TC-29.6 | Security
- □ RTM-183: Path traversal attempts (../) in agent file requests are denied | TC-29.7 | Security

### FR-30: Response Capture Pipeline (`acp/capture.rs`)

- □ RTM-175: Raw ACP events (message chunks, tool calls, permissions, terminal output) written to JSONL event log per session | TC-30.1 | Integration
- □ RTM-176: Workflow step status, tool call lifecycle, permission outcomes, and terminal exit statuses tracked in real-time | TC-30.2 | Integration
- □ RTM-177: Semantic compaction produces markdown with WikiLinks on turn completion | TC-30.3 | Integration
- □ RTM-178: High-value responses written as `memory://` notes trigger incremental sync | TC-30.4 | Integration

### FR-31: OpenCode Plugin Runtime Config Helper (`assets/integrations/opencode/plugins/`)

- □ RTM-195: `lib/config.ts` resolves `process.env` values before consulting OpenCode config fallback | TC-31.1 | Boundary
- □ RTM-196: Unset plugin env values fall back to `~/.config/opencode/opencode.json` → `mcp.maenifold.environment` | TC-31.2 | Boundary
- □ RTM-197: Missing or malformed OpenCode config fails open without throwing | TC-31.3 | Boundary
- □ RTM-198: `persistence.ts` and `confession.ts` remain plugin-loader safe by exposing only callable plugin factories at module top level | TC-31.4 | Integration
- □ RTM-199: `compaction.ts`, `confession.ts`, and `persistence.ts` share the same helper under the runtime plugin tree | TC-31.5 | Integration
- □ RTM-200: OpenCode plugin tests pass from repo-backed plugin path used by the `~/.config/opencode/plugins` symlink | TC-31.6 | Build+Integration

### NFR-7: Agent Isolation

- □ RTM-179: Agent crash does not corrupt workflow state (pauses for human) | TC-NFR7.1 | Fault
- □ RTM-180: Permission policy defaults to deny-all without explicit allowlist | TC-NFR7.2 | Security
- □ RTM-184: Agent prompt timeout prevents infinite hangs (configurable, default 5 min) | TC-NFR7.3 | Fault
- □ RTM-185: Agent output size bounded to prevent memory exhaustion (10MB/turn, 100MB/log, 1MB/terminal) | TC-NFR7.4 | Fault
- □ RTM-186: Agent binary not found or handshake timeout aborts with error, no session created | TC-27.8 | Fault
- □ RTM-187: Post-EndTurn chunks discarded with warning logged | TC-27.9 | Boundary
- □ RTM-188: Terminal commands tokenized; shell metacharacters denied before allowlist eval | TC-29.8 | Security
- □ RTM-189: Concurrent orchestrators on same workflow session prevented via file lock | TC-NFR7.5 | Fault
- □ RTM-190: Watcher started after agent handshake; sync does not block active turns | TC-27.10 | Integration
- □ RTM-191: SequentialThinking submachine registered programmatically with parentWorkflowId | TC-28.8 | Integration
- □ RTM-192: WikiLink validation failure prevents persistence; workflow pauses | TC-28.9 | Boundary
- □ RTM-193: JSONL logs stored outside memory_root at {maenifold_root}/sessions/ with 0600 permissions | TC-30.5 | Security
- □ RTM-194: Workspace root resolved from CLI/env/cwd, frozen for session lifetime, not parent of maenifold_root | TC-29.9 | Boundary
