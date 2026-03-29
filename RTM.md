# Requirements Traceability Matrix — maenifold-rs

For completed requirements (RTM-001 through RTM-114, RTM-147 through RTM-156), see [CHANGELOG.md](CHANGELOG.md).

## Sprint 63 — Silent Failure Remediation (NFR-3, NFR-4)

| RTM | Requirement | Component | Test Case | Test Type |
|-----|-------------|-----------|-----------|-----------|
| RTM-189 | NFR-3 | `crates/maenifold-core/src/sync/watchers.rs` | TC-189: T-63.1 Watcher health counters — `WatcherHealth` struct, failure tracking in batch and per-file error branches, consecutive failure reset on success, poison-safe mutex access. 10 watcher tests pass. | unit |
| RTM-190 | NFR-4 | `crates/maenifold-core/src/workflow/runner.rs` | TC-190: T-63.2 Regex static compilation — 3 `Lazy<Regex>` statics replace 4 `.expect()` calls, zero runtime panic risk. 11 regex tests pass. | unit |
| RTM-191 | NFR-3 | `crates/maenifold-core/src/markdown/storage.rs` | TC-191: T-63.3 YAML frontmatter warning accumulator — `drain_frontmatter_warnings()` with cap at 100, poison-safe mutex. 36 frontmatter tests pass. | unit |
| RTM-192 | NFR-3 | `crates/maenifold-core/src/search/api.rs` | TC-192: T-63.4 Tag filtering error propagation — 4 `[TAG FILTER]` log sites replace silent `.unwrap_or(false)` | unit |
| RTM-193 | NFR-4 | `crates/maenifold-core/src/sync/{watchers,mod}.rs` | TC-193: T-63.5 Orphan vector cleanup in both watcher batch and full sync paths with `sqlite_master` table guard | integration |

## Sprint 51 — Pipeline Active Executor (FR-40)

| RTM | Requirement | Component | Test Case | Test Type |
|-----|-------------|-----------|-----------|-----------|
| RTM-157 | FR-40 | `crates/maenifold-core/src/workflow/pipeline/engine.rs` | TC-157: petgraph StableGraph replaces hand-rolled DAG; cycle detection, toposort, ready-set all pass existing tests | unit |
| RTM-158 | FR-40 | `crates/maenifold-core/src/acp/driver.rs` | TC-158: AcpX driver spawns acpx subprocess, creates sessions, sends prompts with --format json, parses NDJSON output | integration |
| RTM-159 | FR-40 | `crates/maenifold-core/src/workflow/pipeline/executor.rs` | TC-159: Executor run loop computes ready set, dispatches via JoinSet+Semaphore, cascades completion, respects maxConcurrency. Sprint 56: Rust-native async executor replaced bash bridge. | unit |
| RTM-160 | FR-40 | `crates/maenifold-core/src/workflow/pipeline/supervisor.rs` | TC-160: Lifecycle controls — pause/resume via AtomicBool flag, cancel via CancellationToken, rewind behavior covered by `pipeline_rewind_resets_specified_node_to_pending`, `pipeline_rewind_cascades_downstream`, `pipeline_rewind_rejects_running_pipeline`, and `pipeline_rewind_rejects_unknown_node_id`. Sprint 56: process/signal management removed; lightweight tokio task tracking. | unit |
| RTM-161 | FR-40 | `crates/maenifold-core/src/workflow/pipeline/executor.rs` | TC-161: HSM integration — executor drives Workflow tool per node, spawns SequentialThinking submachines | integration |
| RTM-162 | FR-40 | `crates/maenifold-core/src/workflow/pipeline/handler.rs` | TC-162: Graft adds nodes+edges to existing pipeline, validates no cycles | unit |
| RTM-163 | FR-40 | `crates/maenifold-core/src/workflow/pipeline/handler.rs` | TC-163: Join merges two DAGs using collision-rejection when source and target node IDs overlap; Fork clones subgraph with state reset and ID remapping via `fork-{session}-{id}` prefixes | unit |
| RTM-164 | FR-40 | `crates/maenifold-core/src/workflow/pipeline/handler.rs` | TC-164: Delete archives pipeline session and associated workflow sessions | unit |

