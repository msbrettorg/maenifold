#!/usr/bin/env bash
# Runner: plain bash (selected because `bats --version` returned `command not found` on 2026-05-21).
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/test_helpers.sh"

test_session_start_emits_valid_json() {
  local sandbox
  sandbox="$(setup_sandbox session-start-json)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  run_hook "session_start" "$FIXTURES_DIR/payloads/session_start.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_non_empty "$RUN_STDOUT" &&
    assert_valid_json "$RUN_STDOUT" &&
    assert_json_field_contains "$RUN_STDOUT" "additionalContext" "maenifold"
}

test_session_start_graceful_when_daemon_down() {
  local sandbox
  sandbox="$(setup_sandbox session-start-daemon-down)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=7
  run_hook "session_start" "$FIXTURES_DIR/payloads/session_start.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_output_is_empty_or_json "$RUN_STDOUT" &&
    { [[ -z "$RUN_STDOUT" ]] || assert_json_field_empty_or_missing "$RUN_STDOUT" "additionalContext"; }
}

run_case "session_start returns valid JSON with additionalContext" test_session_start_emits_valid_json
run_case "session_start fails open when daemon is unreachable" test_session_start_graceful_when_daemon_down
finish_tests
