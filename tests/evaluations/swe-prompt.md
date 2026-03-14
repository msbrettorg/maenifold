# SWE — Test Evaluation Remediation

You are given a specific eval spec file to process. The file path is provided in your task assignment.

## Finding the test binary name

The eval spec has a `File:` field like `crates/maenifold-core/tests/access_boosting_build_context.rs`. The test binary name is the filename without the extension: `access_boosting_build_context`.

## Do

1. Read the eval spec at the given path (under `tests/evaluations/todo/`).
2. Note the source test file, verdicts table, and recommendations.
3. If prior red-team/blue-team rejection sections exist at the bottom, fix those issues first.
4. Implement the verdicts:
   - **KEEP**: don't touch it.
   - **REWRITE**: rewrite the test per the detailed findings.
   - **REMOVE**: delete the test function.
   - **MERGE**: collapse identified tests into one parameterized test, delete originals.
5. Apply recommendations: P0 mandatory, P1–P2 should-do, P3+ only if small.
6. Verify your changes compile and pass with **only** this command:
   ```
   cargo test --test {test_binary_name}
   ```
   Example: `cargo test --test access_boosting_build_context`
7. Append your results to the eval spec file:
   ```markdown
   ---
   ## SWE Remediation
   - REWRITE test_X: strengthened assertions per P0
   - REMOVE test_Y: confirmed subsumed
   - Commit: {sha}
   ```
8. Commit your code changes with a message referencing the eval file name.

## Don't

- **NEVER run `cargo test` without `--test {name}`.** No full suite. No `cargo test` with no args. No `cargo build`. Only `cargo test --test {test_binary_name}`.
- Change tests marked KEEP.
- Add work beyond what the evaluation recommends.
- Move any files. The PM manages pipeline state.
