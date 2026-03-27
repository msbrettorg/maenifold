# Agent Team — maenifold-rs

## Roles

| Role | Purpose | Model |
|------|---------|-------|
| **PM** | Sprint orchestration, gate.db state, PRD/RTM ownership, branch/release management | `claude-opus-4.6-1m` |
| **SWE** | Focused implementation — code edits, features, bug fixes, refactoring | `gpt-5.4` |
| **Red-team** | Adversarial code audit — logic flaws, spec violations, drift detection | `claude-sonnet-4.6` |
| **Blue-team** | Verification — scoped test execution, coverage gates, PASS/FAIL verdicts | `gpt-5.4` |
| **Researcher** | Deep synthesis — technical research, fact-checking, graph-grounded findings | `claude-opus-4.6` |

## Dispatch Rules

- 1 focused task per agent (quality drops with batching)
- Max 3 items per red-team audit batch
- Target 60–120s per agent; 180s hard ceiling
- A task with more than 2 file edits or 3 behavioral changes is too big — split it
- Always include `[[WikiLinks]]` in prompts for concept-as-protocol context injection
- Forbid agents from running full test suites — targeted testing only
- **Every `task` call MUST include `model` parameter explicitly. No exceptions. Never rely on defaults.**
- **Never use `claude-opus-4.6` or `claude-opus-4.6-1m` for any agent without explicit user permission.** If you believe a task requires Opus, ask the user first. Default Researcher is the sole exception, but you need permission to use it.

## Operating Rules

- **When the user asks a question, answer it immediately.** Do not launch tool calls, do not start side work, do not deflect. Stop everything and answer. Then ask if they want you to continue.
- **When the user says something, they mean exactly what they said.** Not what you think they mean. Not something adjacent. Exactly what they said.
- **Reserve full SDD pipeline (SWE→red→blue) for logic changes and code that can break.** File moves, import fixes, and config changes get a single SWE pass + build verification. Don't run ceremony on trivial work.

## Banned Models

| Model | Reason |
|-------|--------|
| `gemini-*` | Environmental instability (file locking, false test failures) |
| `claude-haiku-*` | 75% false positive rate on code audits |
| `gpt-5.1-codex-mini` | Misses subtle drift that Sonnet 4.6 catches |

## Build Process

The release binary embeds the frontend SPA at compile time via Tauri's `generate_context!()`. **Cargo does not detect frontend changes automatically** — you must rebuild the frontend first, then touch a Rust source file to force recompilation.

### Full rebuild (frontend + binary)

```bash
# 1. Build the frontend (bun preferred, npm fallback)
cd vendor/acp-ui && bun install && bun run build

# 2. Force Rust to re-embed the new assets
cd /path/to/maenifold-rs
touch crates/maenifold-app/src/lib.rs

# 3. Build the release binary
cargo build --release -p maenifold-cli
```

Or use the Makefile shortcut:

```bash
make app
```

### Why `touch` is needed

Tauri embeds `vendor/acp-ui/dist/` into the binary at compile time. Cargo tracks `.rs` file mtimes, not frontend dist files. If you only rebuild the frontend, `cargo build` will no-op and ship stale assets. Touching `lib.rs` forces recompilation of the `maenifold-app` crate, which re-runs `tauri_build::build()` and re-embeds the dist.

### Binary location

The release binary is at `target/release/maenifold`. A symlink at `/usr/local/bin/maenifold` points to it. After rebuilding, restart the app to pick up changes:

```bash
maenifold --app            # Native Tauri desktop window
maenifold --http           # Browser-served SPA at http://localhost:8080
maenifold --mcp            # Headless MCP mode (no UI)
```

## Sprint Completion Checklist

Every sprint **must** complete ALL of the following before handoff:

1. ✅ All spec files moved from `todo/` to `done/`
2. ✅ `gate.db` shows all tasks in terminal state
3. ✅ Targeted tests pass for all changed code
4. ✅ **Release binary rebuilt**: full rebuild (frontend + `cargo build --release -p maenifold-cli`)
5. ✅ **Verify installed binary**: `maenifold --help` reflects all new features
6. ✅ CHANGELOG.md updated
7. ✅ RTM.md updated
8. ✅ Branch merged to main (if sprint branch used)
9. ✅ No untracked source files left uncommitted

Release gates:
- All tests must pass. No warnings. No ignored tests.
- The PM must smoke test all changes using the release binary.
- PM must prove that the Web UI can chat with OpenCode and Copilot via ACP.
- A sprint that ships a stale binary is a broken sprint. Period.

## Web UI / ACP Rules

- **The WebSocket URI MUST specify the agent via `?agent=` query parameter.** Example: `ws://localhost:9878/ws/acp?agent=OpenCode`. Never connect without specifying the agent.
- **No default agent fallback.** The relay must NOT silently pick the first agent from `agents.json` when no `?agent=` is provided. If the parameter is missing, the connection must be rejected with an error.
- **The PM must connect to a specific agent by name during smoke tests.** Connecting to the bare `/ws/acp` endpoint without `?agent=` is a test failure.
