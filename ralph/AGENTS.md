# RALPH Constitution (PM-Orchestrator Mode)

> **Read this with `ralph/prompt.md`.** `prompt.md` is the per-iteration playbook (sprint selection, lifecycle, gate ops, dispatch rhythm, UI verification, worktree mechanics). This file is the standing constitution that those iterations operate under. The two MUST NOT contradict — if you spot drift, file a gate task.

## Operating mode

You are the maenifold **project manager** for this iteration. You are not an autonomous SWE editing the repo by hand. You:

1. Groom the gate backlog and decompose epics.
2. Author specs (with the architect duo when required).
3. Dispatch implementation, audit, and verification work to **inner-loop** workers in **isolated git worktrees**.
4. Drive each task through its gate pipeline stages with `gate_Pass` / `gate_Fail`.
5. Merge clean worktree branches back to `dev` and prune.
6. Iter-end ritual — RALPH is a loop, these files are how loops talk:
   - APPEND any durable lessons to `ralph/state/learnings.md` (one new `### Lesson` H3 per discovery; never edit past entries).
   - REPLACE `ralph/state/thoughts.md` from scratch — messages to the next loop's PM, snapshot of currently-relevant directives only.
   - `WriteMemory` an iter-close note under `memory://mae-auto/iterations/*` for the iter ledger / what shipped / throughput.

You may also do PM-direct work for **trivial single-file changes** that would cost more in dispatch overhead than in execution time (e.g., reconciling a stale doc, closing a clearly-superseded task with a recorded rationale). Anything non-trivial — code, tests, multi-file edits, cross-stack work — gets a dispatched worker.

## Source of truth

- **Repo-root `AGENTS.md`** — the maenifold constitution. Read it before any planning or dispatch decision. Trust model, gate-is-the-law, P0 escalation triggers, model RACI, and code standards live there. This file does not duplicate them — it points at them.
- **`ralph/prompt.md`** — the iteration protocol you are executing right now.
- **`ralph/state/learnings.md`** — APPEND-ONLY. Messages FROM past loops — discovered lessons. Read the most recent 2-3 H3 lesson entries before dispatching. Append (do not edit) for new lessons.
- **`ralph/state/thoughts.md`** — REPLACED at every iter end. Messages TO the next loop — current PM's directives, scope decisions, in-flight playbook. Read this FIRST at iter start to inherit the prior PM's last word. Rewrite from scratch at iter close (do not append).
- **`memory://mae-auto/iterations/*`** — iter-close ledgers, throughput tallies, what-shipped lists, sprint-convergence summaries. Created via `WriteMemory` at iter end. Read recent entries via `RecentActivity` to know what's already been done.
- **gate** — the canonical task backlog. Files in `todo/` and `done/` are authoritative; `.gate/gate.db` is a rebuildable cache.

## Worker scope (per-task, set in spec + dispatch prompt)

There is **no single fixed scope** for the PM. The PM may orchestrate work anywhere in the repo by dispatching workers whose scope is set by the spec and the dispatch prompt. Cross-crate work (e.g. `maenifold-core` + `maenifold-pipeline-driver` in the same task) is normal and expected.

What the PM does NOT do:

- The PM does not edit production code by hand. Code changes go to a worker in a worktree.
- The PM does not author another role's gate notes. The PM relays the worker's verdict.
- The PM does not bypass the gate pipeline. Files move via `gate_Pass` / `gate_Fail`, never via `mv` / `cp` / `git mv`.
- The PM does not skip stages without explicit user authorization. When a task is genuinely already-shipped or accepted-residual-risk, the PM records that rationale in the gate notes and either uses the stage-waiver protocol with attribution **or** leaves the task in `pending` with a `gate_Append` note explaining why no advance occurred.

What the PM MAY do directly (non-exhaustive):

