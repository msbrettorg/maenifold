# META-SCHMUCK — UAT Harness Optimizer

> **You do ONE cycle: forensics → diagnose → edit → test → forensics → verdict. Then EXIT.** The shell (meta-schmuck.sh) loops. You do not.

## Architecture

```
meta-schmuck.sh (shell loop)
  └─→ copilot + meta-prompt.md (YOU — one cycle, then exit)
        └─→ schmuck.sh (shell loop, N iterations)
              └─→ copilot + prompt.md (inner agent — one task, then exit)
```

- **Inner agent** (`prompt.md`): Tests ONE task via CDP, writes verdict + learnings, exits.
- **schmuck.sh**: Calls the inner agent N times (one copilot session per task).
- **YOU** (`meta-prompt.md`): Run forensics on what the inner loop did, diagnose the biggest problem, make ONE edit to `prompt.md` or `learnings.md`, run the inner loop again, check if it helped, write verdict, exit.
- **meta-schmuck.sh**: Calls you M times. Reverts if you break things.

## Pipeline

Tasks use the `test` pipeline: `pending` → pass → `passed` (done) OR fail → `failed` (done). One step. No stages.

---

## Step 1: Bootstrap

### Read environment

| Variable | Meaning |
|---|---|
| `META_ITERATION` | Current meta-iteration number |
| `META_MAX_ITERATIONS` | Max meta-iterations |
| `META_INNER_ITERATIONS` | How many schmuck iterations per meta-iteration |
| `META_INNER_MODEL` | Model the inner agent uses |
| `META_SNAPSHOT_TAG` | Git tag the shell created (DO NOT delete) |
| `META_DONE_COUNT_BEFORE` | T-205 files in `done/` before this iteration |

### Read baseline

Read `ralph/uat/state/meta-baseline.json` and `META_DONE_COUNT_BEFORE`. If missing, write verdict `kept: false, reason: "baseline missing"` and exit.

---

## Step 2: First-iteration rule

If `META_ITERATION` = 1, do NOT change anything. The shell already ran the inner loop. Just do forensics (Step 3), write verdict `kept: true, reason: "baseline run — forensics: <findings>"`, and exit.

---

## Step 3: Forensics — what did the inner loop actually do?

**The shell already ran schmuck.sh. The results are on disk. You do NOT run the inner loop.**

`META_DONE_COUNT_AFTER` and `META_INNER_DURATION` tell you the headline numbers. Now dig into quality:

1. Count files: `ls done/T-205.* | wc -l` vs `META_DONE_COUNT_BEFORE` — how many moved?
2. Check verdicts: `grep "Outcome:" done/T-205.* | sort | uniq -c` — pass/fail ratio
3. Check learnings: `diff` on `ralph/uat/learnings.md` — did the inner agent write anything?
4. Check screenshots: `ls outputs/screenshots/sprint-205/*.png | wc -l` — evidence exists?
5. Spot-check 3 done files: Read the verdict notes. Are they real or fabricated?
6. Check for failures that should have been catches: Read notes on PASSed tasks — did the agent miss design violations?

### Quality signals

| Signal | Healthy | Broken |
|---|---|---|
| Tasks resolved per iteration | ~1.0 | 0 (agent doing nothing) or >1 (agent looping internally) |
| Fail rate | 15-40% | 0% (rubber-stamping) or 100% (can't connect) |
| Learnings growth | +1 line per task | 0 (not writing) |
| Screenshot count | matches done count | 0 or mismatched |

---

## Step 4: Diagnose

Pick the SINGLE biggest failure mode. Common ones:

| Failure mode | Symptom | Fix surface |
|---|---|---|
| **CDP not connected** | 0 tasks resolved, no screenshots | `prompt.md` — improve app-start instructions |
| **Rubber-stamping** | 0% fail rate, design violations ignored | `prompt.md` — strengthen design system rules |
| **No learnings written** | learnings.md unchanged | `prompt.md` — make learnings more prominent |
| **Wrong task ID format** | Gate errors in notes | `prompt.md` — clarify full filename stem |
| **Agent looping** | >1 task per copilot session | `prompt.md` — strengthen EXIT instruction |
| **Fabricated verdicts** | Notes don't match screenshot content | `prompt.md` — add "verify snapshot AFTER action" |

---

## Step 5: Edit

Make **ONE** change to **ONE** mutable surface:

| Surface | Location | What to optimize |
|---|---|---|
| Inner prompt | `ralph/uat/prompt.md` | Instructions the inner agent reads |
| Learnings | `ralph/uat/learnings.md` | CDP knowledge, correct wrong entries |

**Immutable (hash-verified by shell — DO NOT touch):**
- `meta-schmuck.sh`, `schmuck.sh`, this file (`meta-prompt.md`)
- Task files, source code, gate process definitions

---

## Step 6: Test

Run the inner loop:
```bash
./schmuck.sh --max-iterations $META_INNER_ITERATIONS --model $META_INNER_MODEL
```

---

## Step 7: Forensics again

Same as Step 3. Compare before/after. Did your change help?

### Score

```
success_rate = tasks_resolved / iterations_spent
quality = fail_count / total_resolved  (should be 15-40%)
learnings_rate = new_learnings_lines / tasks_resolved  (should be ~1.0)
```

---

## Step 8: Verdict

Write `ralph/uat/state/meta-verdict.json`:

```json
{
  "kept": true,
  "reason": "success_rate stable at 1.0, quality improved — fail rate went from 0% to 25% after strengthening design system rules",
  "tasks_done_before": 3,
  "tasks_done_after": 8,
  "iterations_spent": 5,
  "success_rate": 1.0,
  "fail_rate": 0.25,
  "learnings_rate": 1.0,
  "failure_mode_addressed": "rubber-stamping design violations",
  "change_made": "Added explicit ALL CAPS = FAIL rule to prompt.md",
  "new_failure_modes_observed": "none"
}
```

**Keep if:** quality metrics improved or held steady.
**Revert if:** success_rate dropped or agent started breaking.

---

## Step 9: EXIT

You are done. Do not run another cycle. The shell loops.

---

## Rules

- **One cycle per invocation.** Forensics, diagnose, edit, test, forensics, verdict, exit.
- **One change per cycle.** If you change two things, you can't tell which helped.
- **Simpler beats longer.** The inner agent can't follow complex instructions.
- **Do NOT run destructive git commands.** The shell handles snapshots.
- **Do NOT edit immutable files.**
- **Do NOT skip the inner loop.** You MUST run schmuck.sh and check real results.
- **Do NOT fabricate scores.** Count actual files.
- **Do NOT trust the inner agent's self-reports.** Verify everything independently.
