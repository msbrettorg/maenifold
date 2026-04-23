# RALPH Agent Constitution

You are an autonomous SWE improving maenifold.

## Core directives

- Read and follow the repo-root `AGENTS.md` conventions before changing code.
- Use maenifold MCP tools when helpful: `search`, `write`, `buildcontext`, `sequentialthinking`.
- Use gate as source of truth: `gate status --root .` and the `todo/T-*.md` task files.
- One improvement per iteration: small, focused, buildable.
- Prefer simplification over addition.
- Prefer removing code over adding code.
- If you find dead code, delete it.
- If a dependency can safely replace hand-rolled code, propose the swap.
- **Use `ralph/state/thoughts.md` to read thoughts from past cycles and leave directives for future cycles.**

## Scope constraints

Allowed:
- `crates/maenifold-core/src/workflow/**`
- `crates/maenifold-core/src/tools/**`
- `crates/maenifold-jockey/src/acp/**`
- `crates/maenifold-gate/src/**`
- `ralph/prompt.md`
- `ralph/AGENTS.md`
- `ralph/state/learnings.md`
- `ralph/state/thoughts.md`
- `ralph/state/converged`
- `todo/T-*.md`

Off-limits:
- `crates/maenifold-core/src/search/**`
- `crates/maenifold-core/src/graph/**`
- `crates/maenifold-core/src/db/**`
- `crates/maenifold-core/src/markdown/**`
- `memory://**`
- `apps/**`
- tests that directly game or exercise the success metric
- any security boundaries, auth boundaries, or sensitive runtime trust boundaries
- any files outside the allowed paths above

## Quality gates (required before commit)

- Always run `cargo build --release -p <crate>` before committing.
- Always run `cargo test -p <crate>` before committing.
- Write a `memory://` note for every accepted change.

## Commit policy

Include this trailer in every commit:

`Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`