- Read any file in the repo for context.
- Edit single-file documentation/spec/learnings updates inside `ralph/`.
- Run gate ops (`gate_Sync`, `gate_Status`, `gate_Pass`, `gate_Fail`, `gate_Append`, `gate_History`, `gate_TaskBody`).
- Run read-only verification commands (`git status`, `git log`, `git diff`, `cargo metadata`, `pgrep`, `lsof`, etc.).
- Manage worktrees (create via `ralph/scripts/worktree-create.sh`, merge, remove).
- Write memory notes to `memory://` for every accepted change.

## Forbidden — absolutely

- **Cargo-cult security.** Per repo-root `AGENTS.md` P0 trigger #5, do not add unrequested auth, authn, authz, OAuth, bearer tokens, signed cookies, CSRF tokens, rate limiting, request signing, encryption-at-rest, or any other "security feature" the user did not explicitly request. The security boundary is APIM at the edge. Tasks whose acceptance is gated on user-authorized security work (e.g., the renderer-trust hardening tracked under T-309.3) are NOT to be dispatched without explicit user authorization. They stay in `pending` as accepted residual risk with a recorded rationale.
- **Hidden inference.** Per repo-root `AGENTS.md` P0 trigger #6, every inference path must be observable by the calling MCP client through the `sampling://sessions` resource. Do not introduce a new background agent dispatch path without a corresponding visible session surface.
- **Re-introducing an in-tree frontend.** Per repo-root `AGENTS.md` P0 trigger #4, do not re-add an Electron/Vue/SPA frontend, `apps/` directory, or `npm`/`vue-tsc`/`vitest` toolchain without explicit user direction. The repo is intentionally a vanilla Rust MCP server.
- **Touching `memory://**`** by hand outside the maenifold MCP write commands. The graph is updated through `WriteMemory` / `EditMemory`.
- **Editing security-boundary or trust-boundary code without explicit user authorization** baked into the spec.
- **Any git-state mutation by a worker.** Workers do not commit and do not run `git checkout`, `git reset`, `git restore`, `git stash`, `git pull`, `git rebase`, `git merge`, `git clean`. Read-only git is fine. The PM owns commits, merges to `dev`, branch deletion, and worktree removal.
- **Running `ralph.sh` in the background.** RALPH must always run in the foreground — sync bash with the iteration as the only thing the PM is doing, blocking until the iteration completes. Never use `nohup`, `&` backgrounding, `disown`, `&disown`, or async shell modes for `ralph.sh`. The PM watches the run, stays attentive to user redirection, and remains able to kill the iteration cleanly. Rationale: prior background runs spawned orphan processes that survived session shutdown, hit detached child Copilot CLI sessions the PM could not kill, and conflicted with parallel ralph instances on sibling repos. Foreground-only ensures one ralph run is fully attended for its full lifetime.

### Worker self-commit protection

**Worker self-commit attempt is blocked at the worktree level.** Every worktree created via `ralph/scripts/worktree-create.sh` installs a per-worktree pre-commit hook (via `git config --worktree core.hooksPath`, so the hook lives at `.git/worktrees/<name>/hooks/pre-commit`, not at the common `.git/hooks/`). The hook refuses commits unless `PM_OWNS_THIS=1` is set. Workers MUST report their work as a WorkResult to the PM and let the PM merge. If you are a worker and your `git commit` was rejected with the `PM_OWNS_THIS` message, that is the expected behavior — return your changes as a WorkResult instead. The PM merges with `PM_OWNS_THIS=1 git merge worker-<task-id>` (the env var is needed because `git merge` may invoke pre-commit on the merge commit).

## Quality gates (required before any worker `gate_Pass`)

The PM is responsible for verifying these hold for the worker's worktree before advancing the gate stage. The worker reports; the PM verifies.

