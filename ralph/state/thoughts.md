# RALPH Thoughts

Handoff from **iter-16** (2026-05-26, ~3.5h continuous). User triggered the iter via `/review` of the tray pipeline app + Gate UI request. **2 ships this iter**: T-251.7 (tray Pipeline Viewer shell + MCP Apps UI extension, recovered from prior PM's orphaned dev state) and T-251.9.1 (extract pipeline_app.rs HTML to assets/pipeline-app/ via include_str! foundation refactor).

## ⚠️ NEXT-PM FIRST READ — sprint 251 state + n8n DAG renderer roadmap

**Sprint 251 board (after iter-16):**
- **Done this iter (2):** T-251.7, T-251.9.1
- **Pending in `todo/`:**
  - T-251.3 (redact-secrets-from-config-inspection-surfaces) — untouched this iter, still pending
  - T-251.4 (preserve-http-mcp-auth-and-headers-in-pipeline-external-tool-execution) — untouched, still pending
  - T-251.8 (gate-mcp-app-ui-after-pipeline-viewer-path) — pending; deps on T-251.9.1 (now satisfied), can dispatch any time
  - T-251.9 (umbrella spec for pipeline DAG renderer via Drawflow) — describes the 4-child decomposition; the foundation child (T-251.9.1) is done

**User context (durable, from this session):**
- User explicitly authorized **Path A** (extend embedded-HTML + CDN-lib pattern, NO toolchain change, no `apps/` dir, no npm)
- User explicitly DELEGATED the DAG library pick → research locked it in as **Drawflow** (jerosoler/Drawflow, MIT, vanilla JS, ~80KB minzipped, CDN via jsDelivr; closest n8n-style visual match of the three viable vanilla candidates)
- User explicitly set the quality bar: **"Ensure they look GOOD. Visual UAT required, not just looking at code. Look at the screenshots too."** Every UI-touching child task MUST produce headless-Chrome screenshots at 1200×800 AND 520×720 (real tray min size), PM views them, attaches to Gate notes.
- User set model override: **`gpt-5.3-codex`** for roles where the constitution default is `gpt-5.4` (SWE, blue-team). Other roles unchanged.

## TL;DR — iter-16 result

| Task | Pipeline | Verdict | Merge |
|------|----------|---------|-------|
| T-251.7 | SDD | done | `cf144aa59` |
| T-251.9.1 | SDD | done | `20afb9d24` |

All landed via gate pipeline with PM worktree commits + `--no-ff` merges. Worktrees pruned, branches deleted. Memory note at `memory://mae-auto/iterations/iter16-t2517-tray-pipeline-viewer-and-t25191-html-extract-to-assets`.

## NEXT iter — dispatch order

1. **T-251.9.2** (Drawflow integration, TDD pipeline). Needs explicit spec file under `todo/T-251.9.2-*.md` before dispatch. Scope: replace card-grid `renderNode` logic in `assets/pipeline-app/app.js` with Drawflow node+connection calls driven by `PipelineView` JSON. Bring Drawflow from CDN (`https://cdn.jsdelivr.net/gh/jerosoler/Drawflow@<pinned-tag>/dist/drawflow.min.js` + `.min.css`). Implement dagre-style automatic layout (Drawflow ships none; either bring `dagre@0.8.x` from CDN or ~30 LOC topological-sort + level-assignment). Node coloring (completed=green, active=orange, ready=blue, idle=grey, failed=red). Edge styling (success=solid, workflow=dashed). **Visual UAT mandatory** — produce screenshots, PM views them inline before terminal_pass.
2. **T-251.8** (Gate viewer) — can dispatch **in parallel** to T-251.9.2 (different files: T-251.8 creates `assets/gate-app/*` + new MCP resource `ui://maenifold/gate`; T-251.9.2 only touches `assets/pipeline-app/app.js` and `index.html`). Pattern matches T-251.7+T-251.9.1: new MCP App resource + tray menu item.
3. **T-251.9.3** Selected-node inspector v2 + ACP/thinking deep-links (TDD; deps on T-251.9.2)
4. **T-251.9.4** Visual polish + dark-mode + interaction (SDD with mandatory multi-state visual UAT)
5. **Wave-boundary review** when T-251.7 + T-251.8 + T-251.9.{1..4} all done: dual-model rubber-duck (`claude-opus-4.6-1m` + a second model from different family — e.g., `gpt-5.3-codex`), both PASS-or-remediated before declaring closed.

## What went RIGHT

1. **Honest stop on `gpt-5.5` mismatch.** Caught my own model error (used `gpt-5.3-codex` for red-team when constitution defaults to `gpt-5.5`), discovered `gpt-5.5` doesn't exist in this environment, re-dispatched with `claude-opus-4.6-1m`. Both verdicts agreed PASS with high confidence — got the cross-family check the RACI intended.
2. **Spot-verified every agent claim.** Both red-teams cited file:line evidence; PM `view`'d the cited lines to confirm. Blue-team's "test already exists at line 347-349" claim was verified by reading the lines.
3. **Visual UAT actually shipped screenshots.** User said "look at the screenshots too" and I delivered three: baseline at 1200×800, baseline at 520×720 (real tray min size showing worst case), post-refactor at 1200×800 (proving byte-identical). PM viewed all three via the `view` tool's image rendering.
4. **NO PUNTING enforced.** Filed T-251.9 umbrella spec + dispatched T-251.9.1 first child + shipped it all in the same iter that discovered the gap.

## What to WATCH

1. **Worker can't commit** — PM-must-commit-in-worktree-BEFORE-merge is the easy-to-forget step. Symptom: `git merge` returns "Already up to date" + `git worktree remove` fails with "contains modified or untracked files." Fix: cd into worktree, `PM_OWNS_THIS=1 git commit -am "..."`, retry merge. Documented in iter-16 learnings.md.
2. **gpt-5.5 is not available** — use `claude-opus-4.6-1m` as canonical red-team substitute. The RACI table in repo-root AGENTS.md needs updating to reflect this environment reality (file a task if it persists).
3. **gtimeout not installed** on macOS default. Workers fall back to `python3 -c "subprocess.run(timeout=...)"`. Consider filing a task to add `brew install coreutils` to the maenifold-testing skill's setup docs.
4. **`assets/pipeline-app/` pattern is now established.** T-251.8 (Gate UI) should mirror it: `assets/gate-app/{index.html, app.css, app.js, snapshot.html}` + `include_str!` + OnceLock compose. The parity-test pattern is the regression guard template.
5. **WriteMemory canonical params:** `title` (NOT `name`), `content`, `folder`, optional `tags`. Content MUST contain at least one `[[WikiLink]]` or it errors. 250-line content limit per Ma Protocol. Schema: `GetHelp toolName=WriteMemory`.

## Environmental observations (NOT your scope to fix)

- **`~/Library/LaunchAgents/com.ma-collective.maenifold.plist` contains `AZURE_OPENAI_API_KEY` + `OPENAI_API_KEY` in plaintext.** User's responsibility; surfaced for their awareness only.
- **Stale worktrees not cleaned up by prior PMs:** `worktrees/T-336.10`, `T-336.10-clean`, `T-336.10-remediate`, `T-336.10-swe4`, `T-339.2.3`. Surface in iter-close for cleanup decision; low priority unless they're hogging disk.
- **~30 orphan `cargo test schedule run alpha-*` processes** spawned from `worktrees/T-336.10-swe4/target/agents/T-336.10-blue2/`. Some are live launchctl-registered schedules; some may be leaked test children. Investigate before any future T-336.10 cleanup (don't yank live schedules).

## Open from prior iters (still parked, not touched this iter)

- **T-309.3**: still constitution-blocked under [[anti-cargo-cult-security]] hold per AGENTS.md P0 #5.
- **T-303.4.21**: untouched.
- **T-338.1 / T-338.2 / T-338.3** (sprint-336-discovered help-corpus follow-ups): also untouched.

## NEXT iter — entry checklist

1. `cd /Users/brett/src/ma-collective/maenifold-rs && git log --oneline -5` — confirm `20afb9d24` (T-251.9.1 merge) is on dev.
2. `git reflog | head -3` — check for orphaned commits.
3. `git worktree list` — should show only the 5 stale worktrees + main; no in-flight T-251.* worktrees.
4. `git --no-pager status -sb` — should be clean (`## dev...origin/dev [ahead N]` only).
5. `ls todo/T-251*.md` — should show T-251.3, T-251.4, T-251.8, T-251.9 (umbrella).
6. `ls done/T-251.{7,9.1}*.md` — confirm both this-iter ships are in `done/`.
7. `cat /Users/brett/.copilot/session-state/0581ace8-c9cd-47d2-9acd-34ac760b63f8/plan.md` (or whatever the session-state dir is) — full plan + library decision rationale lives there.
8. Read this thoughts.md in full; read the 3 newest H3 entries in learnings.md.
9. Run `./rebuild.sh` only if you suspect daemon drift; otherwise pick up T-251.9.2 spec authoring + dispatch.

## Memories written this iter

- `memory://mae-auto/iterations/iter16-t2517-tray-pipeline-viewer-and-t25191-html-extract-to-assets`

End of iter-16 handoff.
