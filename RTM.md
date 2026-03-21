# Requirements Traceability Matrix — maenifold-rs

For completed requirements (RTM-001 through RTM-114, RTM-147 through RTM-156), see [CHANGELOG.md](CHANGELOG.md).

## Sprint 51 — Pipeline Active Executor (FR-40)

| RTM | Requirement | Component | Test Case | Test Type |
|-----|-------------|-----------|-----------|-----------|
| RTM-157 | FR-40 | `crates/maenifold-core/src/workflow/pipeline/engine.rs` | TC-157: petgraph StableGraph replaces hand-rolled DAG; cycle detection, toposort, ready-set all pass existing tests | unit |
| RTM-158 | FR-40 | `crates/maenifold-core/src/acp/driver.rs` | TC-158: AcpX driver spawns acpx subprocess, creates sessions, sends prompts with --format json, parses NDJSON output | integration |
| RTM-159 | FR-40 | `crates/maenifold-core/src/workflow/pipeline/executor.rs` | TC-159: Executor run loop computes ready set, dispatches via JoinSet+Semaphore, cascades completion, respects maxConcurrency | integration |
| RTM-160 | FR-40 | `crates/maenifold-core/src/workflow/pipeline/executor.rs` | TC-160: Lifecycle controls — pause drains active, resume continues, cancel aborts, rewind resets subgraph | integration |
| RTM-161 | FR-40 | `crates/maenifold-core/src/workflow/pipeline/executor.rs` | TC-161: HSM integration — executor drives Workflow tool per node, spawns SequentialThinking submachines | integration |
| RTM-162 | FR-40 | `crates/maenifold-core/src/workflow/pipeline/handler.rs` | TC-162: Graft adds nodes+edges to existing pipeline, validates no cycles | unit |
| RTM-163 | FR-40 | `crates/maenifold-core/src/workflow/pipeline/handler.rs` | TC-163: Join merges two DAGs with ID remapping; Fork clones subgraph with state reset | unit |
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