## Sprint 53 — File-Authoritative Pipeline Gate Cache (FR-41)

| RTM | Requirement | Component | Test Case | Test Type |
|-----|-------------|-----------|-----------|-----------|
| RTM-165 | FR-41 | `crates/maenifold-gate/src/fs.rs` | TC-165: T-53.1 Filesystem Operations — discover and move task files for file-authoritative pipeline state | unit |
| RTM-166 | FR-41 | `crates/maenifold-gate/src/db.rs` | TC-166: T-53.2 SQLite Cache Layer — CRUD operations for rebuildable `gate.db` cache | unit |
| RTM-167 | FR-41 | `crates/maenifold-gate/src/status.rs` | TC-167: T-53.3 Status Reporting — JSON and text output with optional DB cross-check while disk remains authoritative | unit |
| RTM-168 | FR-41 | `crates/maenifold-gate/src/sync.rs` | TC-168: T-53.4 Cache Rebuild/Reconcile — rebuild and reconcile cache state from disk | integration |
| RTM-169 | FR-41 | `crates/maenifold-gate/src/advance.rs` | TC-169: T-53.5 Stage Advance — validate transition, append canonical section, and move file on terminal pass | integration |
| RTM-170 | FR-41 | `crates/maenifold-gate/tests/{advance_tests,integration}.rs` | TC-170: T-53.6 Integration Tests — 6 cross-module tests; FR-41 implementation in `crates/maenifold-gate/src/{fs,db,status,sync,advance}.rs` is ✅ Complete with 25 tests total (15 unit + 4 advance + 6 integration); integrated TDD skill (`SKILL.md`), PM agent config, scaffold, and deprecated `DISPATCH.md` updates | integration |

## Sprint 54 — Parser Hardening (FR-41)

| RTM | Requirement | Component | Test Case | Test Type |
|-----|-------------|-----------|-----------|-----------|
| RTM-171 | FR-41 | `crates/maenifold-gate/src/parser.rs` | TC-171: T-54.1 Parser Rework Tolerance — prefix heading matching, last-section-wins, implicit verdicts, emoji parsing, no-SWE tolerance; 9 tests in `parser_rework_tests.rs` | unit |
| RTM-172 | FR-41 | `crates/maenifold-gate/src/{types,status,sync}.rs` | TC-172: T-54.2 Location-Authoritative Fallback — `effective_status()` override for done/ files, incomplete trail note, attempts preservation; 2 new integration tests | integration |
| RTM-173 | FR-41 | `crates/maenifold-gate/src/lib.rs` | TC-173: T-54.3 Case-Insensitive Verdict — `to_ascii_uppercase()` normalization at CLI boundary; 4 unit tests | unit |
| RTM-174 | FR-41 | `crates/maenifold-gate/` | TC-174: T-54.4 Zero-Warning Verification — full pipeline clean: 0 sync warnings on 106 files, 45/45 tests pass, sync rebuild stable | integration |

## Sprint 55 — Configurable Pipeline Engine (FR-42)

