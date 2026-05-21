# BLUE_FINDINGS — T-303.50.1

## Hook-name verification

PASS. I fetched the canonical GitHub Copilot hooks reference at 2026-05-21T01:43:11Z and confirmed these event names verbatim:

- `sessionStart`
- `subagentStart`
- `preCompact`
- `subagentStop`
- `agentStop`
- `postToolUseFailure`

Source: https://docs.github.com/en/copilot/reference/hooks-reference

## Test runner choice

Used a plain bash harness because `bats --version` returned `command not found` on 2026-05-21T01:37:46Z.

## Current red-phase result

The red suite exists under `assets/integrations/copilot/plugin-maenifold/tests/` and currently fails against the unchanged production hook implementation.

- Failing files: 7 / 7
- Failing cases: 13 / 13
- Output artifact: `worktrees/T-303.50.1/BLUE_TEST_OUTPUT.txt`
