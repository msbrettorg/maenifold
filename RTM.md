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
