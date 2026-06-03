# maenifold — Constitution

One product, one Rust workspace. `crates/` = backend (CLI + MCP server + pipeline driver + ACP runtime); clients reach the backend through MCP. `ui/` holds a standalone, vendored ACP UI (copy-and-own, decoupled from maenifold internals) used as a convenience client; it is not part of the backend and must not be wired into MCP/ACP/inferencing resolver paths.

## Key Documents

- **[Architecture Guide](docs/ARCHITECTURE.md)** — Runtime topology, crate responsibilities, three-protocol model (MCP / ACP / Inferencing), end-to-end traces. Read before touching daemon, MCP, ACP, or pipeline code.
- **[ACP Client Architecture](docs/acp-client-architecture.md)** — Law for anything touching the ACP runtime in `maenifold-jockey` or `acp_exec*`.

---

## P0 Escalation Triggers

**Any agent encountering ANY of the following MUST immediately stop work and escalate to the PM as a P0:**

1. **"trio"** — The operating model is **duo** (two architects). Any reference to a trio is contamination.
2. **"deferral"** — We do not defer. Do it now or escalate.
3. **"Architect Alpha" / "Architect Beta" / "Architect Gamma"** — Named architect roles are banned.
4. **Cargo-cult security** — Adding unrequested auth, OAuth, bearer tokens, CSRF, rate limiting, etc. The security boundary is APIM at the edge. The Rust backend is localhost loopback that trusts its caller. If you reach for `axum::middleware`, `Authorization` headers, or any crate with "auth" in its name — stop. (8 sprints burned 3 days on this; all ripped out.)
5. **Hidden inference / invisible ACP sessions** — Any inference path the operator can't inspect, pause, and kill. If maenifold is reasoning, the calling MCP client MUST see it via the canonical `sampling://sessions` resource.
6. **ACP / Jockey inside the MCP gateway path** — Three distinct protocols, three distinct roles:
   - **MCP** = universal client interface into the daemon.
   - **ACP/Jockey** = product runtime for user-visible CLI agents driven from pipelines/workflows. NOT gateway infrastructure.
   - **Inferencing** = direct LLM API access for gateway tools when the client lacks `sampling/createMessage`.
   
   `GatewayIntent` has exactly two resolver paths: (a) MCP client sampling, (b) direct inferencing. There is no third path. If you reach for `AgentsConfig::load`, `jockey::acp::execute_runtime`, or any path requiring a `maenifold.json` agent entry to resolve a GatewayIntent — stop.
7. **WSL on Windows** — This repo is developed on **native Windows**. NO WSL. Never invoke `wsl`, `C:\Windows\System32\bash.exe`, or any `/mnt/c/...` path-based tooling. The bash helper scripts under `ralph/scripts/*.sh` are Unix-only and MUST NOT be run on Windows (e.g. `worktree-create.sh` via system32 bash produces a worktree with a `/mnt/c/...` gitdir that native git cannot open). On Windows use the native toolchain only: PowerShell, `git worktree add`, `mklink /J` for junctions, `rebuild.ps1`, and the MSVC Rust toolchain. If a task can only be done via WSL/bash, stop and escalate — do not reach for WSL.

**Violation response:** Stop all work. Identify the source. Purge the reference. Report to PM. Resume only after confirmed removed.

---

## Trust Model

**We do not trust. We verify.**

0. **Backend verification is non-negotiable.** Every behavior change must be exercised against the real daemon binary (`./rebuild.sh` then probe via MCP). Not unit tests alone. Not "the diff looks right." Curl the gateway, call the tool, read the resource.
1. If any gate stage is skipped, faked, or fast-forwarded — the **entire sprint is discarded**.
2. An agent that fabricates a verdict is terminated.
3. The PM never authors notes for another role's stage.
4. "Probably fine" is not a trust level. Unverified = untrusted.
5. Agent self-reports are unreliable. Verify against actual artifacts.
6. **Agents NEVER touch git.** Read-only only (`git log`, `git diff`, `git status`). PM owns the working tree and commits.

---

## Core Tenets

