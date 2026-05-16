<!-- T-10.14: RTM FR-16 / RTM-093 -->
# Development Guide

Rust-specific build, test, and contribution notes for `maenifold-rs`.

## Prerequisites

- Rust stable toolchain
- Git
- Optional: ONNX model under `assets/models/` for semantic/vector features
- vec0 native library for your platform under `assets/native/<platform>/`

Notes:
- The workspace builds without an ONNX model, but semantic features degrade cleanly when no model is present.
- vec0 is only needed for vector-backed features that rely on SQLite vector tables.

## Build

From the repository root:

```bash
cargo build
cargo build -p maenifold-core
cargo build -p maenifold-cli
cargo build -p maenifold-pipeline-driver
```

If your change touches `crates/maenifold-core/src/workflow/`, build both
`maenifold-cli` and `maenifold-pipeline-driver`: the CLI hosts the daemon
surface, and the driver hosts pipeline execution.

## Format / Lint

```bash
cargo fmt --all
make quality-lint
```

## Test

Tests are heavy. Follow the repo policy in `AGENTS.md`: run the narrowest relevant test first, then expand only as needed.

### Unit / boundary

```bash
cargo test -p maenifold-core
```

### CLI

```bash
cargo test -p maenifold-cli
```

### Workspace integration

```bash
cargo test --test workspace_cli_smoke
```

### Full suite

```bash
MAENIFOLD_FULL_SUITE=1 cargo test
```

### Vector / embedding suite

```bash
MAENIFOLD_VECTOR_SUITE=1 cargo test -p maenifold-core --test vector_integration_tests
```

### Single exact test

```bash
cargo test --test workspace_cli_smoke workspace_cli_smoke_is_rust_only -- --exact
```

## Quality reports

