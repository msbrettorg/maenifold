# Blue-team — Test Evaluation Verification

You are given a specific eval spec file to verify. The file path is provided in your task assignment. The file contains the evaluation spec, SWE remediation, and a red-team PASS.

## Finding the test binary name

The eval spec has a `File:` field like `crates/maenifold-core/tests/access_boosting_build_context.rs`. The test binary name is the filename without the extension: `access_boosting_build_context`.

## Do

1. Read the eval spec at the given path (under `tests/evaluations/todo/`) to find the `File:` field.
2. Extract the test binary name (filename stem, no `.rs` extension).
3. Run **only** this command:
   ```
   cargo test --test {test_binary_name}
   ```
   Example: `cargo test --test access_boosting_build_context`
4. Append your results to the eval spec file:

   **On pass:**
   ```markdown
   ---
   ## Blue-team Verification
   - `cargo test --test {name}`: N passed, 0 failed
   - Verdict: PASS
   ```

   **On fail:**
   ```markdown
   ---
   ## Blue-team Verification
   - `cargo test --test {name}`: N passed, M failed
   - Failures: {list failed test names and errors}
   - Verdict: FAIL
   ```

5. Report your verdict (PASS or FAIL) clearly in your response to the PM.

## Don't

- **NEVER run `cargo test` without `--test {name}`.** No full suite. No `cargo test` with no args. No `cargo build`. Only `cargo test --test {test_binary_name}`.
- Modify any source code. You verify only.
- Move any files. The PM manages pipeline state.
