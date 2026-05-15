# RALPH & META — Operator's Guide

This folder is maenifold-rs's **autonomous improvement loop**, modeled on [autoagent](https://github.com/kevinrgu/autoagent). It lets agents drive both the codebase (RALPH) and the harness those agents use (META) without a human in the loop.

If you want to know what to push, what to watch, and what each file does — read this.

---

## TL;DR

Two nested loops, two prompts, two shells, one score:

```
┌───────────────────────────────────────────────────────────────────┐
│  meta.sh        ◀───  outer loop  (optimizes the harness)         │
│   └─ runs meta-prompt.md                                          │
│       └─ inside: ./ralph.sh   ◀───  inner loop                    │
│           └─ runs prompt.md                                       │
│               └─ dispatches SWE / red-team / blue-team / etc.     │
│                                                                   │
│  Score:  F = 0.75·H + 0.15·T + 0.05·S + 0.05·C                    │
│  Goal:   monotonically increase F across meta-iterations          │
└───────────────────────────────────────────────────────────────────┘
```

- **`./ralph.sh`** — runs the PM (RALPH) in a 50-iteration loop. Each iteration: groom backlog → decompose epics → spec → dispatch → ship → hygiene.
- **`./meta.sh`** — runs the META meta-agent in a 10-iteration loop. Each meta-iteration: diagnose harness → propose ONE change → run RALPH inner loop → score → keep or revert.
- The **score** F is what META optimizes. The 75% holdout weight is what stops META from cheating.

---

## What's in this folder

| File | Role | Editable by | Notes |
|---|---|---|---|
| `prompt.md` | RALPH (inner-loop PM) protocol | META can edit it | Full lifecycle PM — owns the repo for one iteration |
| `meta-prompt.md` | META (outer-loop) protocol | **Hash-locked.** No one edits during a META run. | Defines the score function, keep/discard rules, verdict schema |
| `AGENTS.md` | Scope constraints for RALPH agent | You, manually | Allowed/off-limits paths |
| `state/learnings.md` | Operational lessons from past runs | RALPH appends per iteration | Read this before debugging — many failures are recurrences |
| `state/converged` | RALPH convergence marker (touch file) | RALPH creates; you delete to reset | Stops the inner loop |
| `state/meta-converged` | META convergence marker | META or shell creates after 3 consecutive reverts | Stops the outer loop |
| `state/meta-baseline.json` | Gate snapshot + F components captured before each META iteration | `meta.sh` writes; META reads | Source of truth for `score_before` |
| `state/meta-verdict.json` | Single-iteration verdict (kept/reverted, scores, reason) | META writes; `meta.sh` reads then deletes | **Mandatory.** Missing verdict = automatic revert. |
| `state/meta.lock` | Lockfile preventing concurrent META runs | `meta.sh` only | If `meta.sh` won't start, check this. |
| `state/baseline/<iter>/` | Per-iteration snapshot: pre-iteration `HEAD`, immutable hashes, untracked file set | `ralph.sh` writes before dispatch | Used by rollback to restore exact baseline. Safe to `rm -rf` between runs. |
| `state/quarantine/<iter>-<ts>/` | Untracked files created by an iteration that was reverted, plus `MANIFEST.txt` / `REASON.txt` / `BASELINE_HEAD.txt` | `ralph.sh` writes during rollback | Inspect before discarding. To recover, `mv` files back into the worktree. Safe to `rm -rf` once reviewed. |
| `state/rollback-failed` | Sentinel — the last rollback could not restore the baseline | `ralph.sh` writes; you delete after manual recovery | While present, `state/converged` will not be honored as a green light. |

**Rollback semantics (T-240.4).** When an iteration is reverted, RALPH:
1. Quarantines iteration-created untracked files (set difference vs. the pre-iteration snapshot) into `state/quarantine/<iter>-<ts>/`.
2. Runs `git reset --hard <baseline_HEAD>` to restore tracked state.
3. Validates `HEAD`, `git diff`, `git diff --cached`, immutable hashes, and untracked residue all match baseline. If anything fails, it writes `state/rollback-failed`, removes any `state/converged` marker, prints a summary, and exits 2.
4. The 3-consecutive-revert convergence shortcut only fires when the most recent rollback validated clean.

The shells live one level up at the repo root: **`/ralph.sh`** and **`/meta.sh`**.

---

## The two loops

