#!/usr/bin/env bash
# Runner: plain bash (selected because `bats --version` returned `command not found` on 2026-05-21).
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/test_helpers.sh"

test_tool_failure_known_tool_returns_help() {
  local sandbox
  sandbox="$(setup_sandbox tool-failure-known)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  run_hook "tool_failure" "$FIXTURES_DIR/payloads/tool_failure_known.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_non_empty "$RUN_STDOUT" &&
    assert_valid_json "$RUN_STDOUT" &&
    assert_json_field_contains "$RUN_STDOUT" "additionalContext" "BuildContext"
}

test_tool_failure_unknown_tool_noops() {
  local sandbox
  sandbox="$(setup_sandbox tool-failure-unknown)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  run_hook "tool_failure" "$FIXTURES_DIR/payloads/tool_failure_unknown.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_empty "$RUN_STDOUT"
}

run_case "tool_failure returns maenifold help for known tools" test_tool_failure_known_tool_returns_help
run_case "tool_failure noops for unknown tools" test_tool_failure_unknown_tool_noops
finish_tests