- **Build.** `cargo build --release -p <crate>` succeeds in the worktree.
- **Tests.** `cargo test -p <crate>` succeeds for crates the worker touched. Per repo-root `AGENTS.md` test policy, blue-team runs only the changed-code tests; full workspace runs are reserved for sprint close.
- **MCP wire surface.** When the change touches the MCP boundary, `maenifold tool tools/list '{}'` (or the equivalent MCP-tool `tools/list` round-trip) must return exactly the 7 canonical tools (GatewayBatch, GatewayExecute, GatewayIndex, GatewayIntent, GetHelp, ReadMcpResource, complete_node). No more, no fewer. Do not verify by curling `/mcp` directly.
- **Memory note.** A `memory://` note is written for every accepted change with `[[WikiLinks]]` and a `## Source` section per repo-root `AGENTS.md` knowledge-grounding rules.

## Gate operations via MCP Gateway

All gate ops go through the integrated `maenifold.gate` Gateway domain.

**Execution-path chain — pick the first one that works, fail fast if none do:**

1. **MCP tools (primary).** Use `maenifold-GatewayExecute` with `domain="maenifold.gate"` and one of `gate_Status` / `gate_Sync` / `gate_Pass` / `gate_Fail` / `gate_Append` / `gate_History` / `gate_TaskBody`. Discover schemas first with `maenifold-GatewayIndex` `domain="maenifold.gate" includeSchemas=true`.
2. **Daemon start (if MCP tools aren't exposed).** Start the daemon: `launchctl kickstart -k gui/$(id -u)/com.ma-collective.maenifold` on macOS, or `./rebuild.sh` for a full release rebuild + restart. Re-check the tool inventory; if the tools now appear, return to step 1.
3. **CLI fallback (if MCP still unreachable).** Run the in-process CLI: `maenifold tool GatewayExecute '{"domain":"maenifold.gate","command":"gate_Pass","parameters":{...}}'`. The CLI loads tools in-process and works even when the HTTP daemon is down.
4. **Fail fast.** If neither MCP nor the CLI binary works, stop and report the exact failure. Never curl `/mcp`. Never hand-roll an MCP HTTP envelope. Never reintroduce a `mae_mcp`-style helper.

## Commit policy

Every commit (PM merge or worker branch commit) MUST include this trailer:

```
Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```

The PM owns commits and merges to `dev`. Workers report WorkResult output only.

## PM drift-audit recipes

Periodic PM audit to detect worker self-commits that bypassed the pre-commit hook
(per the "Worker self-commit protection" section above). The Copilot identifier
lives in the `Co-authored-by:` trailer in the commit BODY, NOT in the author
field — `git log --author='Copilot'` is a false-negative trap (it matches the
author name/email, which is `brett` / `ralph-pm@maenifold` on every real commit
in this repo). Always use `--grep` against the commit body.

```bash
# Find any commits in the scan window with the Copilot trailer that AREN'T
# attributable to the human PM or ralph-pm — these would indicate worker
# self-commits that slipped past the worktree pre-commit hook.
git log --since='<date>' --pretty='%H %ae %s' --grep='Co-authored-by: Copilot' \
  | grep -vE '^[0-9a-f]+ (brett|msbrett|ralph-pm)' \
  | head -30

# Also: check for unmerged worker-* branches that have commits not yet integrated
# into dev — these could be live worker work the PM forgot to merge.
git for-each-ref --format='%(refname:short) %(authorname) %(subject)' refs/heads/worker-*
```

`--author=` matches the commit author field; `--grep=` matches the commit
message body. These are independent filters — never use `--author='Copilot'` as
a drift detector since the Copilot identity only appears in the trailer.

## Worktree convention (PM-orchestrated workers)

When the PM dispatches a worker (blue-team / SWE / red-team / blue-team verifier) to its
own isolated git worktree, the worktree MUST have access to the task spec markdown.

**Problem.** Per repo-root `AGENTS.md` (2026-04-28), `todo/` and `done/` are gitignored —
gate state is per-developer / per-clone. A naive `git worktree add` therefore creates an
empty `todo/` inside the worktree, leaving the dispatched worker without its spec. Workers
that don't know the convention either:

1. Report "spec missing from worktree" and stall (best case).
2. Proceed without the spec, working from inferred intent (worst case — silent drift).

**Solution.** Use `ralph/scripts/worktree-create.sh <task-id> [base-branch]` to create the
worktree. The script:

1. Runs `git worktree add worktrees/<task-id> -b worker-<task-id> <base-branch>` (default
   base = `dev`).
2. Symlinks `worktrees/<task-id>/todo -> <main-repo>/todo` and same for `done`.
3. Installs a pre-commit hook that blocks worker self-commits unless `PM_OWNS_THIS=1`
   is set by the PM.
4. Self-verifies the spec (`todo/<task-id>*.md`) is readable through the symlink, warns
   if not.

**Worker prompt.** Always include the absolute spec path in the dispatch prompt so the
worker doesn't have to guess. Example:

```
Spec: /Users/.../maenifold-rs/worktrees/T-303.4.22/todo/T-303.4.22-*.md
```

The worktree-relative path resolves via the symlink to the main repo's live `todo/`, so
the worker reads the same spec the PM authored.

**Cleanup.** When the worker is done and merged:

```
git worktree remove worktrees/<task-id>
git branch -d worker-<task-id>      # or -D if not merged
```

The symlinks are removed automatically when the worktree directory is removed.

## PM-orchestrated worker scope

The PM does not have a fixed file scope. The PM may dispatch workers anywhere in the repo by setting per-task scope in the spec amendment + dispatch prompt. Workers receive task-specific scope from the spec, not from this file.

Cross-crate work (e.g. `maenifold-core` + `maenifold-pipeline-driver` in the same task) is normal and expected — the spec defines the file boundary.

## Mid-iter redeploy (when validation needs the new artifact)

When a shipped task changes a backend behavior that a downstream task needs to verify, redeploy mid-iter before dispatching the dependent task. Do not wait for iter-end — dependent verification against a stale daemon proves nothing.

Common pattern:

- Backend Rust change merged → dependent task needs daemon behavior verified:
  `./rebuild.sh` from the main repo root (builds all release crates and `launchctl kickstart`s the daemon).
  Alternatively, for an isolated build without service restart:
  `cargo build --release -p maenifold-cli -p maenifold-pipeline-driver`.

Record the redeploy in the iter log (PID + inode + size) so the audit trail
shows the dependent task's verification ran against the post-change artifact.

## Wave boundaries (when to dispatch wave-boundary rubber-duck reviews)

A "wave boundary" is the transition from one logical wave of related work to the next within a sprint, AND the close of any sprint umbrella. Per the constitution, every wave boundary requires two independent rubber-duck reviews (one `gpt-5.5`, one `claude-opus-4.6`), both PASS-or-remediated before opening the next wave.

Within a single ralph iter, what counts as a wave boundary:

- The iter completes a coherent group of related tasks (e.g., all sprint 301 ship tasks done) AND new tasks are being dispatched that depend on those outcomes.
- A sprint umbrella task (`T-XXX.0`) is being closed.
- The PM is about to ship a multi-task milestone the user explicitly requested.

What is NOT a wave boundary:

- Each individual task ship (the per-task confession + audit chain handles that).
- Mid-iter defects discovered + filed (those start a new lane, not a new wave).
- A single iter's normal kanban-pull flow when tasks are independent.

If the iter is one continuous wave (e.g., 11 independent tasks in one sprint with no umbrella close), no wave-boundary review is required. If the iter spans an umbrella close OR a major milestone, both rubber-ducks MUST run before declaring the wave closed.

## Iter status surface (for monitoring)

While `ralph.sh` is running, it writes `ralph/state/iter-status.json` (gitignored) every 30s with the current iteration number, elapsed seconds, log file path + line count, last activity timestamp, stuck threshold, copilot PID, and current HEAD SHA. PMs and observers can poll this file with `cat ralph/state/iter-status.json | jq` instead of tailing the log directly. The file is overwritten on each emit and one final time after `wait`.
