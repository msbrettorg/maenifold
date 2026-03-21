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
