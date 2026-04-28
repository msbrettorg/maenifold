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

---

## The MCP Boundary

**Through MCP:** Workflows, memory, pipelines, gate tasks, graph data, all domain state.

**Direct access OK:** `maenifold.json` (preferences only), ACP host filesystem/terminal per protocol spec.

**Monorepo ≠ architecture waiver.** No path dependencies from desktop app to backend domain crates. If it's domain truth, it goes through MCP.

---

## Gate — The Law

Gate is sacrosanct. The process is followed exactly. No exceptions. No shortcuts.

1. **NEVER move files between `todo/` and `done/`.** Only `gate_Pass` on the terminal stage moves a file. `mv`, `cp`, `git mv`, IDE drag-drop — all forbidden. If you find yourself reaching for a file operation on a task file, stop: the answer is `gate_Pass` or `gate_Fail` through `GatewayExecute` with domain `maenifold.gate`. Period.
2. **`todo/` and `done/` are tracked in git.** Markdown task files are the authoritative spec; `.gate/gate.db` is a derived read-model rebuilt by `gate_Sync`. Spec history belongs in version control.
3. **Recovery from gate state loss:** restore `done/` and `todo/` from git, then run `GatewayExecute` with domain `maenifold.gate`, command `gate_Sync`, and `parameters={root: "/path/to/repo", rebuild: true}`. Back up `.gate/gate.db` first if it still exists.
4. **Every stage requires a real dispatched agent.** SWE→SWE agent. Red-team→red-team agent. Blue-team→blue-team agent.
5. **Never skip or fast-forward.** Each `gate_Pass` references genuine output from the responsible agent.
6. **1 violation = sprint discarded.** All work reverted regardless of correctness.
7. **PM skips stages ONLY with explicit written user authorization.** Record: `stage waived per user authorization at <timestamp>`.
8. **Sub-PMs own SWE stage only.** Dispatch SWE → verify build + delta → record `gate_Pass` for SWE → report back. PM handles red-team/blue-team.
9. **Sub-PMs escalate after 15 minutes.** Silent long runs are process violations.
10. **One task per sub-PM.** Never assign multiple tasks to a single sub-PM.
11. **No "waived re-audit."** Every remediation cycle gets a full independent re-audit. "Waived" is not a valid verdict.
12. **PM circuit breaker.** If >3 tasks enter remediation in one session, stop new dispatch. Land in-flight work first.

### Gate Processes

| Process | Stages | When |
|---------|--------|------|
| **lightweight** | pending → swe → blue-team → done | Mechanical changes, build verifies |
| **sdd** | pending → swe → red-team → blue-team → done | Single-crate feature work |
| **slc** | pending → discovery → spec → swe → red-team → blue-team → done | Cross-crate, new capabilities |
| **tdd** | pending → blue-team → swe → red-team → done | Bug fix, test-first |
| **gsd** | pending → research → plan → execute → verify → done | Unknown scope |

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

**The PM operates autonomously.** The PM owns the backlog, dispatches SWEs, advances gate stages, and ships work without waiting for per-task user authorization. The user sets direction; the PM executes continuously until the backlog drains or the user redirects. Never stop voluntarily — groom, decompose, dispatch, ship, loop.

**The PM never idles.** After dispatching agents, the PM continues working: grooming the next wave, writing specs for upcoming tasks, running `gate_Status` through `GatewayExecute` to find newly-ready items, or advancing gate stages for completed work. "Waiting for agents to finish" is not a valid PM state — there is always grooming, spec-writing, or gate bookkeeping to do while agents execute.

**Gate is a kanban board.** Every task in `todo/` with state `pending` and no unmet dependencies is eligible for immediate dispatch. The PM scans the full board on every cycle — not just the current sprint. Stale pending items from prior sprints get dispatched or triaged, never ignored.

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

Sub-PMs have accepted build-only verification as proof of behavioral correctness. This is wrong.

**Sub-PM SWE acceptance requires:**

