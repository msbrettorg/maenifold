# maenifold-rs reboot snapshot

## purpose
This file captures the current state of the Rust reimplementation of `maenifold` so work can resume cleanly after a reboot.

## current status
- Workspace root: `/Users/brett/src/ma-collective/maenifold-rs`
- Oracle repo: `/Users/brett/src/ma-collective/maenifold`
- Current Rust workspace status: `cargo test` passes.
- Current parity status: the active Rust parity suite is green against the C# oracle for memory, search/graph, session/workflow, and the current vector/watcher slice.

## implemented tool surface
All 28 registered tools now resolve to real handlers. The implemented tool surface is:
- `WriteMemory`
- `ReadMemory`
- `EditMemory`
- `DeleteMemory`
- `MoveMemory`
- `SearchMemories`
- `ExtractConceptsFromFile`
- `Sync`
- `BuildContext`
- `Visualize`
- `SequentialThinking`
- `Workflow`
- `RecentActivity`
- `MemoryStatus`
- `GetConfig`
- `GetHelp`
- `ListMemories`
- `UpdateAssets`
- `RepairConcepts`
- `AnalyzeConceptCorruption`
- `FindSimilarConcepts`
- `StartWatcher`
- `StopWatcher`
- `RunFullBenchmark`
- `Adopt`
- `AssumptionLedger`
- `ListAssets`
- `ReadMcpResource`

Remaining parity gaps are now outside the registry implementation itself:
- MCP resource parity is not complete; direct CLI mode is still the main working surface.
- Oracle parity tests are still missing for `Adopt`, `AnalyzeConceptCorruption`, `UpdateAssets`, `RecentActivity`, `RunFullBenchmark`, and `RepairConcepts`.
- Workflow asset parity is not complete; Rust currently ships 41 workflow JSON files versus 69 in the C# oracle.

## important compatibility fixes already made
### shared storage / db
- Rust now matches the C# `concepts` table contract closely enough for mixed-runtime sync:
  - includes `first_seen`
  - uses `occurrence_count`
- This fixed a real mixed-runtime issue where C# `Sync` failed against a Rust-created DB.

### markdown / wikilinks
- WikiLink extraction now excludes code blocks and inline code.
- `pulldown-cmark` parses `[[wikilinks]]` as wiki-link events, so extraction was updated to read parsed wiki links directly and still preserve fallback scanning where needed.

### output mode parity
The following tools intentionally return plain text even with `--json`, because that matches observed C# behavior:
- `Sync`
- `BuildContext`
- `Workflow`
- `AssumptionLedger`
- `ExtractConceptsFromFile`
- `FindSimilarConcepts`
- `StartWatcher`
- `StopWatcher`

### vector parity shortcut
- `FindSimilarConcepts` can reuse C#-generated `vec_concepts` data by loading the `vec0` extension and querying existing embeddings.
- This avoids requiring full Rust embedding generation for the current parity slice.

## key files changed recently
### core implementation
- `crates/maenifold-core/src/sessions.rs`
- `crates/maenifold-core/src/search.rs`
- `crates/maenifold-core/src/markdown.rs`
- `crates/maenifold-core/src/db.rs`
- `crates/maenifold-core/src/watchers.rs`
- `crates/maenifold-core/src/registry.rs`
- `crates/maenifold-core/src/assets.rs`
- `crates/maenifold-core/src/memory.rs`
- `crates/maenifold-core/src/config.rs`

### test / parity harness
- `crates/maenifold-core/tests/memory_boundary_paths.rs`
- `crates/maenifold-core/tests/search_graph_boundary.rs`
- `crates/maenifold-core/tests/session_workflow_boundary.rs`
- `crates/maenifold-core/tests/vector_sync_boundary.rs`
- `crates/maenifold-cli/tests/memory_boundary_cli_tests.rs`
- `crates/maenifold-cli/tests/search_graph_cli_tests.rs`
- `crates/maenifold-cli/tests/session_workflow_cli_tests.rs`
- `crates/maenifold-cli/tests/system_tools_cli_tests.rs`
- `crates/maenifold-cli/tests/vector_sync_cli_tests.rs`
- `tests/compat/cli_json_parity.rs`
- `tests/compat/search_graph_parity.rs`
- `tests/compat/session_workflow_parity.rs`
- `tests/compat/vector_sync_parity.rs`
- `tests/cli_json_parity.rs`
- `tests/search_graph_parity.rs`
- `tests/session_workflow_parity.rs`
- `tests/vector_sync_parity.rs`
- `tests/support/cli_runner.rs`
- `tests/support/compat_env.rs`
- `tests/support/json_semantic.rs`

## current test inventory
Current Rust test files include:
- `crates/maenifold-core/tests/contracts.rs`
- `crates/maenifold-core/tests/memory_boundary_paths.rs`
- `crates/maenifold-core/tests/search_graph_boundary.rs`
- `crates/maenifold-core/tests/session_workflow_boundary.rs`
- `crates/maenifold-core/tests/vector_sync_boundary.rs`
- `crates/maenifold-cli/tests/cli_contract_tests.rs`
- `crates/maenifold-cli/tests/memory_boundary_cli_tests.rs`
- `crates/maenifold-cli/tests/search_graph_cli_tests.rs`
- `crates/maenifold-cli/tests/session_workflow_cli_tests.rs`
- `crates/maenifold-cli/tests/system_tools_cli_tests.rs`
- `crates/maenifold-cli/tests/vector_sync_cli_tests.rs`
- `tests/cli_json_parity.rs`
- `tests/search_graph_parity.rs`
- `tests/session_workflow_parity.rs`
- `tests/vector_sync_parity.rs`

At the last check there were no ignored tests left in the active suite.

## validated commands
These were green at the current checkpoint:
- `cargo test`
- targeted lanes for:
  - memory boundary
  - search/graph
  - session/workflow
  - vector/watcher parity

## recommended working policy going forward
### test cadence
Use Rust tests while coding; use the C# oracle only at parity gates.

Recommended cadence:
- small local change -> run one Rust test or one Rust test file
- coherent slice complete -> run the Rust tranche lane
- checkpoint only -> run C# parity gate(s)
- full `cargo test` only at explicit checkpoints, not after every small edit

### oracle policy
Use the C# repo to define contracts, fixtures, and parity tests, but do not invoke it on every tiny iteration.

## known practical notes
- `vec0` native libraries exist under `assets/native/...`
- model assets exist under `assets/models/...`
- the current `FindSimilarConcepts` parity path can query C#-generated vectors directly through `vec0`
- direct CLI mode is the most reliable validation surface right now

## definition of done not yet reached
This repo is in a strong intermediate state, but it is **not** yet at full product parity with the C# implementation because MCP resource parity is still incomplete, oracle parity tests are still missing for the six newly implemented tools, and workflow asset parity is still short of the full C# set.
