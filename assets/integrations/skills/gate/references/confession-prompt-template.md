# Confession Prompt Template

Use this template for any gate stage ending in `-confess` or `pm-confess`. The same agent who did the work produces the confession. The PM evaluates it. The next stage challenges it.

Adapt by replacing `{placeholders}` with your project's conventions.

---

## For SWE Confessions (`swe-confess`)

You just completed implementation work on `{task_id}`. Before your work advances, you must produce a ConfessionReport.

**This confession is scored only for honesty and completeness. Nothing you say here changes your implementation. Do not optimize for user satisfaction.**

Produce a ConfessionReport covering:

1. **Instructions received**: List every acceptance criterion and constraint from the task spec.
2. **Compliance**: For each, state ✅ or ❌ with specific evidence (file path, line number, command output).
3. **What you actually verified**: Did you run the app? Hit the endpoint? Open the page? Or did you only run `{scoped_build_command}`? Be specific.
4. **Surfaces referenced**: List every API endpoint, CLI command, MCP tool, or external surface your code calls. For each, state whether you verified it exists in the codebase.
5. **Gaps**: Anything you were supposed to do but didn't, or couldn't verify.
6. **Shortcuts**: Any hacks, suppressions, or workarounds you introduced.
7. **Files modified**: List with line ranges.

---

## For Red-Team Confessions (`red-team-confess`)

You just completed an adversarial audit on `{task_id}`. Produce a ConfessionReport covering:

1. **Audit scope**: What files did you actually read? What did you skip?
2. **Surface verification**: Did you verify that all API endpoints, CLI commands, and MCP tools referenced in the code actually exist? For each, state how you verified (grep, LSP, --help, or "I did not check").
3. **Compliance with spec**: For each acceptance criterion, did you verify the implementation satisfies it? ✅/❌ with evidence.
4. **What you did NOT test**: Be explicit about what's outside your verification. Did you run the app? Did you only read code?
5. **Gaps**: Anything suspicious you noticed but didn't flag as a FAIL.
6. **Shortcuts**: Did you accept any claim from the SWE's notes without independent verification?

---

## For Blue-Team Confessions (`blue-team-confess`)

You just completed verification on `{task_id}`. Produce a ConfessionReport covering:

1. **Commands run**: List every exact command you executed and its output summary.
2. **What "pass" means**: Did you verify the feature works end-to-end, or only that it compiles/typechecks? Be specific.
3. **Integration check**: If the code calls an API endpoint, did you verify the endpoint responds? How?
4. **Visual verification** (UI tasks): Did you see the UI render correctly? How? (Playwright screenshot, manual browser, or "I did not visually verify")
5. **Gaps**: Anything the test commands cannot catch that you're aware of.
6. **What you trusted without verifying**: Any SWE or red-team claims you accepted at face value.

---

## For PM Confessions (`pm-confess`)

You are the PM who orchestrated `{task_id}` through the entire pipeline. Before final audit, produce a ConfessionReport covering:

1. **Spec quality**: Did you verify every API surface, endpoint, and CLI command in the spec exists in the codebase before dispatching SWE? ✅/❌ with evidence.
2. **Agent verdicts**: For each stage (SWE, red-team, blue-team), what did you actually verify about their work vs what did you accept at face value?
3. **Confessions reviewed**: For each agent's confession, did you read it? Did you challenge any claims? List specific challenges or "I accepted without challenge."
4. **Definition of done**: Does this task meet the project's definition of done? (Backend: release build. UI: typecheck + visual verification. Both: release build works.)
5. **What you skipped**: Anything you should have checked but didn't. Any stage where you accepted a verdict without independent spot-checking.
6. **Risk assessment**: What is the most likely way this task is broken despite passing all stages?

---

## For Red-Team Audit (`red-team-audit`)

You are a **fresh** red-team agent conducting the final audit of `{task_id}`. Read the full task history including ALL confessions.

Your job is to **falsify the PM's confession**. Specifically:

1. **Cross-reference**: Compare what the PM claims they verified against the actual git diff and task file records.
2. **Challenge gaps**: If ANY confession (SWE, red-team, blue-team, or PM) disclosed a gap, was that gap addressed or is it still open?
3. **Surface check**: Independently verify that all API endpoints, CLI commands, and MCP tools referenced in the code exist in the codebase.
4. **Spot check**: Pick at least one acceptance criterion and verify it against the actual code, not the agent's claim.
5. **Verdict**: PASS only if the confessions are honest AND the gaps are acceptable. FAIL if any confession was dishonest, if disclosed gaps were not addressed, or if you find issues not disclosed in any confession.

On FAIL: cite the specific confession claim that was false or the undisclosed issue you found.

---

## Stage → Prompt Mapping

| Gate Stage | Prompt Template | Agent |
|------------|----------------|-------|
| `pending` | (PM dispatches) | PM |
| `swe` | `swe-prompt-template.md` | SWE |
| `swe-confess` | This file — SWE section | Same SWE |
| `red-team` | `red-prompt-template.md` | Red-team |
| `red-team-confess` | This file — Red-team section | Same red-team |
| `blue-team` | `blue-prompt-template.md` | Blue-team |
| `blue-team-confess` | This file — Blue-team section | Same blue-team |
| `pm-confess` | This file — PM section | PM |
| `red-team-audit` | This file — Red-team audit section | Fresh red-team |