- **AI is the New UI.** MCP is the universal interface. If an agent can't do it through MCP, a human can't either.
- **We Do Not Write Code.** Research first. Prefer libraries, reference impls, prior art.
- **Mechanism, Not Policy.** No silent defaults. Every policy choice explicit and required.
- **Simple, Lovable, Complete.** No stubs, mocks, scaffolds, placeholders, or fake surfaces.
- **No Backwards Compatibility.** User base of one. Compat shims are P0 failures.
- **Help Before Workflow.** Show intent and outputs before exposing orchestration.
- **No Patches on Patches.** When the fix lives outside our codebase, escalate to the user with cited evidence (which dependency, which upstream issue, what the user must decide). Don't shim. Don't silently close.
- **Spec-completism is not a justification.** "The spec says SHOULD" is NOT a reason to ship. Name the production caller or don't build it.
- **NO PUNTING.** Discovery → file + dispatch → ship, THIS ITER. Filing without dispatching is punting. Surfacing in thoughts.md is punting. Recommending is punting. Every task has exactly three terminal dispositions: **FIX-NOW** (do the work this iter), **ESCALATE-TO-USER** (named decision required — cite the evidence and the exact question), or **DELETE-AS-NOT-A-TASK** (the file describes noise, not work — grep/LSP proof that the referenced code, behavior, or premise does not exist; remove the file). There is no "close on cited evidence," no "WONTFIX," no "accept residual," no "defer." If the work needs doing and you can't do it, name the human who must decide and ask them this iter.

---

## The MCP Boundary

**Through MCP:** Workflows, memory, pipelines, gate tasks, graph data, all domain state.

**Direct access OK:** `maenifold.json` (preferences only), ACP host filesystem/terminal per protocol spec.

**MCP tools are the primary interface.** Agents use `GatewayExecute`, `GatewayIndex`, `GatewayBatch`, `GatewayIntent`, `complete_node`, `ReadMcpResource`, and `GetHelp` as their first choice. If the MCP tools aren't exposed in the session, start the daemon (`launchctl kickstart -k gui/$(id -u)/com.ma-collective.maenifold` on macOS, or `./rebuild.sh` for a full redeploy) and re-check the tool inventory. If they still aren't reachable, fall back to the in-process CLI: `maenifold tool <NAME> '<JSON-PAYLOAD>'`. If neither path works, fail fast and report — never curl `/mcp` directly, never hand-roll an MCP HTTP envelope, never reintroduce a `mae_mcp`-style helper.

**No REST data APIs.** The daemon exposes `/mcp` for MCP JSON-RPC. The only non-MCP HTTP endpoints are infrastructure: `/api/health`, `/api/machine-id`, `/api/config/*` (file I/O), `/api/user-prompt/*` (MCP sampling bridge), `/api/acp/session` (ACP WebSocket transport). All domain data flows through MCP.

**Localhost trust model — no in-process access control.** MCP gateway tools accept caller-supplied parameters at face value. The security boundary is APIM, not Rust middleware. Do not add validation, ACLs, or auth to the MCP path.

---

## Gate — The Law

Gate is sacrosanct. No exceptions. No shortcuts.

1. **NEVER move files between `todo/` and `done/`.** Only `gate_Pass` on the terminal stage moves a file.
2. **`todo/` and `done/` are NOT tracked in git.** Gate state is per-clone. `.gate/gate.db` is a derived read-model rebuilt by `gate_Sync`.
3. **Recovery:** `.gate/gate.db` rebuilds via `gate_Sync` with `rebuild: true`. If both markdown and cache are lost, PM rebuilds from code commits.
4. **Task ID format:** `T-{sprint}.{task}` or `T-{sprint}.{task}.{sub}`. Sub-tasks start at 1.
5. **Every stage requires a real dispatched agent.**
6. **Never skip or fast-forward.** Each `gate_Pass` references genuine output.
7. **1 violation = sprint discarded.**
8. **PM skips stages ONLY with explicit written user authorization.**
9. **`task-pm` sub-PMs own intermediate stages.** Top PM dispatches one `task-pm` per ready Gate item; inner-loop workers call `gate_Append` only.
10. **`task-pm` escalates on five triggers:** per-stage cap (3), regression cap (2), total dispatch cap (5), wall-clock (30 min), confession-evidence drift (2 consecutive).
11. **One task per `task-pm`.**
12. **No "waived re-audit."** Every remediation gets a full independent re-audit.
13. **PM circuit breaker.** >3 tasks in remediation → stop new dispatch, land in-flight work.
14. **One-iter block ceiling.** Blocked >1 iter without forward motion is a defect. PM scans for blocked tasks each iter: dispatch a fix, close on evidence, or ask the user this iter.

