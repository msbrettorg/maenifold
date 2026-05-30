# RALPH Thoughts

Handoff from **iter-16b** (2026-05-26, ~5h continuous). User triggered the iter via `/review` of the tray pipeline app + Gate UI request. **8 ships this iter** — sprint 251 advanced significantly: T-251.7 (recovered orphan), T-251.9.1 (assets refactor), T-251.8 (Gate kanban), T-251.9.2 (Drawflow integration), T-251.9.4 (fit-to-view), T-251.8.1 (env var + movable windows), T-251.10 (single-node centering), T-251.11 (derive stages from data).

## ⚠️ NEXT-PM FIRST READ — sprint 251 state + REAL visual UAT bar

**Sprint 251 board (after iter-16b):**
- **Done (10):** T-251.1, T-251.2, T-251.5, T-251.6, T-251.7, T-251.8, T-251.8.1, T-251.9.1, T-251.9.2, T-251.9.4, T-251.10, T-251.11 — 12 actually; the iter shipped 8 net new
- **Pending in `todo/`:**
  - **T-251.3** (redact-secrets-from-config-inspection-surfaces) — UNTOUCHED; real config-leak bug; TDD pipeline; READY
  - **T-251.4** (preserve-http-mcp-auth-and-headers-in-pipeline-external-tool-execution) — UNTOUCHED; real auth-drop bug; TDD pipeline; READY
  - **T-251.9** umbrella — its only unfiled child is T-251.9.3 (selected-node inspector v2 + ACP/thinking deep-links); decide whether to close umbrella or decompose

**User context (durable, from this session):**
- User authorized **Path A** (extend embedded-HTML + CDN-lib pattern, NO toolchain change, NO `apps/` dir, NO npm)
- User locked **Drawflow** as the DAG library (MIT, vanilla, jsDelivr CDN, pinned to `0.0.60`)
- User-explicit model override: `gpt-5.3-codex` for SWE/blue-team (where constitution defaults to gpt-5.4)
- User-explicit visual UAT bar: **"I have yet to see ANY visual UAT happen. Headless does not count."** — must drive real macOS UI via osascript + `screencapture -R` against the deployed tray viewer

## TL;DR — iter-16b result

| Task | Pipeline | Verdict | Merge |
|------|----------|---------|-------|
| T-251.7 | SDD | done | `cf144aa59` |
| T-251.9.1 | SDD | done | `20afb9d24` |
| T-251.8 | SDD | done | `c2fc6c8a7` |
| T-251.9.2 (+ remediation) | SDD | done | `ba4170222` |
| T-251.9.4 | SDD | done | `f3dec51fc` |
| T-251.8.1 | SDD | done | `1cb09eb11` |
| T-251.10 (+ remediation) | SDD | done | `fb5450752` |
| T-251.11 | SDD | done | `392d4d654` |

All 8 landed via gate pipeline (pending → swe → red-team → blue-team → done) with PM worktree commits + `--no-ff` merges. Worktrees pruned, branches deleted. Two remediation cycles in-iter (T-251.9.2: visibility + port allocation; T-251.10: precanvas internal usage caught by PM).

Memory note: `memory://mae-auto/iterations/iter16b-sprint-251-ships-8-tasks-tray-pipeline-dag-gate-kanban-endtoend`

## NEXT iter — dispatch order

1. **T-251.3** (redact-secrets, TDD, P1) — real config-inspection-surface leak: `/api/config` and `GetConfig` skip redaction on `headers`/`envFromFile`/`envFromCommand`. Spec is well-authored with concrete evidence at file:line in `crates/maenifold-core/src/api/routes_config.rs:9-13` and `crates/maenifold-core/src/tools/diagnostics.rs:36-145`. Worker scope is well-bounded.
2. **T-251.4** (preserve-http-mcp-auth-and-headers, TDD, P1) — pipeline external-tool path drops HTTP auth/headers that gateway startup honors. Spec is well-authored with file:line evidence at `external_mcp.rs:75-77` and `external_mcp_transport.rs:126-178`.
3. **T-251.9 umbrella** — decide: file T-251.9.3 (selected-node inspector v2 + ACP/thinking deep-links) and ship, OR close umbrella with "user complaint addressed; inspector v2 is polish not required for user need." Recommendation: ask user before opening T-251.9.3 — they may not need it given the Drawflow renderer + simple JSON inspector already work.
4. **Inherited stale worktrees**: `worktrees/T-336.10`, `T-336.10-clean`, `T-336.10-remediate`, `T-336.10-swe4`, `T-339.2.3`. Worth investigating before any cleanup because some have orphan `cargo test schedule run alpha-*` processes (~30 running). May be live launchctl schedules created during T-336.10 work that need de-registration.

## What went RIGHT

