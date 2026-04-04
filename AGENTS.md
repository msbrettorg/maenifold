# Agent Team — maenifold-rs

## We Do Not Write Code

We do not write code. This is not a guideline. This is not aspirational. This is a hard requirement with zero exceptions.

Every problem has already been solved by someone else. Find that solution. Use it. If you cannot find it, you are looking wrong. Reframe the problem and search again.

We wasted 4 days hand-rolling an ACP client when Goose Desktop already had a working implementation. That will never happen again.

**The rule:**
1. **Find it.** Search GitHub, crates.io, npm, docs, papers. Someone has solved this.
2. **Use it.** Depend on the library. Adapt the example. Reference the implementation.
3. **If you can't find it, you're looking wrong.** Reframe the problem. Search again. The solution exists.
4. **If you still can't find it, escalate.** The PM decides whether the task is valid or the approach needs to change.
5. **Nobody writes code.** Not SWEs, not architects, not PMs, not agents. Nobody. Ever. Without explicit PM authorization for a specific, bounded change backed by a reference implementation.

Violating this policy is a termination event. The PM will kill your agent, revert your changes, and escalate.

This applies to agents too. SWEs research before implementing. Architects research before designing. PMs research before specifying. Nobody gets to skip the "find it first" step.

---

We're building and dogfooding `maenifold` and `gate` at the same time. 
Any issues encountered with usability, semantics, errors, documentation, workflow, naming conventions, etc. should be escalated to the product-manager and added to the backlog using `gate` as a new T- item.
Red-team audit items must ALWAYS be added to `gate` regardless of severity.
Always check for newly added or pre-existing items before adding new.
Default to the TDD process when using `gate`.

## Test Execution Policy

**SWE agents:** `cargo build -p <crate>` only. Never `cargo test`.
**Red-team agents:** Static analysis only — read code and reason. Never execute tests.
**Blue-team agents:** Named tests only — `cargo test -p <crate> <test_name>`. Never full workspace.
**PM:** One `cargo test -p <crate>` at sprint close to confirm green. Never mid-sprint.

Agents running full test suites in parallel is a waste and creates race conditions. Violating this policy is grounds for the PM to kill the agent immediately.

We use Azure AI foundry as the backend for Codex and OpenCode in this environment. 
Copilot CLI uses GitHub Enterprise for the backend.
Codex, OpenCode and Copilot CLI have all been verified as working and have parity in features for our purposes.
Auth issues with ACP Agents = configuration problems with the code.

## Protected Files

**DO NOT rename, move, or delete these files:**
- `assets/integrations/copilot/plugin-maenifold/.mcp.json` — Copilot plugin manifest. The dot prefix is intentional. Never rename to `_.mcp.json`.

## Build

```bash
cargo build --release -p maenifold-cli     # CLI/app changes
cargo build --release -p maenifold-gate    # gate changes
make app                                    # frontend + CLI (touch lib.rs forces re-embed)
```

Binary: `target/release/maenifold` → symlinked at `/usr/local/bin/maenifold`.
Binary: `target/release/gate` for gate runtime changes.

**Words are not delivery. A rebuilt, runnable binary is delivery.**

## Runtime Services

Both `maenifold` and `gate` run as launchd user agents on this system in HTTP mode:

| Service | Label | Port | Binary |
|---------|-------|------|--------|
| maenifold | `com.ma-collective.maenifold` | 8080 | `/usr/local/bin/maenifold --http --port 8080` |
| gate | `com.ma-collective.gate` | 8081 | `/usr/local/bin/gate --http --port 8081` |

Plist files: `~/Library/LaunchAgents/com.ma-collective.{maenifold,gate}.plist`

**After building a new binary, restart the service to pick up changes:**

```bash
# Restart maenifold
launchctl bootout gui/$(id -u) ~/Library/LaunchAgents/com.ma-collective.maenifold.plist
launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.ma-collective.maenifold.plist

# Restart gate
launchctl bootout gui/$(id -u) ~/Library/LaunchAgents/com.ma-collective.gate.plist
launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.ma-collective.gate.plist
```

**Verify running:** `launchctl list | grep ma-collective`

## ACP Architecture

The ACP client architecture is specified in [docs/acp-client-architecture.md](docs/acp-client-architecture.md). That document is law — do not invent architecture outside it.

Key invariant: **`prompt` is the only primitive.** Think and workflow are orchestration loops that chain prompt calls. Neither should have a direct path to the ACP agent.

## ACP Model Policy

All ACP agent interactions MUST specify `model` explicitly. Never rely on agent defaults — different agents default to different models with wildly different latency and cost profiles.

The only valid ACP invocation path is `AcpExec` (via MCP `--tool` or the MCP protocol). There are no CLI aliases. `model` is a required parameter with no default.

## ACP Golden Tests (AcpExec E2E)

See [docs/acp-golden-tests.md](docs/acp-golden-tests.md) for the full test spec covering prompt, think, and workflow modes with memory artifact verification.

## Web UI / ACP Rules

- WebSocket URI must include `?agent=<name>`. Never connect without it.
- No default agent fallback. Missing `?agent=` must be rejected with an error.
- PM smoke tests must connect to a named agent. Bare `/ws/acp` is a test failure.
