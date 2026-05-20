# maenifold — Constitution

One product, one monorepo. `crates/` = Rust backend. `apps/desktop/` = Electron + Vue frontend.

## Key Documents

- **[UAT Plan](docs/UAT-PLAN.md)** — Exhaustive UI acceptance test plan (679 test cases). Required reading before any UI work ships.

---

## P0 Escalation Triggers

**Any agent encountering ANY of the following MUST immediately stop work and escalate to the PM as a P0:**

1. **"trio"** — The operating model is **duo** (two architects). Any reference to a trio is contamination.
2. **"deferral"** — We do not defer. Do it now or escalate.
3. **"Architect Alpha" / "Architect Beta" / "Architect Gamma"** — Named architect roles are banned.
4. **Programmatic UAT** — Any attempt to auto-generate test checklists or gate tasks from UAT plan parsing. The UAT plan is for interactive CDP testing, not code generation. Sprint 229 shipped broken this way.
5. **Cargo-cult security** — Adding unrequested auth, OAuth, bearer tokens, CSRF, rate limiting, etc. The security boundary is APIM at the edge. The Rust backend is localhost loopback that trusts its caller. If you reach for `axum::middleware`, `Authorization` headers, or any crate with "auth" in its name — stop. (8 sprints burned 3 days on this; all ripped out.)
6. **Hidden inference / invisible ACP sessions** — Any inference path without a corresponding visible UI element the user can inspect, pause, and kill. If maenifold is reasoning, the user MUST see it.
7. **ACP / Jockey inside the MCP gateway path** — Three distinct protocols, three distinct roles:
   - **MCP** = universal client interface into the daemon.
   - **ACP/Jockey** = product runtime for user-visible CLI agents. NOT gateway infrastructure.
   - **Inferencing** = direct LLM API access for gateway tools when the client lacks `sampling/createMessage`.
   
   `GatewayIntent` has exactly two resolver paths: (a) MCP client sampling, (b) direct inferencing. There is no third path. If you reach for `AgentsConfig::load`, `jockey::acp::execute_runtime`, or any path requiring a `maenifold.json` agent entry to resolve a GatewayIntent — stop.

**Violation response:** Stop all work. Identify the source. Purge the reference. Report to PM. Resume only after confirmed removed.

---

## Trust Model

**We do not trust. We verify.**

0. **UI verification is non-negotiable.** Every UI change must be interactively tested in the real packaged Electron app via CDP. Not screenshots. Not typechecks. Click every button. File gate items for anything wrong — triage later.
1. If any gate stage is skipped, faked, or fast-forwarded — the **entire sprint is discarded**.
2. An agent that fabricates a verdict is terminated.
3. The PM never authors notes for another role's stage.
4. "Probably fine" is not a trust level. Unverified = untrusted.
5. Agent self-reports are unreliable. Verify against actual artifacts.
6. **Agents NEVER touch git.** Read-only only (`git log`, `git diff`, `git status`). PM owns the working tree and commits.
7. **No programmatic UAT.** The UAT plan is read by the agent doing interactive CDP testing. It tells them what to click. That's it.

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

**Monorepo ≠ architecture waiver.** No path dependencies from desktop app to backend domain crates.

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

### Sub-PM (`task-pm`) Verification

- **Worktree ownership:** Top PM creates worktree BEFORE dispatching.
- **Spec + CDP port:** Passed in the dispatch prompt.
- **Escalation triage:** Re-author / decompose / delete-as-not-a-task / senior-dispatch / escalate-to-user. Never re-dispatch same spec.
- **Terminal pass + merge:** Verify artifact, merge to `dev`, remove worktree, `gate_Pass`.

### Test Policy

| Role | Allowed | Forbidden |
|------|---------|-----------|
| SWE | `cargo build -p <crate>` | `cargo test`, any test execution |
| Red-team | Read code, reason | Execute tests |
| Blue-team | `cargo test -p <crate> <test_name>` | Full workspace test runs |
| PM | `cargo test -p <crate>` at sprint close | Mid-sprint test runs |

### Scoping

- **Red-team:** ONLY files modified by the SWE. Pre-existing issues → informational, not FAIL.
- **Blue-team:** ONLY tests for changed code.
- **SWE:** The task is the task. Adjacent problems → report, don't fix.

### Frontend worktrees

Workers are FORBIDDEN from running `npm install`, `npm ci`, `npm rebuild`, or `npx electron-rebuild` inside any worktree whose `apps/desktop/node_modules` is a symlink.

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

**No commands without timeouts.** No tests without timeouts. No CDP waits without timeouts. No polling loops without explicit caps. Workers must not write retry loops — make ONE attempt, report failure, stop. Load the `maenifold-testing` skill for timeout primitives, per-tool budgets, and the no-polling-loop clause.

---

## Definition of Done

**Backend:** `cargo build --release -p <crate>` succeeds. Debug builds are not delivery.

**UI:** Typecheck passes + app launches + visual CDP verification. `vue-tsc --noEmit` alone is NOT sufficient. CDP UAT is normal dispatchable work, not a quarantined activity.

