# Requirements Traceability Matrix — maenifold-rs

For completed requirements (RTM-001 through RTM-114, RTM-147 through RTM-156), see [CHANGELOG.md](CHANGELOG.md).

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
| RTM-166 | FR-41 | `crates/maenifold-gate/src/db.rs` | TC-166: T-53.2 SQLite Cache Layer — CRUD operations for rebuildable `pipeline.db` cache | unit |
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
