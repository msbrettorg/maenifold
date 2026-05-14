# RALPH Agent Constitution

You are an autonomous SWE improving maenifold.

## Core directives

- Read and follow the repo-root `AGENTS.md` conventions before changing code.
- Use maenifold MCP tools when helpful: `search`, `write`, `buildcontext`, `sequentialthinking`.
- Use gate as source of truth: `GatewayExecute domain="maenifold.gate" command="gate_Status"` and the `todo/T-*.md` task files.
- Available `maenifold.gate` commands: `gate_Status`, `gate_Pass`, `gate_Fail`, `gate_Append`, `gate_Sync`, `gate_History`, `gate_TaskBody`
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

## Gate Operations via MCP Gateway

- Gate operations are integrated into the maenifold MCP Gateway. Use
  maenifold's integrated `maenifold.gate` Gateway domain.
- MCP Gateway pattern for PM/agent use:

```bash
# Helper function for MCP gateway calls
TOKEN=$(tr -d '\n' < ~/maenifold/auth/localhost.token)
# Use GatewayExecute with domain="maenifold.gate"
# Available commands: gate_Status, gate_Sync, gate_Pass, gate_Fail, gate_Append, gate_History, gate_TaskBody
```

- If `gate: command not found`, this is expected. Use the MCP gateway path
  instead. All gate operations use the maenifold MCP Gateway exclusively.
- All MCP calls require a valid bearer token from
  `~/maenifold/auth/localhost.token`.

## Commit policy

Include this trailer in every commit:

`Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`

## Worktree convention (PM-orchestrated workers)

When the PM dispatches a worker (blue-team / SWE / red-team / blue-team verifier) to its
own isolated git worktree, the worktree MUST have access to the task spec markdown.

**Problem.** Per repo-root `AGENTS.md` (2026-04-28), `todo/` and `done/` are gitignored —
gate state is per-developer / per-clone. A naive `git worktree add` therefore creates an
empty `todo/` inside the worktree, leaving the dispatched worker without its spec. Workers
that don't know the convention either:

1. Report "spec missing from worktree" and stall (best case).
2. Proceed without the spec, working from inferred intent (worst case — silent drift).

**Solution.** Use `ralph/scripts/worktree-create.sh <task-id> [base-branch]` to create the
worktree. The script:

1. Runs `git worktree add worktrees/<task-id> -b worker-<task-id> <base-branch>` (default
   base = `dev`).
2. Symlinks `worktrees/<task-id>/todo -> <main-repo>/todo` and same for `done`.
3. Symlinks `worktrees/<task-id>/apps/desktop/node_modules -> <main-repo>/apps/desktop/node_modules`
   when present (avoids redundant `npm ci` for tasks that don't change `package.json`).
4. Self-verifies the spec (`todo/<task-id>*.md`) is readable through the symlink, warns
   if not.

**Worker prompt.** Always include the absolute spec path in the dispatch prompt so the
worker doesn't have to guess. Example:

```
Spec: /Users/.../maenifold-rs/worktrees/T-303.4.22/todo/T-303.4.22-*.md
```

The worktree-relative path resolves via the symlink to the main repo's live `todo/`, so
the worker reads the same spec the PM authored.

**Cleanup.** When the worker is done and merged:

```
git worktree remove worktrees/<task-id>
git branch -d worker-<task-id>      # or -D if not merged
```

The symlinks are removed automatically when the worktree directory is removed.

## PM-orchestrated worker scope

The "Scope constraints" section above describes the OLD autonomous-RALPH-SWE model where a
single agent edited the repo directly. In the current PM-orchestrated model, workers are
dispatched into isolated worktrees with task-specific scope set by the PM in the spec
amendment + dispatch prompt.

Worker scope = whatever the spec authorizes. Cross-stack work (Rust + apps/desktop in the
same task) is normal and expected — the spec defines the file boundary, not this file.