### Gate Processes

**TDD is the default.** SDD is the fallback (author must justify). `lightweight` is deprecated.

| Process | Stages | When |
|---------|--------|------|
| **tdd** | pending → blue-team → swe → red-team → done | Default |
| **sdd** | pending → swe → red-team → blue-team → done | Simple changes |
| **slc** | pending → discovery → spec → swe → red-team → blue-team → done | Cross-crate / new capability |
| **gsd** | pending → research → plan → execute → verify → done | Research-first |

### Gate Gateway

```text
GatewayExecute domain="maenifold.gate" command="gate_Sync" parameters={root: "/path/to/repo", rebuild: true}
GatewayExecute domain="maenifold.gate" command="gate_Status" parameters={root: "/path/to/repo"}
GatewayExecute domain="maenifold.gate" command="gate_Pass" parameters={root: "/path/to/repo", taskId: "<task-id>", agent: "<name>", notes: "..."}
GatewayExecute domain="maenifold.gate" command="gate_Fail" parameters={root: "/path/to/repo", taskId: "<task-id>", agent: "<name>", notes: "..."}
```

---

## Agent Rules

### Authorization

**The PM operates autonomously.** The user sets direction; the PM executes continuously.

**Dogfood Pipeline when it fits, direct-dispatch when it doesn't.** Don't paint yourself into a corner where the broken tool is the only way to fix the broken tool.

**The PM doesn't idle.** Prep next worktree, draft next prompt, read next ready item while work runs.

**Gate is a kanban board.** Every pending task with no unmet deps is eligible. Scan the full board, not just the current sprint.

### SWE Verification Requirements

SWE agents are unreliable narrators. Mandatory behaviors:

1. **Verify edits persisted.** Re-read modified lines after every edit.
2. **Trace render path before UI work.** Use LSP `findReferences` to confirm the component is in the active path.
3. **Report actual changes.** "No changes needed" requires PM review — highest-risk claim.

### Sub-PM (`task-pm`) — DEPRECATED

The `task-pm` tier is deprecated and forbidden. The outer-loop PM owns the per-stage acceptance loop directly.

### Agent Type Mapping

| Gate Role | Agent Type | Model | What it does |
|-----------|-----------|-------|--------------|
| SWE | `inner-loop` | `gpt-5.4` | Scoped code execution in an isolated worktree. Returns WorkResult. |
| Red-team (reasoning) | `rubber-duck` | `gpt-5.5` | Read modified files + reason about correctness. No test execution. For design/logic review. |
| Red-team (diff) | `code-review` | `gpt-5.5` | Analyze branch diff for bugs, security, logic errors. For refactors, deletions, mechanical changes. |
| Blue-team | `inner-loop` | `gpt-5.4` | Write and run tests for changed code. |
| Wave-boundary review 1 | `rubber-duck` | `gpt-5.5` | Independent critique at milestone boundaries. |
| Wave-boundary review 2 | `rubber-duck` | `claude-opus-4.6` | Second independent critique, different model. |
| PM (outer-loop) | — | — | You. Owns Gate transitions, git, worktrees, acceptance loops. |

### Test Policy

All compilation — including test compilation — goes through the rebuild scripts, which set the `MAENIFOLD_REBUILD=1` token the build guard requires (see Build). The role limits below describe **scope** (what each role may compile/test), not a license to invoke `cargo` out of band: run cargo under the rebuild-script environment, never bare.

| Role | Allowed scope | Forbidden |
|------|---------------|-----------|
| SWE | Build only the assigned crate | `cargo test`, any test execution; bare out-of-band `cargo` |
| Red-team | Read code, reason | Execute tests; any compile |
| Blue-team | Test only the changed code | Full workspace test runs; bare out-of-band `cargo` |
| PM | Build/test the touched crate at sprint close | Mid-sprint test runs; bare out-of-band `cargo` |

### Scoping

- **Red-team:** ONLY files modified by the SWE. Pre-existing issues → informational, not FAIL.
- **Blue-team:** ONLY tests for changed code.
- **SWE:** The task is the task. Adjacent problems → report, don't fix.

### Parallelization — Default to Parallel