| RTM | Requirement | Component | Test Case | Test Type |
|-----|-------------|-----------|-----------|-----------|
| RTM-175 | FR-42 | `crates/maenifold-gate/src/pipeline.rs` | TC-175: T-55.1 Pipeline Definition Schema — PipelineDef/StageDef structs, JSON load/validate, cycle detection, reachability check; 8 unit tests | unit |
| RTM-176 | FR-42 | `crates/maenifold-gate/src/db.rs` | TC-176: T-55.2 DB ID Column — extract_task_id with alpha suffix, get_task_by_id, resolve_id_to_file, ID collision guard; 11 unit tests | unit |
| RTM-177 | FR-42 | `crates/maenifold-gate/src/append.rs` | TC-177: T-55.3 Gate Append — logs agent notes without state change, ID resolution, error on unknown ID; 3 unit tests | unit |
| RTM-178 | FR-42 | `crates/maenifold-gate/src/pass.rs` | TC-178: T-55.4 Gate Pass — advances per pipeline JSON, moves to done/ on terminal, shared helpers; 4 tests | integration |
| RTM-179 | FR-42 | `crates/maenifold-gate/src/reject.rs` | TC-179: T-55.5 Gate Reject — rolls back per pipeline JSON, increments attempts, location guard; 5 tests | integration |
| RTM-180 | FR-42 | `crates/maenifold-gate/src/lib.rs` | TC-180: T-55.6 CLI Migration — removed advance, wired pass/reject/append, --pipeline flag, status by ID; 7 CLI parser tests | integration |
| RTM-181 | FR-42 | `crates/maenifold-gate/tests/pipeline_tests.rs` | TC-181: T-55.7 Integration Tests — full pipeline pass-through, reject flows, lightweight pipeline, custom JSON, error cases; 10 integration tests | integration |

## Sprint 56 — Gate Correctness + Skill Alignment

| RTM | Requirement | Component | Test Case | Test Type |
|-----|-------------|-----------|-----------|-----------|
| RTM-182 | FR-41 | `crates/maenifold-gate/src/{append,pass,reject,parser,status,sync}.rs` | TC-182: T-56.1 Canonical State Roundtrip — command sequence artifacts are sufficient for `status` and `sync --rebuild` to derive the same task state from disk and cache | integration |
| RTM-183 | FR-42 | `crates/maenifold-gate/src/{fs,db,sync}.rs` | TC-183: T-56.2 ID + Path Fidelity — nested task paths resolve consistently by ID, file moves are deterministic, and stale cache rows are removed during sync | integration |
| RTM-184 | FR-41 | `crates/maenifold-gate/src/{types,status}.rs` | TC-184: T-56.3 Warning Capture Safety — quiet parsing preserves diagnostics without process-global stderr redirection | unit |
| RTM-185 | FR-41 | `assets/integrations/skills/gate/{SKILL.md,references/**}` | TC-185: T-56.4 Skill Asset Consistency — scaffold, templates, and setup docs match shipped `gate` commands and asset inventory | integration |
| RTM-186 | FR-41 | `crates/maenifold-gate/tests/{integration,pipeline_tests,parser_rework_tests}.rs` | TC-186: T-56.5 Regression Coverage — roundtrip, nested-path, stale-row, and canonical-artifact regressions stay green | integration |

## Sprint 57 — Native Workflow & Think Runners

| RTM | Requirement | Component | Test Case | Test Type |
|-----|-------------|-----------|-----------|-----------|
| RTM-187 | FR-40 | `crates/maenifold-core/src/workflow/runner.rs::run_workflow()` | TC-187: T-57.1 Active workflow step-loop runner — `workflow_runner_completes_multi_step_workflow`, `workflow_runner_stops_on_terminal_success`, `workflow_runner_propagates_agent_failures`, and `workflow_runner_tags_response_without_wikilinks` verify active step-loop orchestration | unit |
| RTM-188 | FR-40 | `crates/maenifold-core/src/workflow/runner.rs::{run_think(),seed_think_session(),drive_think_session()}` | TC-188: T-57.2 Sequential thinking runner — `happy_path_stops_when_agent_says_done`, `overflow_protection_stops_at_total_plus_three`, and `injects_step_wikilink_when_agent_response_has_none` verify think-session orchestration and guardrails | unit |
| RTM-189 | FR-40 | `crates/maenifold-core/src/workflow/runner.rs::{step_think_config(),run_workflow()}` | TC-189: T-57.3 Submachine integration — `workflow_runner_thinks_before_agent_prompt_when_required`, `workflow_runner_skips_thinking_without_flag`, and `run_think_seeds_parent_workflow_linkage` verify SequentialThinking execution within workflow steps | integration |
| RTM-190 | FR-40 | `crates/maenifold-core/src/workflow/pipeline/executor.rs::{execute_pipeline(),classify_workflow_result()}` | TC-190: T-57.4 Pipeline executor wiring — `executor_completes_single_node_pipeline`, `executor_handles_node_failure`, `executor_respects_cancellation`, `executor_respects_max_concurrency`, and `executor_cascades_through_dag` verify executor use of the workflow runner | unit |
| RTM-191 | FR-40 | `crates/maenifold-cli/src/cli.rs::run_script_command()` | TC-191: T-57.5 CLI workflow and think subcommands — `cli_contract_tests` (6 tests) verify `workflow` and `think` command coverage | integration |

