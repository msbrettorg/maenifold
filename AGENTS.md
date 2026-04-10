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
- Code compiles but no visual verification (UI work)? → **Not done.**

---

## Definition of Done

**Backend:** `cargo build --release -p <crate>` succeeds. Debug builds are not delivery.

**UI:** Typecheck passes + app launches + visual verification. Visual verification means a human or browser automation observed the real UI outcome at intended viewport(s), confirmed affected controls are visible, correctly sized, named, and usable. A screenshot alone is not verification — the reviewer must state what was inspected, what was expected, and what was actually seen.

**Both:** Release build. If the release build does not work, the feature is not done.

---

## Guardrails (Hard-Learned)

### Test Execution Policy

- **SWE agents:** `cargo build -p <crate>` only. Never `cargo test`.
- **Red-team agents:** Static analysis only — read code and reason. Never execute tests.
- **Blue-team agents:** Named tests only — `cargo test -p <crate> <test_name>`. Never full workspace.
- **PM:** One `cargo test -p <crate>` at sprint close. Never mid-sprint.

Agents running full test suites in parallel creates race conditions. Violation is grounds for immediate agent termination.

### ACP Invariants

- **`prompt` is the only primitive.** Think and workflow are orchestration loops that chain prompt calls. Neither has a direct path to the ACP agent. See [docs/acp-client-architecture.md](docs/acp-client-architecture.md).
- All ACP agent interactions MUST specify `model` explicitly. No defaults. Different agents default to different models with wildly different latency and cost.
- WebSocket URI must include `?agent=<name>`. Missing `?agent=` must be rejected with an error. Bare `/ws/acp` is a test failure.

### Protected Files

- `assets/integrations/copilot/plugin-maenifold/.mcp.json` — Copilot plugin manifest. The dot prefix is intentional. Never rename.

---

## Dogfooding

We build maenifold and gate while using maenifold and gate. Every friction point is product feedback. Issues become gate tasks. Red-team findings are always filed, regardless of severity. The standard is not "can we explain it" — the standard is "would we trust using it ourselves."

---

## Enforcement

Any agent may block or fail work that violates this constitution. Every boundary bypass, fake-success path, or verification gap becomes a gate item with reproduction steps. Repeated violation requires task rollback, escalation, and narrower re-scope before work resumes.