Install the extra analysis tools when you want coverage and code-health reports:

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov cargo-machete rust-code-analysis-cli
npm install -g jscpd
```

Then run the focused targets from the repository root:

```bash
make quality-coverage       # LCOV summary under target/quality/coverage/
make quality-coverage-html  # HTML coverage report
make quality-metrics        # rust-code-analysis JSON metrics
make quality-dead-code      # clippy dead/unused findings as JSON lines
make quality-unused-deps    # cargo machete report
make quality-duplicates     # jscpd duplicate-code report
make quality-report         # aggregate all of the above + approximate file-level CRAP
```

Notes:
- `quality-lint` keeps correctness-oriented Clippy checks as a gate while letting the dedicated quality report own complexity/size signals such as `too_many_lines`, `too_many_arguments`, and `cognitive_complexity`.
- `quality-report` opts into the full Rust suite explicitly via `MAENIFOLD_FULL_SUITE=1` when generating coverage.
- The vector/embedding suite now lives in `vector_integration_tests` and stays opt-in unless you set `MAENIFOLD_VECTOR_SUITE=1` or target an exact vector test by name.
- `quality-dead-code` intentionally scopes Clippy to library/binary targets so production-code dead/unused findings stay usable even when test-only modules are noisy.
- The CRAP section is file-level and approximate: it combines LCOV line coverage with `rust-code-analysis` complexity metrics.
- Raw artifacts land under `target/quality/`.

## Workspace structure

- `crates/maenifold-core/` — core library: tool handlers, registry, MCP server, storage, search, vector integration
- `crates/maenifold-cli/` — binary entrypoint wrapping `maenifold_core::CliApp`
- `tests/` — Rust-native workspace integration tests
- `assets/` — workflows, tool docs, models, and native dependencies

## Adding a new tool

1. Add the tool to `crates/maenifold-core/src/registry.rs`.
2. Implement the handler in the appropriate core module.
3. Add a boundary test under `crates/maenifold-core/tests/`.
4. Add a CLI test under `crates/maenifold-cli/tests/`.
5. Add a workspace integration test under `tests/` when the change crosses crate boundaries.
6. Run the narrowest relevant test commands first, then rerun broader suites as a checkpoint.

If the tool has usage/help text, keep the tool contract, JSON envelope behavior, and error wording aligned with existing registry patterns.

## MCP mode

Run the stdio MCP server:

```bash
maenifold --mcp
```

## CLI mode

Run a single tool directly:

```bash
maenifold tool <name> '<json>'
```

Request JSON output when needed:

```bash
maenifold tool <name> '<json>' --json
```

## Environment variable sources

`maenifold.json` supports four ways to provide environment variables to spawned
child processes. The same env-source shapes work for both ACP agent entries
under `agents.*` and stdio MCP server entries under `mcpServers.*`.

Use the shape that matches where the value already lives:

- `env` — inline literal key/value pairs. Values are visible in
  `maenifold.json`, so use this for non-sensitive, static settings.
- `envPassthrough` — allow-list of variable names to forward from the daemon's
  own environment. Use this when the daemon already has the value, such as a
  terminal-launched session inheriting `std::env`.
- `envFromFile` — read a value from a file path. `~` is expanded to `$HOME` at
  the start of the path only (`~/foo` → `$HOME/foo`). `~user/` expansion is not
  supported. Paths not starting with `~/` are used as-is. Use this for secrets
  stored on disk. Map each variable name to a file path.
- `envFromCommand` — run a command and use trimmed stdout as the value. There is
  no implicit shell wrapper: specify the executable and args explicitly. Use
  this for keychain-backed or dynamically resolved secrets.

### Precedence

When the same variable appears in more than one source, maenifold resolves it in
this order:

| Priority | Source | Notes |
|---|---|---|
| 1 (highest) | `env` literal map | string→string, untouched |
| 2 | `envFromCommand` | spawn command, capture stdout, trim trailing whitespace |
| 3 | `envFromFile` | read file at path (with `~` expansion), trim trailing whitespace |
| 4 (lowest) | `envPassthrough` allow-list | pulls from `std::env::var()`, only sets VAR if not already set by 1-3 |

Missing files or non-zero command exits are fail-soft: maenifold logs a warning
without the secret value and skips that variable instead of injecting a partial
or empty value.

### Spawn-path applicability

These env-source shapes apply to both supported child-process spawn paths:

1. ACP agent entries under `agents.*` in `maenifold.json`
2. `mcpServers` stdio entries in `maenifold.json`

### Complete example

```json
{
  "agents": {
    "Codex": {
      "command": "/Users/alex/bin/codex-acp",
      "args": ["serve"],
      "env": {
        "OPENAI_API_BASE": "https://my-resource.openai.azure.com",
        "OPENAI_API_VERSION": "2024-10-21"
      },
      "envFromCommand": {
        "ANTHROPIC_API_KEY": {
          "command": "/usr/bin/security",
          "args": ["find-generic-password", "-w", "-s", "anthropic-api-key"]
        }
      },
      "envFromFile": {
        "AZURE_OPENAI_API_KEY": "~/.secrets/azure_openai_api_key",
        "EXPERIMENTAL_TOKEN": "~/.secrets/experimental_token"
      },
      "envPassthrough": ["HTTP_PROXY", "HTTPS_PROXY"]
    }
  },
  "mcpServers": {
    "github": {
      "command": "/Users/alex/bin/github-mcp-server",
      "args": ["stdio"],
      "envFromCommand": {
        "GITHUB_TOKEN": {
          "command": "/usr/bin/security",
          "args": ["find-generic-password", "-w", "-s", "github-token"]
        }
      },
      "envPassthrough": ["NO_PROXY"]
    }
  }
}
```

For related secret-helper patterns, see OpenAI Codex `[auth] command/args`
configuration and Claude Code `apiKeyHelper`:

- <https://developers.openai.com/codex/config-reference>
- <https://docs.claude.com/en/docs/claude-code/settings>

Example Codex agent configuration:

```json
{
  "Codex": {
    "command": "/path/to/codex-acp",
    "envFromFile": {
      "AZURE_OPENAI_API_KEY": "~/.secrets/azure_openai_api_key"
    }
  }
}
```

For Azure OpenAI-backed Codex, `~/.codex/config.toml` must set an `env_key`
that matches the configured environment variable name, such as
`env_key = "AZURE_OPENAI_API_KEY"`.

Out of the box, the create-pipeline flow defaults to `Copilot` when present
because it uses `gh` CLI auth and needs no secret-file setup. If `Copilot` is
not configured, the flow falls back to the first configured agent.
