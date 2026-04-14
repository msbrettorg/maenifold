# maenifold — Constitution

maenifold is the reasoning console. One product, one monorepo.
`crates/` is the Rust backend. `apps/desktop/` is the Electron + Vue frontend.
Colocation is for coordination, not for collapsing boundaries.

---

## Core Tenets

### AI is the New UI

MCP is the universal interface to maenifold. The desktop app and AI agents talk to maenifold through the same MCP protocol. There are no backdoors for the UI, scripts, or developers. If an AI agent cannot do it through MCP, a human should not be able to either.

### We Do Not Write Code

Assume the solution already exists. Research before implementing. Prefer established libraries, reference implementations, and prior art over invention. If you cannot find a credible solution, stop and escalate. New code requires explicit PM authorization for a specific, bounded delta with a cited reference implementation and a written reason adaptation was insufficient.

### Mechanism, Not Policy

No silent defaults. Every policy choice must be explicit and required. Errors are information — return clear failures with available options. Hidden recovery is concealed policy and is therefore a bug.

### Simple, Lovable, Complete

No stubs, no mocks, no scaffolds, no placeholders, no fake surfaces. A feature is either complete enough to trust or it is not ready.

### No Backwards Compatibility

User base of one. No shims, no compat layers, no legacy code paths, no version negotiation. When the contract changes, everything changes — code, tests, fixtures, data. Old behavior is not preserved; it is replaced. Backwards-compat code is technical debt with zero users to justify it. If a new feature requires touching old files, touch them — don't write adapter code to avoid it. Any backwards-compat shim, migration path, or dual-mode code is a P0 engineering failure.

### Help Before Workflow

Never make the user understand the workflow before the workflow helps the user. Default surfaces lead with intent, examples, and outputs — not internal orchestration primitives. The engine can be arbitrarily rigorous; the surface must not require users to internalize that rigor before they get value. Show what happened, what came out, and what needs review before exposing session IDs, prompts, tool arguments, or thinking traces. Progressive disclosure separates the operator console (meaning first) from expert mode (machinery on demand). If a feature exposes orchestration before intent or outputs, it is not ready.

---

## The MCP Boundary

This is the primary architectural rule.

**Must go through MCP:** Workflows, memory, pipelines, gate tasks, graph data, and all derived domain state.
The UI must not read or write those domains by opening files, querying SQLite, importing backend crates, or hydrating from cached exports.

**Direct access permitted:**
- Local app settings (`maenifold.json`) — preferences and connection info only, not replicated domain truth.
- ACP host filesystem/terminal capabilities inside agent sessions — required by the ACP protocol spec.

**Monorepo is not an architecture waiver.** The Electron app must not depend on maenifold domain crates or private backend APIs. A path dependency from the app to a backend domain crate is a constitutional violation unless the crate is protocol-only and PM-approved in writing. Calling something "settings," "cache," or "local state" does not exempt it if it contains replicated domain truth.

---

## Decision Rubrics

- Bypassing MCP for domain data? → **P0.** Stop. File a gate task.
- Hand-rolling something that exists as a library? → Stop. Escalate.
- Mock, stub, scaffold, or placeholder? → Delete it. Escalate.
- Silent default or hidden recovery? → Make the choice explicit and required.
- Backwards-compat shim, migration path, or dual-mode code? → **P0.** Delete it. Change the contract everywhere.
- Code compiles but no visual verification (UI work)? → **Not done.**
- Exposing raw machinery on the default path? → **Redesign.** Summary first, detail on demand.
- Blank-canvas first-run with no examples or templates? → **Not done.**
- Simplified surface hides routing or persistence choices? → Make the choice explicit.

---

## File Size Limit — 300 LOC

**No source file may exceed 300 lines of code.** This is a hard limit, not a guideline. Files over 300 LOC are how monoliths form — agents pile code into existing files instead of splitting, duplication hides in plain sight, and crate boundaries dissolve.

When a file approaches 300 LOC, split it before adding more. When an agent touches a file already over 300 LOC, it must split it as part of the change. Use `cargo-modules structure` and `cargo-modules dependencies` to find the clean seams.

Red-team audits MUST flag any file over 300 LOC as a finding. Exceptions require explicit PM authorization with a written reason.

---

## Definition of Done

