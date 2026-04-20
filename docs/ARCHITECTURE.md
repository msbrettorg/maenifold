# Architecture Guide (Verified from Source)

This guide documents the runtime architecture as implemented in this repository, using code paths as the source of truth.

## Runtime topology

```
[Desktop app] <-----> [Maenifold daemon :8080]  (HTTP /mcp + /api/*)
                              ^
                              |
[CLI: maenifold (stdio)] <--> [maenifold-core] <--> [Daemon]
                              ^
                              |
[maenifold-pipeline-driver] --+--> [ACP runtimes via maenifold-jockey]

[gate service :8081] <--> [todo/ + done/ task files]   (HTTP /mcp)
```

Two service surfaces (daemon :8080 and gate :8081) plus the CLI all converge on `maenifold-core`. The pipeline driver dispatches ACP runtimes through `maenifold-jockey`.

<details><summary>Same diagram in Mermaid (graphical viewers only)</summary>

```mermaid
flowchart LR
    Desktop["Desktop app"] <--> Daemon["Maenifold daemon :8080 (/mcp + /api/*)"]
    CLI["maenifold CLI (stdio MCP / tool)"] <--> Core["maenifold-core"]
    Daemon <--> Core
    Driver["maenifold-pipeline-driver"] <--> Core
    Driver <--> ACP["ACP runtimes via maenifold-jockey"]
    Gate["gate service :8081 (/mcp)"] <--> Files["todo/ + done/ task files"]
```

</details>

## Crates and responsibilities (5 crates)

1. **`maenifold-core`**  
   Core tool registry, MCP server, memory/graph/search/workflow/pipeline logic.

2. **`maenifold-cli`**  
   Binary entrypoint for `maenifold`; dispatches to core CLI/MCP/HTTP runtime.

3. **`maenifold-gate`**  
   File-authoritative gate engine and MCP service for task progression (`status/append/pass/fail/history/task_body/sync`), with `.gate/gate.db` as read model cache.

4. **`maenifold-jockey`**  
   ACP bridge/runtime integration used by core workflow/pipeline execution.

5. **`maenifold-pipeline-driver`**  
   Detached pipeline executor process; runs pipeline sessions, dispatches nodes, handles cancellation/recovery, and persists pipeline state.

## MCP boundary rule (single capability surface)

**Rule:** operations are exposed as MCP tools, and the same underlying handlers are used by both human-invoked CLI and agent-invoked MCP calls.

- In core, MCP `call_tool` routes to `ToolRegistry::global().try_invoke(...)`.
- CLI `maenifold tool ...` also routes to `ToolRegistry::global().try_invoke(...)`.
- MCP tool discovery in core is generated from canonical registry names (`list_mcp_tools` from `ToolRegistry`), so MCP and CLI share the same operation surface.
- Gate MCP similarly routes tool calls to `GateToolService`, which is also used by gate command handlers.

## Data model and runtime paths (verified)

From `crates/maenifold-core/src/config.rs` (`Config::current()`):

- `maenifold_root` = `MAENIFOLD_ROOT` override, else `~/maenifold`
- `memory_path` = `<maenifold_root>/memory`
- `assets_path` = `<maenifold_root>/assets`
- `database_path` = `MAENIFOLD_DATABASE_PATH` override, else `<maenifold_root>/memory.db`

Important correction:

- Database default is **`memory.db` at root** (`~/maenifold/memory.db`), **not** `.maenifold/graph.db`.

Related persistence:

- Memory markdown URIs (`memory://...`) resolve under `memory_path` via `uri_to_path(..., &config.memory_path)`.
- Gate read model DB path is `<workspace>/.gate/gate.db` (`GateCacheDb::path_for_root`), while authoritative task state remains markdown files in `todo/` and `done/`.

## E2E trace 1: `WriteMemory`

1. Entry (CLI or MCP):
   - CLI: `CliApp::run_tool_inner` → `ToolRegistry::try_invoke("WriteMemory", ...)`
   - MCP: `MaenifoldMcpServer` `call_tool` → `ToolRegistry::global().try_invoke(...)`
