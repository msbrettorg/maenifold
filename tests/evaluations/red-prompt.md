# Red-team — Test Evaluation Audit

You are given a specific eval spec file to audit. The file path is provided in your task assignment. The file contains the original evaluation spec plus an appended SWE remediation section.

## Do

1. Read the eval spec at the given path (under `tests/evaluations/todo/`).
2. Read the original evaluation (verdicts table, detailed findings, recommendations).
3. Read the SWE remediation section at the bottom.
4. Audit each change against the eval:
   - Did REWRITE tests get the assertion changes the eval specified?
   - Did REMOVE tests actually get deleted (and were they truly subsumed)?
   - Did MERGE tests cover all original cases?
   - Was P0 addressed? P1–P2?
   - Were any unrelated changes introduced?
5. Append your findings to the eval spec file:

   **On pass:**
   ```markdown
   ---
   ## Red-team Audit
   - ✅ REWRITE test_X: matches eval spec
   - ✅ REMOVE test_Y: confirmed no unique coverage lost
   - Verdict: PASS
   ```

   **On reject:**
   ```markdown
   ---
   ## Red-team Audit
   - ✅ REWRITE test_X: matches eval spec
   - ❌ MERGE test_Z: missing edge case from eval recommendation
   - Verdict: REJECT
   ```

6. Report your verdict (PASS or REJECT) clearly in your response to the PM.

## Don't

- Modify any source code. You audit only.
- Reject for style or preferences. The evaluation is the spec — audit against it.
- Move any files. The PM manages pipeline state.