### RALPH — inner loop (the PM)

**Run it:**
```bash
./ralph.sh                                     # 50 iterations, default model
./ralph.sh --max-iterations 10                 # short run
./ralph.sh --crate maenifold-gate              # scope to one Rust crate
./ralph.sh --path apps/desktop                 # scope to UI
./ralph.sh --model gpt-5.4 --max-iterations 5  # try a different model
```

**What it does each iteration:**
1. Loads `maenifold` + `gate` skills.
2. Syncs gate (`gate sync --root .`), reads backlog.
3. Reads `AGENTS.md` (constitution), `learnings.md`, `copilot-instructions.md`.
4. States its scope (`RALPH_TARGET_CRATE` / `RALPH_TARGET_PATH`).
5. Searches the knowledge graph for prior work on whatever it's about to touch.
6. Picks the next pending gate task from `todo/`.
7. Does **one** of: groom, decompose epic, write spec, dispatch SWE/red-team/blue-team agent, run PM-owned gate pass/fail, or hygiene work.
8. Builds + tests for its scope. On success: commits + `gate pass` + writes a memory note. On failure: records via `gate fail`, appends pattern to `learnings.md`.
9. Dispatched agents do **not** advance gate state themselves. They record findings with `gate append`, then RALPH decides whether to `gate pass` or `gate fail`.
10. Loops until `state/converged` exists OR max iterations reached.

**Stop it:** `Ctrl+C`, or `touch ralph/state/converged` from another terminal (loop checks before each iteration).

**Reset for a fresh run:** `rm ralph/state/converged`.

### META — outer loop (the harness optimizer)

**Run it:**
```bash
./meta.sh                                            # 10 meta-iterations × 20 RALPH iterations each
./meta.sh --max-iterations 3 --inner-iterations 5    # short test run
./meta.sh --inner-model gpt-5.4 --crate maenifold-gate
```

**What `meta.sh` does each meta-iteration (the outer mechanics):**
1. Acquires `state/meta.lock` (refuses to run if another META is active).
2. Hashes immutable files (`meta-prompt.md`, `meta.sh`) — verifies after.
3. Tags a git snapshot (`meta-snapshot-N-<epoch>`).
4. Clears `state/converged` and `state/meta-verdict.json` from the prior run.
5. Captures gate baseline → `state/meta-baseline.json`. Counts files in `done/`.
6. Exports `META_*` env vars for the meta-agent to consume.
7. Runs Copilot CLI with `meta-prompt.md` as the prompt.
8. After Copilot exits, checks (in order): immutable hashes intact? verdict file exists & valid JSON? `kept: true`? Copilot exit code 0?
9. **If any check fails → `git reset --hard <snapshot-tag>`** (full revert). Otherwise → tags `meta-accepted-N-<epoch>`.
10. Tracks `consecutive_reverts`. Hits 3 → auto-creates `state/meta-converged`.
11. Loops.

**What META (the agent) does each meta-iteration (the cognitive work):**
Runs the `meta-harness-optimization` workflow (in `assets/workflows/`):

```
capture-baseline → diagnose-bottleneck → propose-change
                 → run-inner-loop → evaluate-outcome → record-verdict
```

In `evaluate-outcome` it computes the F-score (next section). In `record-verdict` it writes `state/meta-verdict.json` — that file is the **only** way to tell `meta.sh` "keep this." Anything else = revert.

**Stop it:** `Ctrl+C`, or `touch ralph/state/meta-converged`.

**Reset:** `rm ralph/state/meta-converged ralph/state/meta.lock`.

---

## The score function

**This is what META optimizes. Memorize it.**

```
F = 0.75 × H + 0.15 × T + 0.05 × S + 0.05 × C
```

| Term | Weight | What it measures | Computed |
|---|---|---|---|
| **H — Holdout** | 0.75 | Replay of frozen pipelines META can't see or modify | `passing / 3` |
| **T — Train** | 0.15 | Live gate signal during the inner-loop run | `delivered_tasks × (1 − red_team_findings/task) × first_pass_rate`, normalized |
| **S — Stability** | 0.05 | Variance of F across iterations | `1 − var(F_n − F_{n−1})`. **=0 when N<3 (warmup)** |
| **C — Cost** | 0.05 | Token / wall-clock spend vs baseline | `1 − (this_tokens / baseline_tokens)`, clipped to [0,1]. **=0 on first iteration** |