## Sprint — Pipeline UI Frontend (FR-43)

| RTM | Requirement | Component | Test Case | Type |
|-----|------------|-----------|-----------|------|
| RTM-192 | FR-43.1 | pipeline-ui/src/components/PipelineInspector.vue | TC-192: Inspector renders 5 context modes correctly | unit |
| RTM-193 | FR-43.2 | pipeline-ui/src/components/PipelineRunner.vue | TC-193: Runner controls + event log + mermaid tab | unit |
| RTM-194 | FR-43.3 | pipeline-ui/src/components/PipelineSidebar.vue | TC-194: Sidebar lists/loads/creates/deletes pipelines | unit |
| RTM-195 | FR-43.4 | pipeline-ui/src/components/PipelineView.vue + App.vue | TC-195: Four-panel layout integrates into App.vue | integration |
| RTM-196 | FR-44 | pipeline-ui/src/components/PipelineComposer.vue | TC-196: Composer allows adding/removing nodes in design mode | integration |
| RTM-197 | FR-45 | pipeline-ui/src/composables/__tests__/ | TC-197: MCP client integration tests against live server | integration |
| RTM-198 | FR-45 | pipeline-ui/src/composables/__tests__/ | TC-198: Composable integration tests against live server | integration |
| RTM-199 | FR-46 | pipeline-ui/src/components/__tests__/ | TC-199: Component smoke tests with real rendering | integration |

## Sprint 61 — Pipeline Dataflow Architecture (FR-47)

| RTM | Requirement | Component | Test Case | Type |
|-----|-------------|-----------|-----------|------|
| RTM-200 | FR-47 | `crates/maenifold-core/src/workflow/pipeline/types.rs` | TC-200: T-61.4 — PipelineNode tagged enum (Workflow/Prompt/SequentialThinking), NodeState.output, backwards-compatible serde with wire-type adapter; `old_workflow_json_still_deserializes`, `workflow_round_trip`, `prompt_round_trip`, `thinking_round_trip` | unit |
| RTM-201 | FR-47 | `crates/maenifold-core/src/workflow/pipeline/dispatch.rs` | TC-201: T-61.5 — Type-based dispatch branches on node type; workflow→activate_workflow_node_session, prompt→create prompt session, thinking→create thinking session; `dispatch_workflow_node`, `dispatch_prompt_node`, `dispatch_thinking_node` | unit |
| RTM-202 | FR-47 | `crates/maenifold-core/src/workflow/pipeline/update.rs` | TC-202: T-61.6 — Predecessor context reads from NodeState.output; `predecessor_context_from_output`, `predecessor_context_empty_when_no_output`, `predecessor_context_multiple_predecessors` | unit |
| RTM-203 | FR-47 | `crates/maenifold-core/src/workflow/pipeline/executor.rs` | TC-203: T-61.7 — Per-node prompt assembly + type-based execution; `executor_prompt_node_single_shot`, `executor_thinking_node_drives_session`, `executor_captures_output`, `executor_assembles_predecessor_context` | unit |
| RTM-204 | FR-47 | `crates/maenifold-core/src/workflow/pipeline/handler.rs` | TC-204: T-61.8 — Validate new node types on create/graft; `create_pipeline_with_prompt_node`, `create_pipeline_with_thinking_node`, `reject_prompt_node_missing_agent`, `reject_thinking_node_missing_topic` | unit |
| RTM-205 | FR-47 | `crates/maenifold-core/src/workflow/pipeline/view.rs` | TC-205: T-61.9 — Mermaid display differentiates node types; `mermaid_shows_prompt_node_style`, `mermaid_shows_thinking_node_style`, `mermaid_shows_workflow_node_style` | unit |

