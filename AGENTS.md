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
- **If any agent encounters code that violates WHAT_WE_DONT_DO principles — silent defaults, fallback strategies, hidden error recovery, fake confidence — it is a P0 escalation.** Stop work. Report it to the PM. The PM creates a backlog item in `todo/` immediately. No exceptions. No "we'll fix it later." No "it's out of scope." Violations of core principles are never out of scope.

## Daemon Discipline

**Agents must know exactly which gate process they are testing against.** Multiple gate instances cause silent test/verification failures when agents and browsers hit different instances serving different code.

### Rules

- **Before any gate smoke test or browser verification:** run `pgrep -la gate | grep -v taskgated` and `lsof -iTCP -sTCP:LISTEN -P | grep gate`. Know which PID is on which port. If you can't answer "which process am I hitting?", stop.
- **After rebuilding gate:** restart the target instance. Never assume the running process has your changes.
- **Agents must never launch gate processes in the background.** Only the user or LaunchAgent manages daemon lifecycle.
- **When verifying board changes:** confirm the port you're hitting matches the binary you just built. `ls -la /proc/<pid>/exe` or check the process command line.

### Why This Matters

If two gate daemons run on different ports, you test against one and ship another. Every "PASS" verdict is a lie. The board shows stale state. The user sees bugs that "can't exist" because tests passed. This is the definition of fake confidence.

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

Every sprint **must** complete ALL of the following before handoff. **Do not report work as done until the binary is rebuilt and verified. Words are not delivery.**

1. ✅ All spec files moved from `todo/` to `done/`
2. ✅ `gate.db` shows all tasks in terminal state
3. ✅ Targeted tests pass for all changed code
4. ✅ **Affected binaries rebuilt** — mandatory, no exceptions:
   - Gate changes: `cargo build --release -p maenifold-gate`
   - CLI/app changes: `cargo build --release -p maenifold-cli` (or `make app` if frontend changed)
5. ✅ **Verify installed binary works**: run the binary, confirm changes are present
6. ✅ CHANGELOG.md updated
7. ✅ RTM.md updated
8. ✅ Branch merged to main (if sprint branch used)
9. ✅ No untracked source files left uncommitted

Release gates:
- All tests must pass. No warnings. No ignored tests.
- The PM must smoke test all changes using the release binary.
- PM must prove that the Web UI can chat with OpenCode and Copilot via ACP.
- **A sprint that ships a stale binary is a broken sprint. Period.**
- **"Tests pass" is not delivery. A rebuilt, runnable binary is delivery.**

## Web UI / ACP Rules

- **The WebSocket URI MUST specify the agent via `?agent=` query parameter.** Example: `ws://localhost:9878/ws/acp?agent=OpenCode`. Never connect without specifying the agent.
- **No default agent fallback.** The relay must NOT silently pick the first agent from `agents.json` when no `?agent=` is provided. If the parameter is missing, the connection must be rejected with an error.
- **The PM must connect to a specific agent by name during smoke tests.** Connecting to the bare `/ws/acp` endpoint without `?agent=` is a test failure.

## What Ma Intentionally Does NOT Do

This document celebrates the features we've resisted adding. Each absence creates space (Ma) for AI to work more effectively.

### Core Principles We Don't Violate

#### 1. NO FAKE AI
We don't:
- ❌ Add retry logic that tries different approaches
- ❌ Implement fallback strategies when operations fail
- ❌ Create "smart" error recovery mechanisms
- ❌ Build adaptive behavior based on past failures
- ❌ Make decisions about what the user "probably meant"

**Why:** Every decision we make removes a decision the LLM could make. The LLM has context we don't.

#### 2. NO UNNECESSARY ABSTRACTIONS
We don't:
- ❌ Create a "MemoryInterface" when we just need MemoryDatabase
- ❌ Build factory patterns for simple object creation
- ❌ Add dependency injection frameworks
- ❌ Create abstract base classes for single implementations
- ❌ Hide simple operations behind complex APIs

**Why:** Abstractions create cognitive load. Simplicity creates understanding.

#### 3. NO PERFORMANCE OPTIMIZATIONS (without measurement)
We don't:
- ❌ Add caching layers "just in case"
- ❌ Implement connection pooling by default
- ❌ Create background workers for "faster processing"
- ❌ Batch operations that work fine individually
- ❌ Optimize code that isn't proven slow

**Why:** Premature optimization removes flexibility. The LLM might need that "inefficient" behavior.

### Specific Things We Don't Do

