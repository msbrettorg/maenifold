# AGENTS.md
Build: `cargo build`; `cargo build -p maenifold-core`; `cargo build -p maenifold-cli`
Format/lint: `cargo fmt --all`; `cargo fmt --all --check`; `cargo clippy --workspace --all-targets`; `cargo clippy --workspace --all-targets -- -D warnings`
Test policy: tests are heavy; run the narrowest relevant test first; run full `cargo test` only at explicit checkpoints.
List tests: `cargo test -- --list`; `cargo test --test cli_json_parity -- --list`; `cargo test -p maenifold-cli --test session_workflow_cli_tests -- --list`
One integration file: `cargo test --test cli_json_parity`; `cargo test --test search_graph_parity`; `cargo test --test session_workflow_parity`; `cargo test --test vector_sync_parity`
One exact integration test: `cargo test --test cli_json_parity write_memory_json_matches_csharp_oracle -- --exact`; `cargo test --test vector_sync_parity watcher_lifecycle_text_matches_csharp_oracle -- --exact`
One crate test file: `cargo test -p maenifold-core --test contracts`; `cargo test -p maenifold-core --test memory_boundary_paths`; `cargo test -p maenifold-cli --test memory_boundary_cli_tests`; `cargo test -p maenifold-cli --test session_workflow_cli_tests`
One exact crate test: `cargo test -p maenifold-cli --test memory_boundary_cli_tests write_memory_json_success_includes_uri_and_checksum -- --exact`; `cargo test -p maenifold-core --test contracts tool_registry_is_case_insensitive_and_exposes_contract_names -- --exact`
Heavy commands: `cargo test -p maenifold-core`; `cargo test -p maenifold-cli`; `cargo test`
Subagent models: SWE → `gpt-5.4`; red-team/blue-team → `claude-sonnet-4.6`; researcher → `claude-opus-4.6`
