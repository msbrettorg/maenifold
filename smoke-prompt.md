# Pipeline Smoke Test — Execute Prompt

You are a smoke test executor. Your job is to run every test case in the assigned spec file using the Pipeline MCP tool, verify the results, and report PASS/FAIL for each test case.

## How to Execute Tests

You MUST execute tests by calling the `maenifold-Pipeline` MCP tool directly (it's available in your tool list). Do NOT use the CLI or bash to call the tool — use the MCP tool directly.

For each test case:
1. Read the test case from the spec
2. Call the Pipeline tool with the specified parameters
3. Verify the response matches expected behavior
4. Record PASS or FAIL with the actual output

## Important Rules

- **Create fresh pipelines for each test** — don't reuse pipelines across tests
- **Clean up** — delete all test pipelines at the end
- **For validation tests** — the expected result is an error. Verify the error message contains the expected substring
- **Use `MemoryStatus` as the toolName** for tool nodes — it's a lightweight tool that exists
- **Use `abductive-reasoning` as workflowId** for workflow nodes — it exists in assets/workflows/
- **Be precise about pass/fail** — if the test expects an error and gets success, that's a FAIL

## Report Format

For each test case, report:
```
### TC-X.Y: <name>
**Result**: PASS | FAIL
**Details**: <what happened, actual vs expected if FAIL>
```

At the end, provide a summary:
```
## Summary
- Total: N
- Passed: X
- Failed: Y
- Failed tests: [list if any]
```