#### In Memory Management
We don't:
- ❌ Automatically categorize or tag memories
- ❌ Judge the *content quality* of memories
- ❌ Implement memory compression or archival
- ❌ Auto-delete memories — ever
- ❌ Build memory indexes beyond basic search

We *do* apply temporal signals:
- ✅ **Decay weighting** — time-based deprioritization (not deletion) via power-law decay
- ✅ **Access boosting** — deliberate reads reset decay clocks
- ✅ **Tiered half-lives** — episodic memories decay faster than procedural ones
- ✅ **Cognitive sleep cycle** — periodic consolidation of episodic → semantic knowledge

**Why the evolution:** This document originally said "don't score memory importance." Then the internal decay research happened — 29 citations from Ebbinghaus through Richards & Frankland proving that memory systems without forgetting become *worse* over time. The engineering demanded change.

**The distinction we preserve:** Decay scores *freshness*, not *worth*. The system has opinions about when you last used something, not about whether it was good. The LLM still decides what's important. The system decides what's recent.

#### In Tool Design
We don't:
- ❌ Combine tools that could be separate
- ❌ Add optional parameters for "convenience"
- ❌ Create tool aliases or shortcuts
- ❌ Build meta-tools that call other tools
- ❌ Implement tool versioning or deprecation

**Why:** Simple tools with single purposes are easier to understand and compose.

#### In Error Handling
We don't:
- ❌ Catch and recover from errors automatically
- ❌ Retry failed operations with modifications
- ❌ Provide suggested fixes for errors
- ❌ Hide error details for "cleaner" output
- ❌ Create error categorization systems

**Why:** Errors are information. The LLM needs all of it to make decisions.

#### In Configuration
We don't:
- ❌ Validate configuration beyond basic types
- ❌ Provide configuration wizards or generators
- ❌ Auto-migrate old configurations
- ❌ Suggest "better" configuration values
- ❌ Create configuration inheritance hierarchies

**Why:** Configuration is user space. We provide mechanism, not policy.

#### In Testing
We don't:
- ❌ Mock external dependencies
- ❌ Create test fixtures that don't match reality
- ❌ Use in-memory replacements for real systems
- ❌ Generate random test data
- ❌ Clean up test artifacts automatically

**Why:** Real tests with real systems find real problems. Test artifacts help debugging.

### Features We've Explicitly Rejected

#### 1. **Plugin System**
"We should add a plugin system so users can extend functionality!"

**No.** Users can modify the source. A plugin system adds complexity and constraints. The absence of a plugin API means users can change anything.

#### 2. **Memory Search Ranking** *(evolved)*
"We should rank search results by relevance/recency/importance!"

**Partially.** We now apply decay weighting to search and context results — recent and frequently-accessed memories rank higher. But we don't evaluate content quality or topical relevance. Hybrid search (semantic + full-text) handles relevance; decay handles freshness. The LLM still decides what matters in its current context.

#### 3. **Automatic Backup**
"We should automatically backup the memory directory!"

**No.** Backup is a user concern. We don't make assumptions about their backup strategy.

#### 4. **Rate Limiting**
"We should add rate limiting to prevent abuse!"

**No.** This is a local tool. If users want to abuse their own system, that's their choice.

#### 5. **Telemetry**
"We should add anonymous usage statistics!"

**No.** Not even anonymous. Not even opt-in. The absence of telemetry is a feature.

#### 6. **Update Checker**
"We should check for updates and notify users!"

**No.** Users can check for updates when they want to. We don't phone home.

### The Power of Absence

Each missing feature creates space for:
- User customization without fighting defaults
- LLM decision-making without constraints
- Simple debugging without abstraction layers
- Clear understanding without documentation
- Trust without privacy concerns

### What We Changed Our Minds About

間 is a design instinct, not a design law. Some absences had to give way when the evidence demanded it:

| Original Position | What Changed | Why |
|---|---|---|
| Don't score memory importance | Added decay weighting | 29-citation research paper proved unmanaged memory degrades retrieval quality |
| Don't create memory hierarchies | Added tiered decay (episodic/semantic/procedural) | Different memory types have different shelf lives — neuroscience is clear on this |
| Return results by modification time only | Added decay-weighted ranking | Without freshness signals, old noise drowns recent signal (context rot) |

**What we preserved through the change:**
- Never delete memories (decay deprioritizes, doesn't destroy)
- Never judge content quality (decay measures freshness, not worth)
- Never make decisions the LLM should make (the LLM decides what's important; the system decides what's recent)

Changing our minds isn't a violation of Ma. Refusing to change when the evidence demands it would be.

**The absence IS the feature. And knowing when to break the rule IS the wisdom.**
