# web-feature-flag

**Requirement**: FR-32
**Test Cases**: TC-32.3
**Component**: crates/maenifold-core/Cargo.toml

## Acceptance Criteria

1. A `web` feature flag exists in `crates/maenifold-core/Cargo.toml`
2. `web` implies the existing `http` feature (activates axum, tower-http, tokio-util)
3. `web` additionally gates: `rust-embed` (static file embedding), `agent-client-protocol` (ACP crate — add as `agent-client-protocol = { git = "https://github.com/agentclientprotocol/rust-sdk" }` to workspace dependencies)
4. Building without `--features web` excludes all web UI code and dependencies
5. `cargo build -p maenifold-cli --features web` compiles successfully with all web deps

## Verdicts

- TC-32.3 (web feature flag gates deps): ADD — test does not exist yet