## Sprint 62 — Gate Generalization (FR-48)

| RTM | Requirement | Component | Test Case | Type |
|-----|-------------|-----------|-----------|------|
| RTM-210 | FR-48: pulldown-cmark parser | `crates/maenifold-gate/src/parser/sections.rs` | TC-210: pulldown-cmark AST extraction — `extract_sections`, verdict detection via Strong node state machine; parser_rework_tests (10 tests) | unit |
| RTM-211 | FR-48: string-based stages | `crates/maenifold-gate/src/types.rs` | TC-211: TaskStage/TaskStatus enums removed, StageState.stage is String, TaskState.status is String; all 95 tests validate string comparisons | refactor |
| RTM-212 | FR-48: pipeline definitions | `assets/pipelines/{sdd,tdd,gsd,lightweight}.json` | TC-212: pipeline validation — `load_valid_sdd_json`, `load_valid_tdd_json`, `load_valid_gsd_json`, `load_valid_lightweight_json`, `allow_cycle_with_escape_path` | unit |
| RTM-213 | FR-48: pipeline-aware parsing | `crates/maenifold-gate/src/parser/pipeline_aware.rs` | TC-213: `parse_task_content_with_pipeline` — pipeline-driven status derivation; `full_tdd_pipeline_pass_through`, `custom_pipeline_json`, `embedded_pipeline_name_works_end_to_end` | integration |
| RTM-214 | FR-48: named pipeline selection | `crates/maenifold-gate/src/pipeline.rs` | TC-214: `load_embedded_pipeline` — supports sdd/tdd/gsd/lightweight by name; `--pipeline` wired into all 4 commands; `status_accepts_pipeline_flag`, `sync_accepts_pipeline_flag` | unit |
| RTM-215 | FR-48: dynamic JSON status | `crates/maenifold-gate/src/status/output.rs` | TC-215: `StatusOutput.stages: Vec<StageOutput>` — deprecated swe/red/blue fields removed; `test_status_json_reports_dynamic_pipeline_stages` | integration |
| RTM-216 | FR-48: DB schema generalization | `crates/maenifold-gate/src/db/mod.rs` | TC-216: CHECK constraints removed from status/stage columns; `open_handles_existing_schema_variants`, `upsert_task_inserts_and_updates` | unit |
| RTM-217 | FR-48: deprecated no-pipeline API | `crates/maenifold-gate/src/parser.rs`, `types.rs` | TC-217: `#[deprecated]` on parse_task_content, parse_task_file, done_override_from_content; zero compiler warnings | refactor |

## Sprint 67 — Native ACP Client Hardening (FR-50)

| RTM | Requirement | Component | Test Case | Type |
|-----|-------------|-----------|-----------|------|
| RTM-220 | FR-50: capability capture | `crates/maenifold-core/src/acp/native.rs` | TC-220: T-67.1 — `AgentHandle` exposes `agent_info()`, `capabilities()`, `auth_methods`; InitializeResponse stored; protocol version warning logged | integration |
| RTM-221 | FR-50: session config passthrough | `crates/maenifold-core/src/workflow/agent_session.rs` | TC-221: T-67.2 — `open_agent_session()` accepts `SessionOpts`; pipeline node model/thought_level reach ACP session | integration |
| RTM-222 | FR-50: session update handling | `crates/maenifold-core/src/acp/native.rs` | TC-222: T-67.3 — `HeadlessClient` accumulates ToolCall/AgentThought; `AcpxResponse.tool_calls` populated; `thought` field added | unit |
| RTM-223 | FR-50: crash recovery | `crates/maenifold-core/src/acp/native.rs` | TC-223: T-67.4 — dead agent evicted from pool; `is_alive()` exposed; stderr captured; no zombie processes | integration |
| RTM-224 | FR-50: prompt timeout | `crates/maenifold-core/src/acp/native.rs` | TC-224: T-67.5 — `prompt()` wrapped with timeout; cancel sent on timeout; dead `_timeout` removed from `AcpxDriver` | unit |

