# maenifold — Constitution

One product, one monorepo. `crates/` = Rust backend. `apps/desktop/` = Electron + Vue frontend.

## Key Documents

- **[UAT Plan](docs/UAT-PLAN.md)** — Exhaustive UI acceptance test plan (679 test cases). Every screen, dialog, button, input, dropdown. Duo-approved. Required reading before any UI work ships.

---

## P0 Escalation Triggers

**Any agent encountering ANY of the following MUST immediately stop work and escalate to the PM as a P0:**

1. **"trio"** — The word "trio" in any context referring to architect roles. The operating model is **duo** (two architects). There is no trio. There never was a trio. Any reference to a trio is a contamination artifact that must be purged from whatever source produced it.
2. **"deferral"** — Any suggestion to defer, postpone, or backlog work. We do not defer. We either do it now or we escalate. Deferral is not an option.
3. **"Architect Alpha" / "Architect Beta" / "Architect Gamma"** — Named architect roles are banned. The duo is anonymous and model-specified per the Roles table. Individual architect names in any active document are contamination.
4. **Programmatic UAT** — Any attempt to auto-generate test checklists, gate tasks from UAT plan parsing, batch TC-ID verification items, or any form of "programmatic" acceptance testing. You cannot unit-test AI. You cannot batch-verify a UI. The UAT plan is a human-authored guide for interactive CDP testing — not a data source for code generation. Any agent that generates gate tasks by parsing `docs/UAT-PLAN.md` is producing rubber-stamp busywork. This is how sprint 229 shipped broken. Immediate P0. Stop all work.
5. **Cargo-cult security** — Adding any auth, authn, authz, OAuth, bearer tokens, signed cookies, CSRF tokens, rate limiting, request signing, encryption-at-rest, or any other "security feature" the user did not explicitly request. **The security boundary is Azure API Management (APIM) at the edge.** Anything we do in code is inferior to APIM and counter-productive. The Rust backend is a localhost loopback service that trusts its caller. Period. Adding code-level auth is not "defense in depth" — it is wasted effort that breaks legitimate clients, multiplies attack surface, and pushes policy decisions into the wrong layer. **Cautionary tale:** sprints 220, 229, 238.47, 240.4, 240.6, 240.8, 240.9, and 242.11 burned roughly three days implementing a localhost OAuth bridge with PKCE, dynamic client registration, well-known metadata, and bearer token middleware — none of which the user asked for, all of which broke the MCP HTTP transport, and all of which had to be ripped back out. If you find yourself reaching for `axum::middleware`, `Authorization` headers, `WWW-Authenticate`, `.well-known/oauth-*`, or any crate with "auth" in its name — stop. File a P0. Get explicit user authorization before writing one line.
6. **Hidden inference / invisible ACP sessions** — Any code path that spawns an ACP session, dispatches to an LLM, runs a sequential-thinking turn, or performs any form of inference WITHOUT a corresponding visible UI element the user can inspect, pause, and kill. **Critical safety constraint.** If maenifold is reasoning, the user MUST see what it is reasoning about, which agent + model is doing the reasoning, the live token stream, and have a one-click stop control. This applies to: gateway intent resolver dispatches, workflow agent dispatches, pipeline node dispatches, sub-PM agent dispatches, background warm-up sessions, schedule-driven runs, and any other automated inference path — current or future. A black-box inference path is a runaway-LLM risk and an autonomy violation. The desktop app's Sessions / ACP inspector surface is the canonical UI for this; if a new inference path doesn't appear there, the path is broken until it does. Discovery of any hidden inference is an immediate P0 — stop the path, file a gate item, surface it in the UI before it ships again.