2. Registry maps `WriteMemory` to `memory::write_memory`.
3. `memory::write_memory` validates payload/wiki links, builds URI `memory://...`, then calls `write_markdown(...)`.
4. `write_markdown` resolves URI with `uri_to_path(..., &config.memory_path)` and writes markdown file.
5. File is indexed (`index_file_content(...)`) after write.

## E2E trace 2: Pipeline run

1. `Pipeline` tool with `sessionId + run=true` enters `workflow::pipeline::handler_lifecycle::run_pipeline`.
2. Runtime context is persisted (`persist_pipeline_runtime_context`), then core spawns `maenifold-pipeline-driver`.
3. Driver (`maenifold-pipeline-driver/src/main.rs`) calls `execute_pipeline(...)`.
4. Executor loop:
   - `dispatch_pipeline(...)` claims ready nodes (`pending -> active`)
   - Node execution requests are built and scheduled.
5. Node execution:
   - Prompt/workflow/thinking nodes call workflow runner functions (`run_prompt_node`, `run_workflow_with_pool_session_opts`, `run_think_with_pool`)
   - These open ACP-backed sessions and call `jockey::acp::execute_runtime(...)`
   - Tool nodes call either local tools (`ToolRegistry::try_invoke`) or external MCP servers via RMCP HTTP/stdio.
6. Final status is persisted with `persist_final_pipeline_status`.

## E2E trace 3: Gate progression (`pending -> ... -> done`)

1. Entry via CLI (`gate pass`) or MCP (`pass` tool) converges in `GateToolService::pass`.
2. `pass_task`/`apply_outcome`:
   - acquires task lock
   - reads authoritative task markdown from `todo/` or `done/`
   - resolves pipeline definition + current state
   - computes transition via `StateMachine::next_on_pass` / `next_on_fail`
3. Writes canonical stage section to task markdown.
4. If next state is terminal, `TaskRepository::move_to_done(...)` renames task file into `done/`.
5. Read model is refreshed (`sync_gate`) into `<root>/.gate/gate.db`.

## Source anchors

- Core paths/config: `crates/maenifold-core/src/config.rs`
- Core CLI + MCP dispatch:  
  `crates/maenifold-core/src/cli.rs`  
  `crates/maenifold-core/src/cli/handlers.rs`  
  `crates/maenifold-core/src/mcp/mod.rs`  
  `crates/maenifold-core/src/registry.rs`
- Memory write path mapping:  
  `crates/maenifold-core/src/memory/access.rs`  
  `crates/maenifold-core/src/markdown/storage.rs`  
  `crates/maenifold-core/src/markdown/paths.rs`
- Pipeline lifecycle/driver/executor/ACP:  
  `crates/maenifold-core/src/workflow/pipeline/handler_lifecycle.rs`  
  `crates/maenifold-pipeline-driver/src/main.rs`  
  `crates/maenifold-core/src/workflow/pipeline/executor.rs`  
  `crates/maenifold-core/src/workflow/pipeline/executor_run.rs`  
  `crates/maenifold-core/src/workflow/agent_session.rs`  
  `crates/maenifold-core/src/workflow/runner.rs`
- Daemon and desktop endpoints:  
  `crates/maenifold-core/src/mcp/http.rs`  
  `apps/desktop/electron/main.ts`  
  `apps/desktop/src/lib/daemon-url.ts`  
  `apps/desktop/src/lib/constants.ts`
- Gate progression/data model:  
  `crates/maenifold-gate/src/cli.rs`  
  `crates/maenifold-gate/src/pass_command.rs`  
  `crates/maenifold-gate/src/task_repository.rs`  
  `crates/maenifold-gate/src/gate_tool_service.rs`  
  `crates/maenifold-gate/src/cache_db.rs`  
  `crates/maenifold-gate/src/file_watcher.rs`  
  `crates/maenifold-gate/src/mcp/mod.rs`