## Sprint 68 — ACP Client Hardening Phase 2 (FR-50, FR-51)

| RTM | Requirement | Component | Test Case | Type |
|-----|-------------|-----------|-----------|------|
| RTM-225 | FR-51: client capabilities | `crates/maenifold-core/src/acp/native/{mod,client}.rs` | TC-225: T-68.2 — `ClientCapabilities` advertised with fs.read/write=true; `resolve_and_jail` rejects parent-dir traversal and paths outside CWD; `read_text_file`/`write_text_file` implemented | unit |
| RTM-226 | FR-51: MCP server passthrough | `crates/maenifold-core/src/acp/{config,native/session,native/types}.rs` | TC-226: T-68.3 — `McpServerEntry` enum (Stdio/Http); `mcp_servers` on `AgentEntry` and `SessionOpts`; threaded through `NewSessionRequest` | unit |
| RTM-227 | FR-51: session list & load | `crates/maenifold-core/src/acp/native/{mod,types}.rs` | TC-227: T-68.4 — `ListSessions`/`LoadSession` commands; capability-guarded via `AgentCapabilities`; `AcpxSessionInfo` mapping | unit |
| RTM-228 | FR-51: pipeline config | `crates/maenifold-core/src/workflow/pipeline/{types,types_wire,executor_nodes}.rs` | TC-228: T-68.5 — `AgentConfig` struct on node variants; flat field precedence via `.or_else()`; forward-compat warnings for mode/config_options | unit |
| RTM-229 | FR-51: feature flags | `Cargo.toml`, `crates/maenifold-core/src/acp/native/{client,types}.rs` | TC-229: T-68.6 — `unstable_session_usage` enabled; `UsageUpdate` handled; `AcpxUsage` surfaces usage in responses | unit |

## Sprint 69 — Pipeline Node Configuration Parity (FR-52)

| RTM | Requirement | Component | Test Case | Type |
|-----|-------------|-----------|-----------|------|
| RTM-230 | FR-52: view config fields | `crates/maenifold-core/src/workflow/pipeline/view_nodes.rs` | TC-230: T-69.2 — `add_pipeline_node_view_fields` serializes model/thoughtLevel/allowedTools/maxTurns/timeout/toolMode/agentConfig per node type. 12 view tests pass. | unit |
| RTM-231 | FR-52: schema config fields | `crates/maenifold-core/src/mcp/schema/pipeline_node_schema.rs` | TC-231: T-69.3 — oneOf schema per node type with correct required arrays (totalThoughts, type). 6 schema tests pass. | unit |
| RTM-232 | FR-52: graft guard fix | `crates/maenifold-core/src/workflow/pipeline/{engine,handler_surgery_graft,handler_crud}.rs` | TC-232: T-69.4 — `has_remaining_work()` checks Pending/Active/Ready node statuses. Graft+delete guards use node state not frontmatter. 20 graft+delete tests pass. | unit |
| RTM-233 | FR-52: stuck transition | `crates/maenifold-core/src/workflow/pipeline/update_restart.rs` | TC-233: T-69.5 — max-attempts rejection sets NodeStatus::Stuck and persists before error. 4 restart tests pass. | unit |
| RTM-234 | FR-52: wire agent mode | `crates/maenifold-core/src/acp/native/{types,session}.rs` | TC-234: T-69.6 — AgentConfig.mode → SessionOpts.mode → session/set_mode. Warning removed. | unit |
| RTM-235 | FR-52: wire config options | `crates/maenifold-core/src/acp/native/{types,session}.rs` | TC-235: T-69.7 — config_options → set_config_option calls with built-in key filtering. 5 config tests pass. | unit |
| RTM-236 | FR-52: list agents tool | `crates/maenifold-core/src/tools/agents.rs` | TC-236: T-69.8 — ListAgents MCP tool enumerates agents.json entries. 3 aliases registered. 11 agent tests pass. | unit |