**Every unblocked Gate task should have an in-flight `task-pm` up to the concurrency ceiling.** Serial requires a cited reason (same-file contention, daemon-redeploy, Rust build lock).

**Per-agent build isolation:** `CARGO_TARGET_DIR=target/agents/<task-id>`. Workspace-global `target/` reserved for blue-team and the human. Caches evicted after 7 days by `scripts/clean-stale-agent-caches.sh`.

### Formatting

SWEs format only files they created or modified. Workspace-wide formatters are forbidden.

### Rendering Fallbacks

Agent-facing markdown MUST render in terminal MCP clients. No ` ```mermaid ` (use ASCII fallback in `<details>`), no bare images (describe inline), no LaTeX (use unicode/plain text).

---

## Severity

- **P0** — Compile errors. Breaks `cargo build --release` or `cargo test --no-run`. PM handles personally.
- **P1** — Active work or blocked by P0.
- **P2** — Discovered items. Backlog until prioritized.

---

## Command Execution Discipline

**No commands without timeouts.** No tests without timeouts. No polling loops without explicit caps. Workers must not write retry loops — make ONE attempt, report failure, stop. Load the `maenifold-testing` skill for timeout primitives, per-tool budgets, and the no-polling-loop clause.

---

## Definition of Done

**Backend:** `cargo build --release -p <crate>` succeeds, and the resulting binary is exercised through MCP against the running daemon (curl `/mcp` or call from an MCP client). Debug builds and unit tests alone are not delivery.

**Release build works or it's not done.**

**Side-effect contract:** Build/test MUST NOT leave orphan processes (>50% CPU) or grow `target/` past 50 GB.

---

## Build

### The only approved build path: the rebuild scripts

Compilation happens **only** through the rebuild scripts. They are the one approved build entry point on each OS:

```bash
./rebuild.sh                              # macOS / Linux
pwsh -NoProfile -File .\rebuild.ps1       # Windows
```

**Direct `cargo build` / `cargo test` / `cargo check` / `cargo run` is "out of band" and is forbidden.** A repo-level build guard enforces this: the `build.rs` scripts in `maenifold-core`, `maenifold-gate-core`, and `maenifold-jockey` abort any compile that does not carry the `MAENIFOLD_REBUILD=1` token, which **only** the rebuild scripts set. An out-of-band compile fails fast with a pointer back to this section. rust-analyzer's background pre-pass is exempt (it sets `RA_RUSTC_WRAPPER=1`), so LSP intelligence keeps working. Do not set `MAENIFOLD_REBUILD` by hand to bypass the guard — that token is reserved for the rebuild scripts.

This is the canonical redeploy. It builds every crate in release, verifies the `/usr/local/bin/maenifold` symlink (macOS), and `launchctl kickstart`s the launchd service. **There is no separate "restart the daemon" step.** `/usr/local/bin/maenifold` is a symlink to `target/release/maenifold`, so the live daemon binary updates the moment the release build finishes, and the launchd kickstart picks up the new bits on next request. Never ask the user "should I redeploy myself or do you want to?" — just run the rebuild script for the OS.

### Non-compiling helpers

```bash
make audit                                          # all constitution linters (no compile)
make audit-crates                                   # Rust: 500 LOC, no TODO, no unwrap (no compile)
```

Binaries produced by the rebuild scripts: `target/release/maenifold` (symlinked to `/usr/local/bin/maenifold` on macOS) and `target/release/maenifold-pipeline-driver`.

The daemon hosts MCP/HTTP; the driver hosts pipeline node execution. ANY change touching `crates/maenifold-core/src/workflow/` requires rebuilding both — which the rebuild scripts do. After rebuild + restart, terminate stale driver PIDs one by one.

---

## Code Standards

- **500 LOC hard limit** on Rust source files. Build-enforced.
- **Research before building.** Hand-rolling a library's job is a P0 escalation.
- **LSP is preferred** for Rust navigation (rust-analyzer). Load the `maenifold-testing` skill for server status and workarounds.
- **MCP-end-to-end verification.** A unit-tested handler with no production caller is not a feature.
- **No TODO/FIXME/HACK.** File a gate task instead. Build-enforced.
- **No .unwrap() in non-test Rust.** Build-enforced.
- **In-process tests ≠ interop proof.** `tokio::io::duplex` proves serialization, NOT that a foreign peer accepts the bytes. Test names promising end-to-end must exercise the real transport and peer.

### Decision Rubrics

| Signal | Action |
|--------|--------|
| Bypassing MCP for domain data | **P0.** |
| ACP/Jockey inside MCP gateway resolver | **P0.** |
| Hand-rolling a library's job | Stop. Escalate. |
| Mock, stub, scaffold, placeholder | Delete. Escalate. |
| Silent default or hidden recovery | Make explicit and required. |
| Compat shim or dual-mode code | **P0.** Delete. Change everywhere. |
| Handler with no production caller | **Not done.** Either wire it to a caller or delete it. |
| Dead-premise task | **Delete the task file this iter** with grep/LSP proof the premise does not exist. |
| Fix lives in code we don't own | **Escalate to user this iter** — name the dependency, the upstream issue, and what they must decide. Don't shim. |
| Blocked >1 iter | Close or dispatch this iter. |
| Instinct: "file for next PM" | **Stop. That's punting.** Fix this iter. |
| Instinct: "should I redeploy or do you want to?" | **Stop. Just run `./rebuild.sh`.** It is the one-shot full redeploy. The binary is a symlink to `target/release/maenifold`; rebuild.sh builds + restarts launchd. Never ask. |
| Instinct: "I'll just `cargo build`/`cargo test`/`cargo check`" | **Stop. That's out of band and the build guard will reject it.** Compile only via `./rebuild.sh` (macOS/Linux) or `pwsh -NoProfile -File .\rebuild.ps1` (Windows). |

---

## ACP

- Architecture: [docs/acp-client-architecture.md](docs/acp-client-architecture.md) is law.
- Wholesale lifts: follow `docs/wholesale-lift-protocol.md`.
- **`prompt` is the only primitive.** Think and workflow chain prompt calls. No direct agent path.
- All ACP calls MUST specify `model` explicitly.
- WebSocket URI requires `?agent=<name>`. Missing → reject.
- **Agent-name resolution:** `?agent=<name>` must match `~/maenifold/maenifold.json` `agents` block (case-insensitive). Unrecognized → HTTP 400.
- **Versions:** Rust `sacp` `11.0.0`, `rmcp` `1.4.0`.

---

## Protected Files

- `assets/integrations/copilot/plugin-maenifold/.mcp.json` — dot prefix intentional, never rename.

---

## Forensics references

When stealing components from prior maenifold UI eras (Pipeline / Workflow / Onboarding wizard / Settings / etc.), use these checked-out snapshots — DO NOT re-resurrect them from git history each time:

- **`C:\src\maenifold\worktrees\old-ui-vue\apps\desktop\`** — Vue 3 + Vite + Pinia + Electron (forge). Detached HEAD at `0080ca9a2` (snapshot of `main` pre-`06ad66f15` vanilla-rip).
  - 204 Vue components: 24 Pipeline*, 20 Workflow*, 7 Onboarding* (+ `OnboardingWizard.vue`), 6 Setting*, 147 other.
  - shadcn-vue + Tailwind v4 + Lucide + Vue-flow + cytoscape + mermaid + KaTeX + Shiki.
  - **Build proven:** `cd apps/desktop && npx vite build` -> clean `dist/` in ~3s. `npm install --ignore-scripts --no-audit --no-fund` already done (1410 packages, 764MB).
  - Inventory map: `~/.copilot/session-state/ab143dec-cee0-4810-b5ca-fa1f56287c15/files/T-412/old-ui-inventory.md`
  - Provenance: rip commit `06ad66f15` explicitly preserved this state at `main` snapshot `0080ca9a2`.

- **Recent React/shadcn Pipeline Viewer (T-251.13)** — `apps/maenifold-console/` at `8d57b589a^`. React + Vite + Tailwind v4 + shadcn + React Flow. 69 files. NOT currently checked out as a worktree (smaller scope, can resurrect on demand).

To clean up the forensics worktree when done: `git worktree remove C:\src\maenifold\worktrees\old-ui-vue`.

---

## Enforcement

Any agent may block or fail work that violates this constitution. Every bypass, fake-success, or verification gap becomes a gate item. Repeated violation → task rollback, escalation, narrower re-scope.

We dogfood maenifold and gate. The standard is not "can we explain it" — it's "would we trust using it ourselves."