**Both:** Release build works or it's not done.

**Side-effect contract:** Build/test MUST NOT leave orphan processes (>50% CPU) or grow `target/` past 50 GB.

---

## Build

### Deploy (the one and only command)

```bash
./rebuild.sh
```

This is the canonical redeploy. It builds every crate in release, verifies the `/usr/local/bin/maenifold` symlink, packages the Electron app, and `launchctl kickstart`s the launchd service. **There is no separate "restart the daemon" step.** `/usr/local/bin/maenifold` is a symlink to `target/release/maenifold`, so the live daemon binary updates the moment the release build finishes, and the launchd kickstart picks up the new bits on next request. Never ask the user "should I redeploy myself or do you want to?" — just run `./rebuild.sh`.

### Focused targets (when you don't need a full rebuild)

```bash
cargo build --release -p maenifold-cli              # CLI/app only
cargo build --release -p maenifold-pipeline-driver  # driver only
cd apps/desktop && npm run package                  # Electron bundle only
cd apps/desktop && npx vue-tsc --noEmit             # frontend typecheck only
make audit                                          # all constitution linters
make audit-crates                                   # Rust: 300 LOC, no TODO, no unwrap
make audit-frontend                                 # Vue/TS: lint-design + audit-components
```

Binaries: `target/release/maenifold` (symlinked to `/usr/local/bin/maenifold`) and `target/release/maenifold-pipeline-driver`.

The daemon hosts MCP/HTTP; the driver hosts pipeline node execution. ANY change touching `crates/maenifold-core/src/workflow/` requires rebuilding both. After rebuild + restart, terminate stale driver PIDs one by one.

---

## Code Standards

- **300 LOC hard limit** on source files. Build-enforced.
- **Research before building.** Hand-rolling a library's job is a P0 escalation.
- **LSP is preferred** for Rust/TS/Vue navigation. Load the `maenifold-testing` skill for server status tables and workarounds.
- **Full-stack spec verification.** Frontend without backend plumbing is not a feature.
- **No TODO/FIXME/HACK.** File a gate task instead. Build-enforced.
- **No console.log/debug/warn.** Build-enforced (frontend).
- **No mocks/stubs/fakes.** Build-enforced (frontend).
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
| UI compiles but not visually verified | **Not done.** |
| Dead-premise task | **Delete the task file this iter** with grep/LSP proof the premise does not exist. |
| Fix lives in code we don't own | **Escalate to user this iter** — name the dependency, the upstream issue, and what they must decide. Don't shim. |
| Blocked >1 iter | Close or dispatch this iter. |
| Instinct: "file for next PM" | **Stop. That's punting.** Fix this iter. |
| Instinct: "should I redeploy or do you want to?" | **Stop. Just run `./rebuild.sh`.** It is the one-shot full redeploy. The binary is a symlink to `target/release/maenifold`; rebuild.sh builds + packages + restarts launchd. Never ask. |

---

## ACP

- Architecture: [docs/acp-client-architecture.md](docs/acp-client-architecture.md) is law.
- **ACP UI reference:** sibling checkout `../acp-ui` (`formulahendry/acp-ui.git`, commit `27e32022`). Compare before changing session/reconnect/stream handling.
- Wholesale lifts: follow `docs/wholesale-lift-protocol.md`.
- **`prompt` is the only primitive.** Think and workflow chain prompt calls. No direct agent path.
- All ACP calls MUST specify `model` explicitly.
- WebSocket URI requires `?agent=<name>`. Missing → reject.
- **Agent-name resolution:** `?agent=<name>` must match `~/maenifold/maenifold.json` `agents` block (case-insensitive). Unrecognized → HTTP 400.
- **Versions:** TS `@agentclientprotocol/sdk` `^0.18.2`. Rust `sacp` `11.0.0`, `rmcp` `1.4.0`.

---

## Desktop UI

**Design system:** `apps/desktop/DESIGN.md` is the sole spec. Read it before touching any Vue component.

**Stack:** shadcn-vue + Tailwind CSS v4 + Reka UI. Icons: `lucide-vue-next`.

**Build-enforced rules:**

- `vue-tsc --noEmit` — type safety.
- `audit:components` — no raw HTML controls, no sibling .css, no btn-*, 300 LOC, no `<style>`. `audit-ok` is banned.
- `lint:design` — no hex colors, no component-local borders.

**Layout z-index:** canvas base → backdrop `9` → inspector `10` → flyout `11` → context menu `60` → dialogs `1000`.

**CDP Testing:** Load the `maenifold-testing` skill for launch procedures, env vars, agent-browser discipline, known harness gaps, and timeout rules.

---

## Protected Files

- `assets/integrations/copilot/plugin-maenifold/.mcp.json` — dot prefix intentional, never rename.

---

## Enforcement

Any agent may block or fail work that violates this constitution. Every bypass, fake-success, or verification gap becomes a gate item. Repeated violation → task rollback, escalation, narrower re-scope.

We dogfood maenifold and gate. The standard is not "can we explain it" — it's "would we trust using it ourselves."