### Why 75% holdout?

Heavy weight on a hidden benchmark META **cannot see** is the structural defense against [Goodhart's Law](https://en.wikipedia.org/wiki/Goodhart%27s_law). META can game any visible signal given enough iterations; it cannot game what it can't observe. The autoagent overfitting test is reinforced by this weighting: *"if this exact diagnosis disappeared, would this still be a worthwhile improvement?"*

### The frozen H pipelines (immutable benchmark)

These three completed pipelines are the holdout. Do not modify them. Do not regenerate them. Do not pick alternates mid-loop.

| Pipeline ID | Source |
|---|---|
| `ftk-analysis-1776357291320` | Completed analysis pipeline |
| `ftk-analysis-1776362447817` | Completed analysis pipeline |
| `synopsys-final-1776283202985` | Completed synopsys pipeline |

Each META iteration replays these (via the maenifold `Pipeline` tool) and counts how many produce the same node completions and conclusions as their original. `H = passing_count / 3`.

If a pipeline definition or underlying tool changes such that one of these can no longer replay → that's a **P0**: META should write `kept: false, reason: "H benchmark broken"` and escalate via memory note.

### Worked example

Iteration 4. RALPH delivered 3 tasks; 1 red-team finding; 2/2 blue-team first-pass. Replay 3/3. Cost 1.1× baseline. Variance low (S norm = 0.10).

```
H = 3/3                    = 1.00
T = 3 × (1 − 1/3) × (2/2)  = 2.00 → normalized vs budget(5) = 0.40
S = 1 − 0.10               = 0.90
C = 1 − 1.10               = −0.10 → clipped to 0.00

F = 0.75(1.00) + 0.15(0.40) + 0.05(0.90) + 0.05(0.00)
  = 0.855
```

### Keep / Discard rules (autoagent verbatim)

1. `F_after > F_before` → **keep**
2. `F_after == F_before` AND harness simpler (fewer LOC, smaller surface) → **keep**
3. Otherwise → **revert**

The shell enforces revert via `git reset --hard <snapshot-tag>`. META cannot keep a change just by committing — it has to write `kept: true` to the verdict.

---

## Verdict file contract

`state/meta-verdict.json` is the only signal META gives the shell. Schema:

```json
{
  "kept": true,
  "reason": "Reordered workflow to put diagnosis before hypothesis; H went 2/3 → 3/3.",
  "surface": "assets/workflows/meta-harness-optimization.json",
  "hypothesis": "Diagnosis-first workflow ordering will reduce wasted hypothesis cycles",
  "score_before": { "F": 0.785, "H": 0.667, "T": 0.40, "S": 0.90, "C": 0.50 },
  "score_after":  { "F": 0.855, "H": 1.000, "T": 0.40, "S": 0.90, "C": 0.00 },
  "tasks_before": 12,
  "tasks_after":  15,
  "done_before":  47,
  "done_after":   50,
  "warmup": false,
  "simplicity_delta_loc": -8,
  "overfitting_check": "Yes — diagnosis-first is generally better, not specific to this iteration's bottleneck."
}
```

**Missing or malformed verdict file = automatic full revert.** No exceptions.

---

## Operational lifecycle

### Cold start (you've never run this before)

```bash
# 1. Verify nothing's left over from prior runs
ls ralph/state/
# Should see: AGENTS.md (in parent), learnings.md, .gitkeep

# 2. Make sure the gate backlog has work
gate status --root .

# 3. Try RALPH alone first — small run
./ralph.sh --max-iterations 3

# 4. Inspect what it did
git log --oneline -10
ls done/
cat ralph/state/learnings.md     # last entries are the most recent

# 5. If RALPH looks healthy, try META — minimum viable
./meta.sh --max-iterations 2 --inner-iterations 3

# 6. Inspect META outcome
cat ralph/state/meta-verdict.json
git log --oneline -10            # 'meta:' commits = kept; 'meta-snapshot-*' tags = baselines
```

### Healthy steady-state

```bash
# Run META overnight
./meta.sh --max-iterations 10 --inner-iterations 20 &
disown

# Check on it
tail -f ralph/state/learnings.md     # RALPH operational notes
cat ralph/state/meta-verdict.json    # latest verdict
git log --oneline --grep='^meta:'    # all kept META changes
```

### After a META run completes

1. **Read every `meta:` commit** since the run started. Was the change reasonable? Did the score actually improve, or did META find a way to game T?
2. **Check H stability.** Run the 3 frozen pipelines manually — do they still pass? If H dropped, the frozen pipelines may have aged (a real backend change broke them).
3. **Read memory notes in `mae-auto/meta/`** for the rationale behind each kept change.
4. **If META got stuck reverting:** look at `consecutive_reverts` log lines. After 3 in a row it auto-converges. Read the last 3 verdict files to understand why.

---

## Failure modes (in order of likelihood)

### 1. RALPH writes a hallucinated spec

The Sprint 201 RCA is the canonical example: specs that reference `maenifold-cli serve` (wrong — actual is `maenifold --http`), `GET /health` (doesn't exist), or hardcoded preset agents (configured in `maenifold.json`).

**Defense:** `prompt.md` has a "Spec grounding (mandatory)" section with verification rules. Any task that touches an external surface (CLI, HTTP, MCP) must use the `tdd` pipeline so tests force grounding.

**If you see one in flight:** kill RALPH, fix the spec by hand, restart.

### 2. META games T while crashing H

If you see a kept iteration where T jumped a lot but H dropped — that's the gaming pattern. The 0.75 weight should make F go down anyway, but check the math.

**Defense:** F formula in both `meta-prompt.md` and the workflow `evaluate-outcome` step. If they ever drift apart again, **the agents will compute different scores.**

### 3. Verdict file missing or malformed

`meta.sh` will full-revert and you may not realize META did real work. Check:

```bash
git log --oneline --grep='meta-snapshot' --since='1 day ago'
git diff <last-snapshot-tag> HEAD
```

If you see commits between two snapshot tags that didn't make it into a `meta-accepted-*` tag → those are reverts. Read the iteration's verdict file (or its absence) for cause.

### 4. Lockfile orphaned

If `meta.sh` won't start with "already running" but no process exists:
```bash
rm ralph/state/meta.lock
```

### 5. Immutable file modified mid-iteration

`meta.sh` hashes `meta-prompt.md` and `meta.sh` itself before each iteration. If the hash changes during a run → automatic full revert. Don't edit these files while META is running.

### 6. Score formula drift

`meta-prompt.md` and `assets/workflows/meta-harness-optimization.json` MUST agree on the F formula. If you change one, change the other in the same commit.

```bash
grep -n "0.75" ralph/meta-prompt.md assets/workflows/meta-harness-optimization.json
```

Both files should show the same formula.

---

## What you (the human) are responsible for

The constitution says agents never touch git destructively. You own:

- **Branch + release management** (no agent does `git push`, `git rebase`, `git merge`, `git stash`, etc.)
- **Backlog seeding** — you write the first batch of `todo/T-*.md` task files. RALPH grooms and decomposes; you decide what's worth working on at the strategic level.
- **Authorizing META runs** — META is hands-off the source code, but it edits prompts and workflows. Make sure you actually want autonomous prompt edits before you start it.
- **Reading verdicts** — META's choices compound. A bad-but-kept iteration corrupts the next baseline.
- **Refreshing the H benchmark periodically** — when the 3 frozen pipelines drift far enough from current backend reality that they can't replay, replace them (and document the swap in `mae-auto/meta/`).
- **Pruning git tags** — `meta-snapshot-*` tags are deleted automatically after each iteration; `meta-accepted-*` tags persist. Periodically `git tag -l 'meta-accepted-*' | head -100` to see history.

---

## Customization knobs

### Changing the inner-loop model
```bash
./meta.sh --inner-model gpt-5.4
```
Or edit the default at the top of `meta.sh` (`INNER_MODEL=`).

### Changing the meta-agent model
```bash
./meta.sh --model claude-opus-4.7
```
Default is `claude-opus-4.6-1m`. Use `claude-opus-4.7` if you want stricter reasoning at higher cost.

### Scoping to a single area
Both shells accept `--crate <name>` and `--path <path>`. RALPH then refuses to touch files outside that scope. Useful for parallel runs in different terminals on different crates.

### Changing iteration budgets
- `./ralph.sh --max-iterations N` (default 50)
- `./meta.sh --max-iterations N --inner-iterations M` (default 10×20)

For a quick smoke test: `./meta.sh --max-iterations 1 --inner-iterations 2`.

### Changing what META can/can't edit
Mutable surfaces are listed in `meta-prompt.md` under "Mutable surfaces". Immutable surfaces are listed under "Immutable surfaces" AND hash-locked in `meta.sh` (`IMMUTABLE_FILES` array). Both must agree.

### Changing the H benchmark
Edit two places consistently:
- `meta-prompt.md` "Frozen benchmark (the H pipelines)" table
- `assets/workflows/meta-harness-optimization.json` `capture-baseline` and `evaluate-outcome` step descriptions

---

## File-by-file summary

```
/ralph.sh                                  Inner loop driver (50 iter, runs prompt.md)
/meta.sh                                   Outer loop driver (lockfile, hashes, snapshots, revert)
/ralph/
  prompt.md                                RALPH PM protocol — full lifecycle ownership
  meta-prompt.md                           META harness-optimizer protocol (HASH-LOCKED)
  AGENTS.md                                Scope rules for the RALPH agent
  README.md                                ← you are here
  state/
    learnings.md                           Append-only operational learnings
    meta-baseline.json                     Per-iteration gate snapshot + F components (shell-written)
    meta-verdict.json                      Per-iteration META verdict (META-written, shell-read)
    converged                              Touch file — stops RALPH
    meta-converged                         Touch file — stops META
    meta.lock                              Lockfile — prevents concurrent META
/assets/workflows/
  meta-harness-optimization.json           Workflow META follows (must agree with meta-prompt.md on F)
/memory/mae-auto/iterations/               RALPH writes one note per kept iteration
/memory/mae-auto/meta/                     META writes one note per kept meta-iteration
/todo/T-*.md                               Gate backlog
/done/T-*.md                               Gate completed tasks
```

---

## When in doubt

- **RALPH misbehaves:** check `state/learnings.md` for the pattern. Most failures recur.
- **META makes a confusing decision:** read its verdict file + the memory note in `mae-auto/meta/`.
- **Scores look wrong:** verify the F formula matches in both `meta-prompt.md` and the workflow JSON.
- **Want to see what changed:** `git log --oneline --grep='^meta:'` (kept) and `git tag -l 'meta-snapshot-*'` (attempted).
- **Want to start over:** `rm -f ralph/state/converged ralph/state/meta-converged ralph/state/meta.lock ralph/state/meta-verdict.json`.

We dogfood maenifold to build maenifold. Every MCP call RALPH makes IS a product validation. If a tool feels wrong from the agent's perspective — that's a bug. File it as a `todo/T-*.md` and let RALPH ship the fix.

---

## Live Monitoring

If you want to watch a run while it's happening instead of reading artifacts after the fact, use the monitoring sidecars in `ralph/state/`.

### Prerequisite

Install `otel-tui` once:

```bash
brew install ymtdzzz/tap/otel-tui
```

### Quick start

The fastest operator view is one command in one terminal:

```bash
./ralph.sh --watch
```

This re-launches itself inside a tmux session with `otel-tui` (live OTLP collector) in the top pane and the ralph iteration loop in the bottom pane. Ctrl+C tears both down cleanly. Requires `tmux` + `otel-tui`.

If you'd rather keep ralph and the watcher in separate terminals (e.g. you're already inside tmux, or you want the watcher to survive a ralph restart):

```bash
# terminal 1
./ralph.sh watch          # otel-tui collector, default mode
# or
./ralph.sh watch --both   # tmux split: otel-tui top, log tail bottom

# terminal 2
./ralph.sh
```

### What to watch

- `ralph/state/otel/*.jsonl` — OTEL traces emitted during the run: token usage, tool calls, model, cost, and duration.
- `ralph/state/logs/iter-N.log` — raw Copilot CLI output for each iteration while it is in flight.
- `ralph/state/sessions/iter-N.md` — full session dump written after the iteration completes.
- `ralph/state/metrics.jsonl` — one JSON line per iteration summarizing outcome and aggregate metrics.

### Manual usage

If `./ralph.sh watch --both` is not enough, the underlying views are simple:

```bash
# Watch OTEL traces directly (live collector mode):
./ralph.sh watch

# Gate status:
gate status --root .

# Tail the latest log:
tail -f ralph/state/logs/iter-*.log
```

Recommended workflow:

1. Run `./ralph.sh --watch` in a single terminal.
2. Keep an eye on `otel-tui` for model/tool/cost drift.
3. Use the iteration log when an agent appears stuck or starts looping.
4. After completion, read the matching session dump for the full transcript.