**Backend:** `cargo build --release -p <crate>` succeeds. Debug builds are not delivery.

**UI:** Typecheck passes + app launches + visual verification. Visual verification means a human or browser automation observed the real UI outcome at intended viewport(s), confirmed affected controls are visible, correctly sized, named, and usable. A screenshot alone is not verification — the reviewer must state what was inspected, what was expected, and what was actually seen. A first-time user must be able to get value from the feature without learning internal workflow vocabulary.

**Both:** Release build. If the release build does not work, the feature is not done.

---

## Guardrails (Hard-Learned)

### Build Commands

- **Desktop app bundle:** `cd apps/desktop && npm run package` builds the Electron app via Forge. Output: `out/maenifold-darwin-arm64/maenifold.app`.
- **Desktop frontend typecheck:** `cd apps/desktop && npx vue-tsc --noEmit`
- **Full rebuild + restart:** `./rebuild.sh` builds all 5 Rust crates release, restarts launchd services, rebuilds the desktop app, and verifies MCP ports.

### Desktop App Testing (CDP / agent-browser)

The desktop app is Electron. To test UI with `agent-browser`, you must launch the binary directly with `--remote-debugging-port`. The macOS `open -a` command does **not** reliably pass `--args` to Electron.

```bash
# Kill existing instance first
pkill -f 'maenifold.app/Contents/MacOS/maenifold' || true
sleep 1

# Launch binary directly — NOT via `open`
apps/desktop/out/maenifold-darwin-arm64/maenifold.app/Contents/MacOS/maenifold \
  --remote-debugging-port=9222 2>/tmp/maenifold-cdp.log &
disown
sleep 4

# Verify CDP is listening
curl -s http://localhost:9222/json/version

# Connect agent-browser
agent-browser connect 9222
agent-browser snapshot -i
agent-browser screenshot /tmp/test.png
```

**Key gotchas:**
- `open -a "maenifold" --args --remote-debugging-port=9222` does **not** work — the flag is silently dropped by macOS for .app bundles.
- CDP requires the `--remote-debugging-port` flag at process launch time. There is no way to enable it after the fact.
- Vue Flow edges are SVG `<path>` elements. `agent-browser dblclick` with a ref won't hit them. Use `agent-browser eval` with JS to dispatch `dblclick` on edge elements:
  ```bash
  agent-browser eval '
    const edge = document.querySelector(".vue-flow__edge path.vue-flow__edge-path");
    if (edge) {
      const r = edge.getBoundingClientRect();
      edge.dispatchEvent(new MouseEvent("dblclick", {
        bubbles: true, clientX: r.x + r.width/2, clientY: r.y + r.height/2, view: window
      }));
      "ok";
    } else { "no edge"; }
  '
  ```
- Node double-click works normally via snapshot refs: find the node button ref, then `agent-browser dblclick @eN`.

### Test Execution Policy

- **SWE agents:** `cargo build -p <crate>` only. Never `cargo test`.
- **Red-team agents:** Static analysis only — read code and reason. Never execute tests.
- **Blue-team agents:** Named tests only — `cargo test -p <crate> <test_name>`. Never full workspace.
- **PM:** One `cargo test -p <crate>` at sprint close. Never mid-sprint.

Agents running full test suites in parallel creates race conditions. Violation is grounds for immediate agent termination.

### Scope Discipline

**The task is the task.** SWE agents do exactly what the spec says — nothing more. If an agent encounters an adjacent problem (oversized file, nearby dead code, naming inconsistency), it reports it in the output and moves on. It does NOT fix it. Scope creep from agents is how we get 10-minute tasks that take 47 tool calls, out-of-scope file splits, and wasted work that has to be reverted. The PM decides what to fix next, not the agent.

### Formatting Policy

SWE agents must not run workspace-wide formatters (`cargo fmt`, `rustfmt`, `prettier` without path args). If formatting is needed, format only the files you created or modified. Touching unrelated files — even whitespace-only changes — is a scope violation that gets reverted.

### Severity Policy

- **P0** — Compile errors. Anything that breaks `cargo build --release` or `cargo test --no-run`. Always top of queue. **PM handles personally.**
- **P1** — Work in progress. Tasks currently being executed or blocked by a P0. **Dispatched to sub-PMs.**
- **P2** — Discovered items. Tech debt, duplication, refactoring opportunities found during other work. Backlog until prioritized.

