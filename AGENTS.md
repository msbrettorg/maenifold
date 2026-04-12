# maenifold — Constitution

maenifold is the reasoning console. One product, one monorepo.
`crates/` is the Rust backend. `apps/desktop/` is the Tauri + Vue frontend.
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

**Monorepo is not an architecture waiver.** The Tauri app must not depend on maenifold domain crates or private backend APIs. A path dependency from the app to a backend domain crate is a constitutional violation unless the crate is protocol-only and PM-approved in writing. Calling something "settings," "cache," or "local state" does not exempt it if it contains replicated domain truth.

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

## Definition of Done

**Backend:** `cargo build --release -p <crate>` succeeds. Debug builds are not delivery.

**UI:** Typecheck passes + app launches + visual verification. Visual verification means a human or browser automation observed the real UI outcome at intended viewport(s), confirmed affected controls are visible, correctly sized, named, and usable. A screenshot alone is not verification — the reviewer must state what was inspected, what was expected, and what was actually seen. A first-time user must be able to get value from the feature without learning internal workflow vocabulary.

**Both:** Release build. If the release build does not work, the feature is not done.

---

## Guardrails (Hard-Learned)

### Build Commands

- **Desktop app bundle:** `cd apps/desktop && npx tauri build` builds the desktop app. Treat the `.app` bundle as the deliverable; DMG bundling may fail.
- **Desktop frontend typecheck:** `cd apps/desktop && npx vue-tsc --noEmit`
- **Tauri Rust code:** `cargo build --release --manifest-path apps/desktop/src-tauri/Cargo.toml` — `apps/desktop/src-tauri` is excluded from the workspace root, so do **not** use `cargo build -p` for the Tauri app.

### Test Execution Policy

- **SWE agents:** `cargo build -p <crate>` only. Never `cargo test`.
- **Red-team agents:** Static analysis only — read code and reason. Never execute tests.
- **Blue-team agents:** Named tests only — `cargo test -p <crate> <test_name>`. Never full workspace.
- **PM:** One `cargo test -p <crate>` at sprint close. Never mid-sprint.

Agents running full test suites in parallel creates race conditions. Violation is grounds for immediate agent termination.

### ACP Invariants

- **Version baselines:** TypeScript uses `@agentclientprotocol/sdk` `^0.18.2`. Rust uses `sacp` `11.0.0` (git rev pinned in `[patch.crates-io]`) and `rmcp` `1.4.0`.
- **`prompt` is the only primitive.** Think and workflow are orchestration loops that chain prompt calls. Neither has a direct path to the ACP agent. See [docs/acp-client-architecture.md](docs/acp-client-architecture.md).
- All ACP agent interactions MUST specify `model` explicitly. No defaults. Different agents default to different models with wildly different latency and cost.
- WebSocket URI must include `?agent=<name>`. Missing `?agent=` must be rejected with an error. Bare `/ws/acp` is a test failure.

### Desktop UI Conventions

- **Icon library:** `lucide-vue-next` is the established structural UI icon set. Current pipeline rail/flyout usage includes `PanelLeftOpen`, `PanelLeftClose`, `Plus`, and `RefreshCw`.
- **Pipeline layout contract:** canvas is the base layer; backdrop `9`; inspector `10`; flyout `11`; context menu `60`; dialogs `1000`.
- **Pipeline layout tokens:** `--rail-width: 48px`; `--flyout-width: 320px`.

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