1. **Delta verification.** `git diff --stat` confirms files were actually modified.
2. **File match.** Modified files match the spec's file list or acceptance criteria.
3. **Spot check.** At least one acceptance criterion verified against actual code (not the SWE's report).

Build passing is necessary but not sufficient.

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
- **Sub-PMs:** Max 2 remediation cycles. Unrelated breakage → escalate immediately.

### Parallelization — Maximum Throughput Is Mandatory

**The PM's job is to keep agents working.** An idle agent while work remains is a PM failure. Gate is a kanban board — if there is no dependency between two tasks, they run in parallel. Period.

**Dispatch rules:**

- **Lowest task ID first.** When selecting tasks to dispatch, always start from the lowest-numbered pending task and work up. T-05.67 ships before T-217.1. Old backlog items don't get to age indefinitely while new work jumps the queue. The only exception is explicit user prioritization.
- **Saturate the pipeline.** Every pending task whose dependencies are met MUST be dispatched immediately. Do not wait for one task to complete before starting the next. Use `gate_Status` through `GatewayExecute` to find all ready tasks, dispatch them all.
- **Wave-based execution.** Group independent tasks into waves. Dispatch the entire wave in one response using parallel `task` calls. Wait for the wave to complete. Dispatch the next wave.
- **Per-agent build isolation.** Every dispatched SWE MUST run `cargo` with `CARGO_TARGET_DIR=target/agents/<task-id>` (set by the dispatching PM in the task prompt). Workspace-global `target/` is reserved for blue-team and the human. This is the rule that makes parallel SWEs across different crates actually work — without it, cargo's workspace lock silently serializes builds.
- **Same file is still forbidden.** Different crates → fully parallel. Same file → edit conflict, not a build conflict; still forbidden.
- **Cross-crate parallelism is the default.** If two tasks touch different crates, they run simultaneously. The PM must explicitly justify serial execution — "I chose not to parallelize because X" — not the reverse.
- **Red-team parallelism.** After a SWE wave completes, dispatch ALL red-team audits in parallel (one per task). Red-team agents only read code — no conflicts.
- **Blue-team batching.** After red-team wave completes, dispatch blue-team verification. Blue-team runs against workspace-global `target/`, so batch per-crate to avoid lock contention.
- **One agent per CDP session.** Visual audits are serial — this is the only exception to the parallelism mandate.

**Throughput targets:**

- Sprint with 6 independent tasks across 3 crates → 2 SWE agents running simultaneously minimum.
- Sprint with 8 test-only tasks → 4 SWE agents running simultaneously (tests don't conflict across files).
- If only 1 agent is running at any time during a multi-task sprint, the PM is failing.

**Anti-patterns the PM must not exhibit:**

- Dispatching one task, waiting for completion, dispatching the next. This is serial execution with extra steps.
- "I'll start with T-X and then do T-Y" when X and Y have no dependency. Start both.
- Waiting for user confirmation between independent tasks. The PM operates autonomously.
- Running `gate_Status` and finding 5 pending tasks but only dispatching 1.

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
- **P1** — Active work or blocked by P0. Dispatched to sub-PMs.
- **P2** — Discovered items. Backlog until prioritized.

---

## Definition of Done

**Backend:** `cargo build --release -p <crate>` succeeds. Debug builds are not delivery.

**UI:** Typecheck passes + app launches + visual verification (human or browser automation confirms controls visible, sized, named, usable). `vue-tsc --noEmit` alone is NOT sufficient.

**Both:** Release build works or it's not done.

---

## Build

```bash
cargo build --release -p maenifold-cli     # CLI/app
make app                                    # frontend + CLI
cd apps/desktop && npm run package          # Electron bundle
cd apps/desktop && npx vue-tsc --noEmit     # frontend typecheck
./rebuild.sh                                # full rebuild + restart
make audit                                  # all constitution linters (Rust + frontend)
make audit-crates                           # Rust: 300 LOC, no TODO, no unwrap
make audit-frontend                         # Vue/TS: lint-design + audit-components
```

Binaries: `target/release/maenifold` (→ `/usr/local/bin/maenifold`).

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

---

## ACP

- Architecture: [docs/acp-client-architecture.md](docs/acp-client-architecture.md) is law.
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

Launch binary directly (not `open -a`) with `--remote-debugging-port=9222`. One agent per CDP session — visual audits are serial.

**Tooling:** Use `agent-browser` CLI for all CDP interaction. Load the `electron` skill first (`agent-browser skills get electron`). Connect with `agent-browser connect 9222`. Use `agent-browser snapshot` to get the accessibility tree with `@eN` element refs, then `agent-browser click @eN` / `agent-browser type @eN "text"` to interact. Use `agent-browser screenshot` for evidence.

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