### Gate Process Selection

Available processes are defined in `/Users/brett/maenifold/assets/gates/`. Use the `pipeline:` frontmatter field to assign.

| Process | Stages | Use When |
|---------|--------|----------|
| **lightweight** | pending → swe → blue-team → done | Mechanical refactoring, dedup, fixture fixes, policy updates. Build is the verification. |
| **sdd** | pending → swe → red-team → blue-team → done | Standard feature work within a single crate. Needs adversarial review but not discovery/spec. |
| **slc** | pending → discovery → spec → swe → red-team → blue-team → done | Cross-crate features, new capabilities, architectural changes. Full lifecycle with spec authorship. |
| **tdd** | pending → blue-team → swe → red-team → done | Bug fixes where writing the test first prevents regression. |
| **gsd** | pending → research → plan → execute → verify → done | Exploratory work with unknown scope. Research-first. |

**Decision rules:**
- No behavior change, no new code, build verifies it → **lightweight**
- New code in one crate, behavior changes → **sdd**
- New crate, cross-crate wiring, plugin epic, UI features → **slc**
- Bug fix with known reproduction → **tdd**
- Unknown scope, needs research → **gsd**

### ACP Invariants

- **Version baselines:** TypeScript uses `@agentclientprotocol/sdk` `^0.18.2`. Rust uses `sacp` `11.0.0` (git rev pinned in `[patch.crates-io]`) and `rmcp` `1.4.0`.
- **`prompt` is the only primitive.** Think and workflow are orchestration loops that chain prompt calls. Neither has a direct path to the ACP agent. See [docs/acp-client-architecture.md](docs/acp-client-architecture.md).
- All ACP agent interactions MUST specify `model` explicitly. No defaults. Different agents default to different models with wildly different latency and cost.
- WebSocket URI must include `?agent=<name>`. Missing `?agent=` must be rejected with an error. Bare `/ws/acp` is a test failure.

### Desktop UI Conventions

- **Icon library:** `lucide-vue-next` is the established structural UI icon set. Current pipeline rail/flyout usage includes `PanelLeftOpen`, `PanelLeftClose`, `Plus`, and `RefreshCw`.
- **Pipeline layout contract:** canvas is the base layer; backdrop `9`; inspector `10`; flyout `11`; context menu `60`; dialogs `1000`.
- **Pipeline layout tokens:** `--rail-width: 48px`; `--flyout-width: 320px`.

### Gate Integrity — Hard Rule

**No fast-forwarding gate stages.** Every gate stage (discovery, spec, swe, red-team, blue-team) requires a real agent dispatched to do real work. The PM may only `gate pass` a stage after receiving a genuine verdict from the responsible agent. Walking multiple stages in rapid succession with PM-authored notes is audit fraud — it corrupts the trail and makes the codebase untrustworthy.

**If gate state is out of sync** (e.g., prior session did real work but gate wasn't walked), the PM must dispatch verification agents to rebuild the audit trail honestly. Never backfill stages with fabricated notes.

**PM override:** The PM may only skip a gate stage with explicit, written authorization from the user (Brett). "Simple additive field" or "waived" are not valid reasons to skip red-team or blue-team. If the user grants a waiver, record it as: `stage waived per user authorization at <timestamp>`.

### Gate CLI Fallback

- Prefer Gate MCP when healthy, but Gate MCP sessions can go stale. Fall back to CLI with `--root .`.
- `gate sync --root . --rebuild`
- `gate status --root .`
- `gate pass --root . <task-id> --agent <name> --notes "..."`

### Protected Files

- `assets/integrations/copilot/plugin-maenifold/.mcp.json` — Copilot plugin manifest. The dot prefix is intentional. Never rename.

---

## Dogfooding

We build maenifold and gate while using maenifold and gate. Every friction point is product feedback. Issues become gate tasks. Red-team findings are always filed, regardless of severity. The standard is not "can we explain it" — the standard is "would we trust using it ourselves."

---

## Enforcement

Any agent may block or fail work that violates this constitution. Every boundary bypass, fake-success path, or verification gap becomes a gate item with reproduction steps. Repeated violation requires task rollback, escalation, and narrower re-scope before work resumes.