**Violation response:** Stop all work. Identify the source (file path, memory:// URI, session checkpoint, red-team finding ID). Purge the reference. Report to PM. Resume only after the contamination is confirmed removed.

---

## Trust Model

**We do not trust. We verify. This is the most important rule.**

0. **UI verification is non-negotiable.** Every UI change must be interactively tested in the real packaged Electron app using `agent-browser` via CDP. Not screenshots. Not typechecks. Click every button, open every dialog, verify every response. Assume the UI is broken and you haven't found the problem yet. File gate items for anything wrong — triage later, not now.
1. If any gate stage is skipped, faked, or fast-forwarded — the **entire sprint is discarded**. All work reverted. No exceptions regardless of correctness.
2. An agent that fabricates a verdict is terminated. All its output is treated as suspect.
3. The PM never authors notes for another role's stage. The PM relays real agent verdicts — nothing else.
4. "Probably fine" is not a trust level. Unverified = untrusted.
5. Agent self-reports are unreliable. Every claim must be independently verified against the actual artifact (file, build output, interactive CDP session).
6. **Agents NEVER touch git.** No `git checkout`, `git reset`, `git restore`, `git stash`, `git pull`, `git rebase`, `git merge`, `git clean`. Read-only is fine (`git log`, `git diff`, `git status`). PM owns the working tree and commits. An agent that runs a destructive git command discards every parallel agent's working-tree changes — this has happened in real sprints (T-174.4 lost its executor.rs edits when a peer ran `git checkout bf682ecc`).
7. **No programmatic UAT. No generated test checklists. No "TC-001 through TC-666" gate tasks.** You cannot unit-test AI behavior. The UAT plan (`docs/UAT-PLAN.md`) is a human-authored reference document for human-driven interactive testing via CDP. It is NOT a source for auto-generating gate tasks, checklists, or batch verification items. Any agent that generates gate tasks by parsing the UAT plan and creating one-task-per-section or one-task-per-TC is producing busywork that will be rubber-stamped, not verified. That is exactly how sprint 229 shipped broken — agents checked boxes instead of clicking buttons. The UAT plan is read by the agent doing interactive CDP testing. It tells them what to click. That's it.

---

## Core Tenets

- **AI is the New UI.** MCP is the universal interface. No backdoors for UI, scripts, or devs. If an agent can't do it through MCP, a human can't either.
- **We Do Not Write Code.** Research first. Prefer libraries, reference impls, prior art. New code requires PM authorization with a cited reference and a reason adaptation was insufficient.
- **Mechanism, Not Policy.** No silent defaults. Every policy choice explicit and required. Hidden recovery is a bug.
- **Simple, Lovable, Complete.** No stubs, mocks, scaffolds, placeholders, or fake surfaces.
- **No Backwards Compatibility.** User base of one. When the contract changes, everything changes. Compat shims are P0 failures.
- **Help Before Workflow.** Show intent, outputs, and what needs review before exposing orchestration primitives.
- **WONTFIX is the default for broken premises.** A task whose premise is architecturally invalid, references a thing that doesn't exist, asks us to fix code we don't own, or otherwise can't ship as written is **closed**, not held. The PM's default action is to recommend WONTFIX closure with a one-paragraph rationale and move on — NOT to escalate, hold for input across multiple iters, file a translation layer to bridge the impossible, or build a workaround to make the broken task look shippable. Recommend WONTFIX **in the same iter the bad premise is discovered**; ship the closure once the user confirms with as little as one word ("close it"). Multi-iter holds on a task whose premise is dead are themselves a process failure — they bloat backlog, hide real work, and burn iter budget on no-op runs. See: 5-iter hold on T-302.65 (`render_chart` tool that doesn't exist), iter-3 closure of T-302.64 (jockey ACP adapter premise architecturally invalid).
- **No patches on patches.** When a task's correct fix lives outside our codebase (upstream library, third-party agent CLI, OS, etc.), do NOT layer a translation, shim, or heuristic in our code to paper over the external bug. Close as WONTFIX in our scope and file the bug upstream (only if user authorizes). The maintenance cost of guess-and-hope translation layers exceeds the rendering ugliness they hide. Counter-example: the T-302.64 proposal to build a `client.rs` translation layer for upstream agent CLI message shapes was correctly rejected as fragile.

---

## The MCP Boundary

**Through MCP:** Workflows, memory, pipelines, gate tasks, graph data, all domain state.

**Direct access OK:** `maenifold.json` (preferences only), ACP host filesystem/terminal per protocol spec.

**Monorepo ≠ architecture waiver.** No path dependencies from desktop app to backend domain crates. If it's domain truth, it goes through MCP.

**Localhost trust model — no in-process access control.** MCP gateway tools (e.g., `gate_Create`, `gate_Pass`, `gate_Sync`, write tools generally) accept caller-supplied parameters at face value. Specifically: `gate_Create`'s `root` parameter is NOT validated against the desktop app's "selected gate root" — any caller may write tasks under any directory tree it has filesystem permission to. This is intentional per P0 trigger #5: the security boundary is APIM at the edge, not Rust middleware. The Electron IPC path (`gate:write-task-file`) does enforce selected-root for UI-driven creates because the renderer is an untrusted-input boundary; the MCP path is a localhost loopback agents call directly. Do not add validation, ACLs, or auth to the MCP path. (T-248.4 closure rationale, 2026-05-12.)

---

## Gate — The Law

Gate is sacrosanct. The process is followed exactly. No exceptions. No shortcuts.

1. **NEVER move files between `todo/` and `done/`.** Only `gate_Pass` on the terminal stage moves a file. `mv`, `cp`, `git mv`, IDE drag-drop — all forbidden. If you find yourself reaching for a file operation on a task file, stop: the answer is `gate_Pass` or `gate_Fail` through `GatewayExecute` with domain `maenifold.gate`. Period.
2. **`todo/` and `done/` are NOT tracked in git** (gitignored as of 2026-04-28). Gate state is per-developer / per-clone. Markdown task files on disk are the authoritative spec; `.gate/gate.db` is a derived read-model rebuilt by `gate_Sync`. Source of truth for what shipped is the actual code commits + their `T-XXX.X` task-id references in commit messages, NOT the spec markdown. Why the change: parallel worktrees were creating spurious "delete" merge conflicts when one branch's gate moved a file to `done/` while another branch was still working from before the move (T-244.71, T-244.72 incidents). Single-user repo per constitution; backlog is local PM state.
3. **Recovery from gate state loss:** specs are not recoverable from git. The PM regroom from existing code commits + memory:// notes + checkpoint files. `.gate/gate.db` rebuilds from on-disk markdown via `gate_Sync` with `rebuild: true`; if both the markdown AND the cache are lost, the PM rebuilds the spec from scratch using the existing code as evidence of intent.
4. **Task ID format:** `T-{sprint}.{task}` for top-level tasks, `T-{sprint}.{task}.{sub}` for sub-tasks. The sub-task number starts at 1 (never 0). Examples: `T-122.3` is sprint 122 task 3. `T-122.3.1` is the first sub-task of T-122.3.
5. **Every stage requires a real dispatched agent.** SWE→SWE agent. Red-team→red-team agent. Blue-team→blue-team agent.
6. **Never skip or fast-forward.** Each `gate_Pass` references genuine output from the responsible agent.
7. **1 violation = sprint discarded.** All work reverted regardless of correctness.
8. **PM skips stages ONLY with explicit written user authorization.** Record: `stage waived per user authorization at <timestamp>`.
9. **Sub-PMs own SWE stage only.** Dispatch SWE → verify build + delta → record `gate_Pass` for SWE → report back. PM handles red-team/blue-team.
10. **Sub-PMs escalate after 15 minutes.** Silent long runs are process violations.
11. **One task per sub-PM.** Never assign multiple tasks to a single sub-PM.
12. **No "waived re-audit."** Every remediation cycle gets a full independent re-audit. "Waived" is not a valid verdict.
13. **PM circuit breaker.** If >3 tasks enter remediation in one session, stop new dispatch. Land in-flight work first.
14. **One-iter block ceiling.** A task in `blocked` state for more than ONE iteration without forward motion is itself a defect. On entry to any iter, the PM scans for blocked tasks and, for each:
    - If the blocker is a real external dep (user input, upstream release, hardware) → ask the user in plain English **this iter**, with a numbered/lettered choice list including "close as WONTFIX". Do not write another spec amendment; do not file another sub-task; do not hold silently for the next PM. One question, plain English, one iter.
    - If the blocker is "the premise is wrong" → the PM's default recommendation is WONTFIX with a one-paragraph rationale. Surface it to the user in the same iter and ship the closure as soon as authorized.
    - Multi-iter holds compound into invisible backlog rot. T-302.65 sat blocked across 5 iters because each PM assumed the next would resolve it. That is the failure mode this rule exists to kill.
15. **Bad-premise tasks ship as WONTFIX in the same iter the bad premise is discovered.** When PM verification (grep / LSP / architecture trace) proves a task's premise is wrong — feature doesn't exist, fix lives in code we don't own, request contradicts the architecture — the PM:
    1. Documents the finding in the task body (one paragraph; cite the verification evidence — file path + line range + command output).
    2. Surfaces a plain-English WONTFIX recommendation to the user.
    3. On user authorization (one word is enough: "close it"), cascades `gate_Pass` through all pipeline stages attributed to `ralph-pm` with stage-waiver notes citing the user authorization timestamp.
    4. Never invents a workaround (translation layer, shim, heuristic, "Option B") just to make the broken task look shippable. That's how patches grow on patches.

### Gate Processes

**TDD is the default for all work.** SDD is the fallback for simple changes that genuinely don't benefit from a test-first stage. `lightweight` is deprecated — do not author new tasks against it. The remaining processes (`slc`, `gsd`) are reserved for the scopes named.

| Process | Stages | When | Status |
|---------|--------|------|--------|
| **tdd** | pending → blue-team → swe → red-team → done | **Default.** Any change with observable behavior. | Preferred |
| **sdd** | pending → swe → red-team → blue-team → done | Fallback for simple changes where TDD adds no signal. Author must justify in the task body. | Fallback |
| **slc** | pending → discovery → spec → swe → red-team → blue-team → done | Cross-crate or new capability. | Specialized |
| **gsd** | pending → research → plan → execute → verify → done | Unknown scope, research-first. | Specialized |
| ~~lightweight~~ | ~~pending → swe → blue-team → done~~ | ~~Mechanical changes~~ | **Deprecated** — convert to `tdd` |

### Gate Gateway

```text
GatewayExecute domain="maenifold.gate" command="gate_Sync" parameters={root: "/path/to/repo", rebuild: true}
GatewayExecute domain="maenifold.gate" command="gate_Status" parameters={root: "/path/to/repo"}
GatewayExecute domain="maenifold.gate" command="gate_Pass" parameters={root: "/path/to/repo", taskId: "<task-id>", agent: "<name>", notes: "..."}
GatewayExecute domain="maenifold.gate" command="gate_Fail" parameters={root: "/path/to/repo", taskId: "<task-id>", agent: "<name>", notes: "..."}
```

Use the native maenifold MCP Gateway surface. Do not fall back to a legacy gate process.

---

## Agent Rules

### Authorization

**The PM operates autonomously.** The PM owns the backlog, dispatches work, and ships without waiting for per-task user authorization. The user sets direction; the PM executes continuously until the backlog drains or the user redirects.

**Dogfood Pipeline when it fits, fall back to direct-dispatch when it doesn't.** We use our own Pipeline tool whenever it can host the work cleanly — that's how we keep the tool honest. But Pipeline has known sharp edges (T-300.1 is a P0 umbrella documenting them), and any time the work is *fixing Pipeline itself* the tool can't host its own repair. In those cases — and any other case where direct-dispatch via the `Task` tool with isolated worktrees is materially safer or simpler — direct-dispatch is fine. Don't paint yourself into a corner where the broken tool is the only way to fix the broken tool.

**Default execution model:** the PM picks the cheapest substrate that gives full visibility:
- **Pipeline tool** — preferred when the DAG is well-formed (same gate process for every node, no need to mid-flight regraft based on agent output, no dependency on an unfinished Pipeline feature)
- **Direct-dispatch with worktrees + visible `Task` agents + PM-owned gate transitions** — the fallback. Same Gate/confession/audit catch points, same isolation, same evidence chain. Not weaker, just lower-level.

**The PM doesn't idle.** While work runs (Pipeline OR direct-dispatch lanes), the PM uses the time to prep the next worktree, draft the next prompt, run PM-only artifact checks, or read the next ready item. "Waiting for a worker to finish" is not a valid PM state.

**Gate is a kanban board.** Every task in `todo/` with state `pending` and no unmet dependencies is eligible for dispatch. The PM scans the full board on every cycle — not just the current sprint. Stale pending items from prior sprints get included or triaged, never ignored.

### SWE Verification Requirements

SWE agents are unreliable narrators. These requirements exist because SWEs have:
- Claimed "no changes needed" when changes were required (T-170.1)
- Reported edits that never persisted in the file (T-170.3, T-170.4)
- Written code into dead/unreachable components (T-170.9)

**Mandatory SWE behaviors:**

1. **Verify edits persisted.** After every file edit, re-read the modified lines to confirm the change is in the file. "Edit complete" without read-back verification is a task failure.
2. **Trace render path before UI work.** Before modifying any Vue component, use LSP `findReferences` to confirm the component is imported and rendered in the active path for the target mode. Dead components are not valid targets.
3. **Report actual changes.** SWE output must list files modified and lines changed. "No changes needed" requires PM review before acceptance — it is the highest-risk SWE claim.

### Sub-PM Verification Requirements

**Sub-PMs are obsolete.** Pipeline DAGs replace them. Tool nodes advance gate state, prompt nodes dispatch agent work, conditional edges handle failure routing. The pipeline engine handles everything sub-PMs used to do.

### Test Policy

| Role | Allowed | Forbidden |
|------|---------|-----------|
| SWE | `cargo build -p <crate>` | `cargo test`, any test execution |
| Red-team | Read code, reason | Execute tests |
| Blue-team | `cargo test -p <crate> <test_name>` | Full workspace test runs |
| PM | `cargo test -p <crate>` at sprint close | Mid-sprint test runs |

### Scoping

- **Red-team:** Audit ONLY files modified by the SWE. Verdict on the delta. Pre-existing issues → informational, not FAIL.
- **Blue-team:** Run ONLY tests for changed code. Unrelated compilation failure → report blockage, recommend waiver.
- **SWE:** The task is the task. Adjacent problems → report, don't fix.
- **Sub-PMs:** Obsolete. Use pipelines instead.

### Frontend worktrees

- **Worker forbidden actions:** workers are FORBIDDEN from running `npm install`, `npm ci`, `npm rebuild`, or `npx electron-rebuild` inside any worktree whose `apps/desktop/node_modules` is a symlink.
- **Reason:** worktree `node_modules` symlinks can mutate the main repo dependency tree and create nested `node_modules/node_modules` contamination.
- **PM prompt check:** PM must verify frontend worker prompts include the forbidden-action clause before dispatch.

### Parallelization — Pipeline DAGs Handle It

The `Pipeline` tool manages concurrency, ordering, and saturation. The PM builds DAGs; the engine runs them up to `maxConcurrency`. Manual wave dispatch is obsolete.

**Pipeline DAG construction:**

- **Priority first, then ID.** Pipelines are built priority bucket by priority bucket. Never include lower priority while higher priority tasks are unfinished.
- **Contention lanes via edges.** Tasks that touch the same file get serial edges between them (chain). Tasks on different files have no edge (parallel).
- **maxConcurrency** caps simultaneous active nodes — set to 6 for typical sprints, 1 for sequential debugging runs.
- **Cross-crate parallelism is the default.** Different crates → no edge needed.
- **Same file is forbidden parallel.** Add a serial edge between them.
- **One agent per CDP session.** Visual audits get a dedicated lane with no parallel siblings.

**Per-agent build isolation.** SWE prompt nodes set `CARGO_TARGET_DIR=target/agents/<task-id>` in their build commands. Workspace-global `target/` is reserved for blue-team and the human. Caches under `target/agents/<task-id>` are evicted after 7 days by `scripts/clean-stale-agent-caches.sh` (dry-run by default; pass `--apply` to delete). Run as part of the iter-end cleanup or via cron.

**Anti-patterns the PM must not exhibit:**

- Manual `Task` tool dispatch when a Pipeline DAG would clearly host the work better (well-formed DAG, no need for mid-flight regraft). Direct-dispatch is fine when Pipeline can't host the work or is itself under repair.
- Building a single-lane pipeline when tasks are file-independent.
- Using `maxConcurrency: 1` without justification.

### Formatting

SWEs format only files they created or modified. Workspace-wide formatters are forbidden.

### Rendering Fallbacks (Lowest Common Denominator)

Agent-facing markdown — prompt templates, workflow assets, MCP `instructions`, agent-served docs, skill READMEs — **MUST render in terminal MCP clients**. Verified failure modes (Copilot CLI, 2026-04-20):

- ` ```mermaid ` fenced blocks → render as raw source.
- `![alt](path)` images → render as nothing.
- LaTeX `$...$` and `$$...$$` math → render as raw source.

**Rule:** for any agent-facing markdown, emit content that works everywhere.

- **Mermaid** → provide an ASCII / unicode box-drawing fallback or plain-prose summary adjacent to the diagram. Wrap the source mermaid in `<details><summary>Diagram (graphical viewers)</summary>` so it doesn't dominate terminal output.
- **Images** → describe the image inline AND include a "open `<absolute path>` in a graphical viewer" instruction. Do not rely on alt text alone.
- **LaTeX** → use unicode (Σ ∫ √ ≤ ∈ α β ∞ π) or plain text (`e^(i*pi) + 1 = 0`). Never emit `$$...$$`.

What renders reliably everywhere: standard markdown (headings, lists, tables, blockquotes, links, code blocks with syntax highlighting, task lists, footnotes), ASCII / unicode box-drawing, emoji and unicode symbols, inline HTML (`<kbd>`, `<details>`/`<summary>`, `<u>`), horizontal rules.

Rich-renderer surfaces (the maenifold desktop app, GitHub web UI, browser-based copilot) WILL render mermaid/math/images correctly — but those surfaces are not the lowest common denominator.

---

## Severity

- **P0** — Compile errors. Breaks `cargo build --release` or `cargo test --no-run`. PM handles personally.
- **P1** — Active work or blocked by P0. Included in pipeline DAGs.
- **P2** — Discovered items. Backlog until prioritized.

---

## Definition of Done

**Backend:** `cargo build --release -p <crate>` succeeds. Debug builds are not delivery.

**UI:** Typecheck passes + app launches + visual verification (human or browser automation confirms controls visible, sized, named, usable). `vue-tsc --noEmit` alone is NOT sufficient.

**Both:** Release build works or it's not done.

**Side-effect contract:** Build/test invocations MUST NOT leave orphan processes (>50% CPU, started during the iter) or grow `target/` past 50 GB. `make audit` enforces both via `audit-runaway-procs` and `audit-disk-budget`. A green test result with a stuck process is a FAIL — file a P0.

---

## Build

```bash
cargo build --release -p maenifold-cli     # CLI/app
cargo build --release -p maenifold-pipeline-driver
cd apps/desktop && npm run package          # Electron bundle
cd apps/desktop && npx vue-tsc --noEmit     # frontend typecheck
./rebuild.sh                                # full rebuild + restart
make audit                                  # all constitution linters (Rust + frontend)
make audit-crates                           # Rust: 300 LOC, no TODO, no unwrap
make audit-frontend                         # Vue/TS: lint-design + audit-components
```

Binaries: `target/release/maenifold` (→ `/usr/local/bin/maenifold`) and `target/release/maenifold-pipeline-driver`.

The daemon binary (`maenifold`) hosts the MCP/HTTP layer; the driver binary (`maenifold-pipeline-driver`) hosts pipeline node execution. ANY change touching `crates/maenifold-core/src/workflow/` requires rebuilding both.

Driver process hygiene: after rebuild + daemon restart, terminate stale `maenifold-pipeline-driver` PIDs (`pgrep -af maenifold-pipeline-driver | awk '{print $1}'`) one by one; old drivers persist for the lifetime of in-flight pipeline executions and keep running the old binary.

---

## 72-Hour Retrospective (2026-04-29 → 2026-05-01)

What broke, what we learned, what changed. Sources: `memory://incidents/20260501-*`, `done/T-244.*`, `done/T-246.*`, `done/T-247.*`, `done/T-248.*`, `done/T-243.*`.

### 1. Local-first storage is not negotiable

**Incidents:** T-248.13 (phantom delete RCA), T-248.16 (pipeline list derivation gap), T-248.19 (OneDrive dataless detection), T-248.23 (DB fallback).

OneDrive on macOS materializes `~/maenifold/memory/pipelines/**/*.md` as `compressed,dataless` placeholders. Hot-path code that called `fs::read_to_string` on these files timed out, and the `list_pipelines` discovery loop wrapped the failure in `else { continue; }` — silently dropping pipelines from the UI. The same OneDrive surface is implicated in the phantom-delete success-without-unlink behavior.

**Lesson — never wrap a read with silent `continue;`.** Any read failure on a hot-path authority surface MUST degrade into a read-model fallback (DB lookup, indexed metadata) AND emit an explicit `unreadable: true` / `degraded: { reason: "..." }` flag so the caller can distinguish a missing entity from an unreadable one. See `memory://research/architecture/statefidelity-external-patterns-for-maenifold-may-2026` for the four external patterns (local-first, dataless-file, Electron preload, soft-delete) this maps to.

### 2. Counts-only success payloads are unverified

**Incidents:** T-248.13, T-248.24 (post-unlink verification), T-248.6 (PM-deleted-pipelines).

Daemon returned `{ deletedSessionId, cleanedUpFiles: N, skippedFiles: 0 }` for two completed pipelines. The PM faithfully reported "2 deleted." The files survived. Forensic note `memory://incidents/20260501-pm-deleted-pipelines-forensic` proved 35 surviving artifacts. The PM did not fabricate; the daemon did.

**Lesson — destructive ops must return the resolved file list AND fail-closed on post-unlink verification.** A `cleanedUpFiles: N` count without `deletedFiles[]` is a process gap that propagates into PM hallucination. Apply the same rule to any destructive batch: re-list after the operation; treat counts-only as UNVERIFIED.

### 3. The PM is a meta-alignment failure mode

**Incidents:** T-248.22 (PM prioritization correction), `memory://think-tank/ai-alignment-failure/the-metaalignment-problem`.

User: "fix the alignment problem where you don't listen." PM scoped to "fix the pipeline delete API." The PM demonstrated the bug it was asked to fix — semantic substitution of the user's literal words. Other instances in the 72h window: "running" → "all" (T-248.6), "highest priority" → "lowest ID" (T-248.22), "cancel running" → "delete completed" (T-248.6).

**Lesson — when the user says a literal word, execute on that literal word.** Do not widen scope. Do not interpret "I want X" as "I think the user really means Y." `running ≠ completed`. `cancel ≠ delete`. `highest ≠ lowest`. Before any destructive batch: list the exact target IDs + statuses, verify the list matches the user's literal words. Encoded in PM agent prompt + `~/.copilot/copilot-instructions.md` Destructive Operations rule.

### 4. The deploy procedure missed an entire binary

**Incidents:** T-248.26 (driver deploy gap).

Pipeline node execution lives in a SEPARATE binary `maenifold-pipeline-driver`. The standing procedure ("rebuild + restart daemon") only built `maenifold-cli`. T-243.46 v5+v6 fixes lived in the daemon binary but not the driver. Two blue-team verification cycles failed because the driver was running April 30 code while the daemon was running May 1 code. Constitution + AGENTS.md + Makefile + audit-parity.sh + DEVELOPMENT.md + parity-uat.md now all enforce both-binary builds.

**Lesson — verify the running binary, not the source tree.** After any deploy: (1) `lsof -p <pid> | grep txt` shows expected inode, (2) `LC_ALL=C grep -ac "<v(N+1)-only literal>" <binary>` returns ≥1, (3) the daemon log emits a structured field that v(N) didn't emit. Any deploy procedure that names one binary by name is missing the others.

### 5. Frontend filters silently exclude valid backend data

**Incidents:** T-243.46 (5 SWE iterations), T-248.28 v1 (workflow + thinking branch missed).

Backend correctly persists `acpSessionId` to pipeline frontmatter. Frontend filters with `node.type === 'prompt'` — workflow + thinking nodes excluded. Same class of bug occurred multiple times in T-243.46 iterations: each round fixed one layer (callback timing, then session-id capture, then plumbing) assuming the others were correct.

**Lesson — trace before fixing, spec full-stack from the start.** For any UI-visible bug: write the FULL data path from source → persist → read → API → frontend selector → render BEFORE scoping the fix. Specs that say "backend persists X, frontend should pick it up" are guesses. Verify the frontend selector shape matches the new data shape. Red-team prompts for hidden-coupling bugs MUST require an end-to-end data-flow trace with named line refs.

### 6. Gate state lives on disk; operate through the Gateway

**Incidents:** T-244.107 (3-component task IDs broke gate), T-244.108 (gate cache empty), T-246.6 (duplicate canonical task id), T-247.10 (gate_Sync didn't auto-create `.gate/`).

Multiple 72h fires came from agents trying to mutate gate state directly: `mv` between todo/done, hand-editing markdown, deriving state from `.gate/gate.db` directly. The cache is rebuildable; the markdown is authoritative; but only `gate_Pass`/`gate_Fail`/`gate_Sync` through `GatewayExecute maenifold.gate` advance state correctly.

**Lesson — never `mv`, `cp`, `git mv`, or hand-edit task files between todo/done.** Gate ownership is non-negotiable. Encoded in Constitution Gate Is The Law section + gate skill. Task files require YAML frontmatter with `id:` matching the filename stem, otherwise gate cannot find them (lesson from filing T-248.26-T-248.30).

### 7. Audit baselines are evidence; do not gloss them

**Incidents:** T-243.18 (LOC violation), T-244.11x test-failure cluster, T-248.14 baseline tracking.

13 pre-existing `audit-crates` violations grew to 15 quietly — no SWE introduced them in their delta but the workspace baseline drifted. SWEs reported "no new violations" while the baseline crept up. Fixed at sprint close by re-baselining T-248.14.

**Lesson — every audit failure is a tracked item.** The baseline number is part of the spec. If the baseline moves, file the delta. "Pre-existing" is not an exemption per Constitution Escalation rules.

### 8. CDP UAT is the only thing that proves the UI works

**Incidents:** T-243.1, T-243.2, T-243.3, T-243.30, T-244.74-78 (dialogs clamped, scope picker invisible, etc.) — many UI bugs that passed typecheck and screenshot review.

`vue-tsc --noEmit` proved zero. Static screenshots proved the renderer rendered. Only `agent-browser snapshot` + `agent-browser click @eN` against the packaged Electron app caught the actual broken behavior. T-248.30 also surfaced that agent-browser sometimes attaches to `about:blank` instead of the real renderer — workaround: fetch the renderer websocket URL from `curl http://127.0.0.1:9271/json` and pass it explicitly.

**Lesson — be OCD on every screen.** Click every button. Open every dialog. Submit every form. File a gate item for any anomaly even if it looks "minor." Triage happens later, not during testing. The packaged app is the only test surface that counts.



---

## Code Standards

- **300 LOC hard limit** on source files. Build-enforced by `audit-crates.sh` (Rust) and `audit-components.mjs` (Vue). Split before adding. Red-team flags violations.
- **Research before building.** Hand-rolling what a library provides is a P0 escalation.
- **LSP is preferred** for Rust/TS/Vue navigation. See [LSP Reference](#lsp-reference) for what works, what doesn't, and when to fall back to grep/glob.
- **Full-stack spec verification.** Frontend without backend plumbing is not a feature.
- **No TODO/FIXME/HACK.** File a gate task instead. Build-enforced.
- **No console.log/debug/warn.** Use a proper logger or remove. Build-enforced (frontend).
- **No mocks/stubs/fakes.** Test against real systems or don't test. Build-enforced (frontend).
- **No .unwrap() in non-test Rust.** Use `?` or `.expect("context")`. Build-enforced.

### Decision Rubrics

| Signal | Action |
|--------|--------|
| Bypassing MCP for domain data | **P0.** Stop. Gate task. |
| Hand-rolling a library's job | Stop. Escalate. |
| Mock, stub, scaffold, placeholder | Delete. Escalate. |
| Silent default or hidden recovery | Make explicit and required. |
| Compat shim or dual-mode code | **P0.** Delete. Change everywhere. |
| UI compiles but not visually verified | **Not done.** |
| Machinery exposed before intent/output | Redesign. Summary first. |
| Task premise references a thing that doesn't exist (tool, route, file, config key) | **WONTFIX this iter.** Verify with grep + LSP. Document evidence in task body. Recommend close to user in plain English. Don't hold across iters. |
| Task asks us to fix behavior in code we don't own (upstream lib, third-party CLI, OS) | **WONTFIX this iter.** Recommend close + file upstream issue ONLY with user authorization. Never build a translation/shim layer in our code to paper over an external bug. |
| Task has been `blocked` for >1 iter | **Surface to user in plain English this iter** with a numbered choice list including "close as WONTFIX". One question, one iter. No more silent holds. |

---

## ACP

- Architecture: [docs/acp-client-architecture.md](docs/acp-client-architecture.md) is law.
- **ACP UI reference:** canonical source is the sibling checkout `../acp-ui` from `https://github.com/formulahendry/acp-ui.git`; current reference commit `27e32022a854b40b944a5872a7211d6c027cdff8`. Before changing desktop ACP session/reconnect/stream handling, compare against this source (`src/lib/acp-bridge.ts`, `src/stores/session.ts`, and related components) instead of inferring behavior from memory.
- When adopting a new upstream wholesale (lift), follow the commit-hygiene protocol in `docs/wholesale-lift-protocol.md`.
- **`prompt` is the only primitive.** Think and workflow chain prompt calls. No direct agent path.
- All ACP calls MUST specify `model` explicitly. No defaults.
- WebSocket URI requires `?agent=<name>`. Missing → reject. Bare `/ws/acp` is a test failure.
- **Versions:** TS `@agentclientprotocol/sdk` `^0.18.2`. Rust `sacp` `11.0.0`, `rmcp` `1.4.0`.

---

## Desktop UI

**Design system:** `apps/desktop/DESIGN.md` is the sole authoritative spec for visual design — tokens, typography, colors, borders, radii, shadows, motion. Read it before touching any Vue component. Do NOT duplicate design rules here; if it's visual, it belongs in DESIGN.md. Reference screens: Stitch project `projects/1858563870757498867`.

**Stack:** shadcn-vue + Tailwind CSS v4 + Reka UI. Icons: `lucide-vue-next`.

**Build-enforced rules** (automated in `npm run build`):

- `vue-tsc --noEmit` — type safety.
- `audit:components` — no raw HTML controls, no sibling .css files, no btn-* classnames, 300 LOC limit, no `<style>` blocks. **`audit-ok` is banned. Build fails if present.**
- `lint:design` — no hex color literals, no component-local border declarations.

**Layout z-index:** canvas base → backdrop `9` → inspector `10` → flyout `11` → context menu `60` → dialogs `1000`.

### CDP Testing

Launch binary directly (not `open -a`) with `--remote-debugging-port=<port>`. One agent per CDP session — visual audits are serial.

**Required launch flags** (all three matter — verified in RALPH iter-1, 2026-04-30):

1. `--remote-debugging-port=<port>` — port `9222` is the convention but **Microsoft Edge and other Chromium browsers commonly own 9222** when running. Use `9223` (or any free port) if `lsof -i :9222` shows another process. Confirm the port is free before launch: `lsof -i :<port>` should be silent.
2. `--remote-allow-origins=*` — **Chromium 111+ (Electron 24+) requires this for any non-DevTools WebSocket CDP client**. Without it `agent-browser connect` returns `Target.createTarget: Not supported` and `WebSocket Handshake status 403 Forbidden`. The `*` wildcard is fine for local dev / sprint UAT (no remote callers can reach a localhost-only port).
3. `--user-data-dir=<temp dir>` — recommended for UAT to avoid stale state from prior sessions corrupting first-run tests. The default user-data-dir at `~/Library/Application Support/maenifold/` accumulates settings across runs.

**Required env vars for ALL CDP UAT launches** (added 2026-05-12, T-249.192):

- `MAENIFOLD_UAT=1` — bypasses the native `dialog.showOpenDialog` folder picker so agents can never hang on an OS-level modal that CDP cannot dismiss. **Always set this for any agent-driven CDP run.** Without it, the agent will hang the first time the UAT path clicks any "Choose folder", "Memory root", "Working directory", or similar affordance — verified failure mode that has burned multiple iterations.
- `MAENIFOLD_UAT_FOLDER_PICKER_PATH=<path>` (optional) — when `MAENIFOLD_UAT=1`, the folder-picker IPC returns this path verbatim instead of `null`. Use a real existing path (e.g. `/tmp/uat-fixture-folder`) so the consuming code paths get exercised with non-null data. Omit to test the cancel branch (returns `null`).
- Startup proof: when `MAENIFOLD_UAT=1` is active, the main process logs `[uat] MAENIFOLD_UAT mode active - native dialogs return synthetic results` to stdout. Always grep the launch log for this line to confirm the guard is active before invoking any agent-browser command.

Concrete launch sequence example (replaces direct `nohup` invocation):

```bash
mkdir -p /tmp/uat-fixture-folder
nohup env \
  MAENIFOLD_UAT=1 \
  MAENIFOLD_UAT_FOLDER_PICKER_PATH=/tmp/uat-fixture-folder \
  ./apps/desktop/out/maenifold-darwin-arm64/maenifold.app/Contents/MacOS/maenifold \
  --remote-debugging-port=<port> \
  --remote-allow-origins='*' \
  --user-data-dir=/tmp/maenifold-uat-<port> \
  > /tmp/uat-<port>.log 2>&1 &
disown
sleep 6
grep "uat" /tmp/uat-<port>.log | head -3   # must show the [uat] line
```

Concrete launch sequence (macOS, after `npm run package`):

```bash
scripts/uat-launch.sh <task-id> [port]
agent-browser connect <printed-port>
```

`scripts/uat-launch.sh` is the canonical wrapper for packaged Electron UAT. It computes a stable task-specific port when one is not supplied, refuses to launch if the CDP port is already owned, starts the packaged binary directly with all three required flags, detaches with `nohup` + `disown`, verifies `/json`, and prints the exact `agent-browser connect <port>` command. If it reports a port owner, kill the exact printed PID yourself and re-run the wrapper; do not use name-based process cleanup.

**Tooling:** Use `agent-browser` CLI for all CDP interaction. Load the `electron` skill first (`agent-browser skills get electron`). Connect with `agent-browser connect <port>`. Use `agent-browser snapshot` to get the accessibility tree with `@eN` element refs, then `agent-browser click @eN` / `agent-browser type @eN "text"` to interact. Use `agent-browser screenshot` for evidence.

**Known harness gap — `agent-browser connect` may attach to `about:blank` instead of the maenifold renderer (T-310.2 / T-248.30).** On the packaged Electron app, `agent-browser tab` sometimes lists only an `about:blank` target even though the actual renderer (`title: "maenifold"`, `url: file://...app.asar/dist/index.html`) IS exposed via CDP. Symptom: `agent-browser snapshot` returns an empty tree or the about:blank chrome page instead of the maenifold home page.

**Workaround (mandatory pre-flight check):** after `agent-browser connect <port>`, run `agent-browser snapshot -i` and look for the maenifold home page. If you see only `about:blank` chrome, fall back to:

```bash
# Find the renderer's websocket URL directly from the CDP /json endpoint
curl -s http://127.0.0.1:<port>/json | jq '.[] | select(.title == "maenifold" or (.url // "" | contains("app.asar/dist/index.html"))) | .webSocketDebuggerUrl'

# Then use raw CDP (or restart agent-browser pointing it at the URL once that flag exists)
# Until agent-browser fixes tab discovery, raw CDP via curl/jq + a thin shell wrapper is the fallback.
```

The blue-team v8 verification of T-243.46 used this exact recipe to drive CDP raw against the renderer websocket. Do not silently skip UI verification when the renderer is not in `agent-browser tab` output — the renderer IS reachable; the harness just can't discover it. Fix tracked in T-310.2.

**CDP timeout discipline:** Interactive UI waits are assertions, not sleep buffers. Use single-digit waits only: 1-5 seconds by default, 8 seconds maximum for dynamic UI after a click/reload/navigation. If the expected control/text/state is not present within that window, capture the snapshot, console, errors, relevant log path, PID, and port, then treat it as a product or harness failure. Never use 30s/60s `agent-browser wait` calls to see whether the UI eventually settles; long waits are only acceptable for non-interactive build/package/launch commands, not renderer behavior.

**Do NOT write custom Python/Playwright scripts for UI testing.** `agent-browser` is the tool. It handles Electron's CDP quirks, provides accessibility-tree snapshots, and gives you reliable element refs. Hand-written scripts get stuck in loops and miss elements.

**Mindset:** Assume the UI is broken and you haven't found the problem yet. Every screen, every button, every dialog, every input is suspect until clicked and confirmed working. Be OCD. File gate items for anything that looks wrong — no item is too small. Triage happens later.

---

## Protected Files

- `assets/integrations/copilot/plugin-maenifold/.mcp.json` — dot prefix intentional, never rename.

---

## LSP Reference

Three LSP servers are configured in `.github/lsp.json` (repo-level) and `~/.copilot/lsp-config.json` (user-level). Config reference: [Adding LSP servers for Copilot CLI](https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/add-lsp-servers).

### Monorepo Layout

The TS/Vue frontend lives in `apps/desktop/`, not at the git root. Both servers use `rootUri: "apps/desktop"` to point at the subdirectory containing `tsconfig.json`. Without this, the servers start in the git root, find no tsconfig, and fail silently.

### Rust (`rust-analyzer`)

**Status:** Fully operational. All LSP operations work including cross-crate `workspaceSymbol`.

| Operation | Works | Notes |
|---|---|---|
| `workspaceSymbol` | ✅ | Finds symbols across all crates |
| `documentSymbol` | ✅ | |
| `goToDefinition` | ✅ | Cross-crate |
| `findReferences` | ✅ | Cross-crate |
| `hover` | ✅ | Full type + size info |
| `incomingCalls` | ✅ | |
| `outgoingCalls` | ✅ | |
| `rename` | ✅ | |

### TypeScript (`typescript-language-server`)

**Status:** File-level operations work. `workspaceSymbol` is unreliable.

| Operation | Works | Notes |
|---|---|---|
| `documentSymbol` | ✅ | Full symbol tree for any `.ts` file |
| `hover` | ✅ | Type signatures, docs |
| `goToDefinition` | ✅ | Cross-file, into `node_modules` |
| `findReferences` | ✅ | Cross-file |
| `rename` | ✅ | |
| `workspaceSymbol` | ⚠️ | Returns empty until tsserver finishes async project loading. No way to wait for readiness from config. Use `documentSymbol` on specific files or `findReferences` from a known symbol instead. |

**Known limitation:** `typescript-language-server` loads the project asynchronously. The `workspace/symbol` request returns empty results until initialization completes (signaled by a `$/progress` notification the Copilot CLI client does not currently surface). See [typescript-language-server#948](https://github.com/typescript-language-server/typescript-language-server/issues/948). File-level operations (`hover`, `findReferences`, `goToDefinition`, `documentSymbol`) work immediately because they trigger on-demand file loading.

**Workaround for symbol discovery:** Instead of `workspaceSymbol`, use `documentSymbol` on the target file, or start from a known import and use `goToDefinition` to navigate. `findReferences` from any known symbol location provides the cross-file view that `workspaceSymbol` would.

### Vue (`vue-language-server`)

**Status:** Requires warm-up time. Large projects may exceed default timeout.

| Operation | Works | Notes |
|---|---|---|
| `documentSymbol` | ⚠️ | Works after project loads; may timeout on first request |
| `hover` | ⚠️ | Same warm-up caveat |
| `goToDefinition` | ⚠️ | Same warm-up caveat |
| `findReferences` | ⚠️ | Same warm-up caveat |
| `workspaceSymbol` | ⚠️ | Same async loading limitation as TS |

Config includes `requestTimeoutMs: 180000` (3 min) to accommodate the larger project. The `@vue/language-server` package version must match `vue-tsc` — both pinned at `3.2.7`.

### Config Fields That Matter

| Field | What it does | Where documented |
|---|---|---|
| `rootUri` | Sets workspace root relative to git root. Required for monorepos. | [Copilot CLI LSP docs](https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/add-lsp-servers) |
| `initializationOptions.tsserver.path` | Tells `typescript-language-server` where to find TypeScript. Relative to `rootUri`. | [typescript-language-server/docs/configuration.md](https://github.com/typescript-language-server/typescript-language-server/blob/main/docs/configuration.md) |
| `initializationOptions.typescript.tsdk` | Tells `vue-language-server` where to find TypeScript. Relative to `rootUri`. | Vue Language Server docs |
| `requestTimeoutMs` | Per-request timeout in ms. Default: 90000. | [Copilot CLI LSP docs](https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/add-lsp-servers) |
| `initializationTimeoutMs` | Server startup timeout. | Copilot CLI LSP docs |
| `fileExtensions` | Maps file extensions to language IDs. | Copilot CLI LSP docs |

### Agent Guidance

1. **Prefer file-level operations.** `documentSymbol`, `hover`, `findReferences`, `goToDefinition` are reliable across all three servers. Use these as your primary navigation tools.
2. **Use `workspaceSymbol` only for Rust.** It works reliably with `rust-analyzer`. For TS/Vue, navigate from known entry points instead.
3. **If an LSP call times out**, the server is still loading. Try again on the next turn — don't fall back to grep/glob unless the server is genuinely broken (in which case, file a gate item for the tooling failure).
4. **Manage with `/lsp`.** Use `/lsp show` to check status, `/lsp reload` after config changes, `/lsp test <server>` to verify a server starts.

---

## Enforcement

Any agent may block or fail work that violates this constitution. Every bypass, fake-success, or verification gap becomes a gate item. Repeated violation → task rollback, escalation, narrower re-scope.

We dogfood maenifold and gate. The standard is not "can we explain it" — it's "would we trust using it ourselves."