1. **REAL visual UAT replaced headless** — when user rejected headless screenshots, pivoted to osascript-driven tray menu click + atomic `screencapture -R` of the live wry window. Got `T-251.10-REAL-pipeline-centered.png` and `T-251.11-REAL-gate-with-derived-stages.png` showing actual deployed UI with real data. **This is now the visual UAT bar going forward.**
2. **NO PUNTING held under pressure** — every defect discovered during real-UAT (modal/unmovable windows, Gate empty state, single-node clipping, hardcoded stages) was filed + dispatched + shipped same iter. 4 ships were discovered-mid-iter; all landed.
3. **PM caught a constitution violation in flight** — T-251.10 SWE's first attempt used `editor.precanvas.style.transform` (private Drawflow internal flagged off-limits by T-251.9.4 red-team). PM caught it before accepting; used `write_agent` to send remediation guidance with documented alternatives; SWE rewrote with two-step `zoom_refresh()` workaround. **The `write_agent` channel preserves SWE context vs. dispatching a new agent.**
4. **Dual-model red-team agreement on T-251.7 close-affordance** — gpt-5.3-codex and claude-opus-4.6-1m independently returned identical PASS verdicts with overlapping file:line citations, giving high confidence in the recovery.
5. **Privacy-aware screenshot handling** — when an accidental fullscreen capture grabbed user desktop content, I deleted both privacy-affected screenshots immediately and switched to atomic osascript-driven window-bounds capture to avoid recurrence. Did not retain.

## What to WATCH

1. **`launchctl setenv` after launch does NOT propagate to running processes** — must kickstart-restart the daemon to inherit. User-driven env var changes (like `MAENIFOLD_GATE_ROOT`) need `launchctl setenv X Y && launchctl kickstart -k gui/UID/com.ma-collective.maenifold` then verify with `ps eww -p PID | grep X`.
2. **Gate parser rejects 4-level IDs** — max is `T-{sprint}.{task}.{sub}`. When decomposing a sub-sub-task, use parallel sprint-level IDs (T-X.10, T-X.11) with `discovered_in:` frontmatter for provenance.
3. **Drawflow's `editor.precanvas` is OFF-LIMITS** — multiple iters of red-team enforcement. SWEs see "easy fix via precanvas" and need explicit reminder. Build it into worker prompts when touching Drawflow.
4. **Drawflow `zoom_refresh()` short-circuits when zoom===zoom_last_value** — single-node DAG centering needs the two-step bump-then-restore workaround (now in learnings.md).
5. **Worker post-build refactors must trigger PM re-verification** — T-251.8.1 SWE honestly disclosed a post-build LOC refactor (no second cargo build per "ONE attempt" rule); PM re-ran cargo build and verified clean. This is now the pattern: SWE discloses, PM re-builds.
6. **REAL visual UAT requires Accessibility permission** for the host running osascript. Symptom of missing permission: empty process lists or 0 menu bars. Check System Settings → Privacy & Security → Accessibility.

## Open from prior iters (still parked, NOT touched this iter)

- **T-309.3**: constitution-blocked under [[anti-cargo-cult-security]] hold per AGENTS.md P0 #5.
- **T-303.4.21**: untouched.
- **T-338.1 / T-338.2 / T-338.3**: sprint-336-discovered help-corpus follow-ups; untouched.

## Environmental observations (NOT my scope to fix; user responsibility or future task)

- `~/Library/LaunchAgents/com.ma-collective.maenifold.plist` contains `AZURE_OPENAI_API_KEY` + `OPENAI_API_KEY` in plaintext. User's call to fix.
- `MAENIFOLD_GATE_ROOT` env var set via `launchctl setenv` this session — NOT persisted in the plist. After next reboot or `bootout`/`bootstrap` cycle, will be lost. User should add to plist EnvironmentVariables for persistence.
- 5 stale worktrees from sprints 336/339: `worktrees/T-336.10`, `T-336.10-clean`, `T-336.10-remediate`, `T-336.10-swe4`, `T-339.2.3`.
- ~30 orphan `cargo test schedule run alpha-*` processes spawned from `worktrees/T-336.10-swe4/target/agents/T-336.10-blue2/`. May be live launchctl-registered schedules; needs investigation before removal.
- `gtimeout` not installed on macOS host; workers fall back to `python3 subprocess.run(timeout=...)`. Could `brew install coreutils` to get it.
- `gate_Status` MCP command returns `todo/` only; `done/` not exposed. Out of scope for UI work but potential follow-up if user wants done/ visibility in Gate Board.

## NEXT iter — entry checklist

1. `cd /Users/brett/src/ma-collective/maenifold-rs && git log --oneline -10` — confirm `392d4d654` (T-251.11 merge) is on dev.
2. `git --no-pager status -sb` — should be clean (`## dev...origin/dev [ahead N]`).
3. `git --no-pager worktree list` — should show only the 5 stale T-336/T-339 worktrees + main.
4. `ls todo/T-251*.md` — should show T-251.3, T-251.4, T-251.9 (umbrella).
5. `ls done/T-251*.md | wc -l` — should be 12.
6. Read this thoughts.md in full; read the 5 newest H3 entries in learnings.md.
7. **Verify MAENIFOLD_GATE_ROOT is set in env** — `launchctl getenv MAENIFOLD_GATE_ROOT` should return the repo path. If not set, run `launchctl setenv MAENIFOLD_GATE_ROOT /Users/brett/src/ma-collective/maenifold-rs && launchctl kickstart -k gui/UID/com.ma-collective.maenifold`.
8. Pick T-251.3 or T-251.4 (or both in parallel — different files, no contention). Confirm with user if unclear.

## Memories written this iter

- `memory://mae-auto/iterations/iter16b-sprint-251-ships-8-tasks-tray-pipeline-dag-gate-kanban-endtoend`

End of iter-16b handoff.
