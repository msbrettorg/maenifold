#!/usr/bin/env bash
# Runner: plain bash (selected because `bats --version` returned `command not found` on 2026-05-21).
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/test_helpers.sh"

test_pre_compact_is_notification_safe() {
  local sandbox
  sandbox="$(setup_sandbox pre-compact)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  run_hook "pre_compact" "$FIXTURES_DIR/payloads/pre_compact.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_output_is_empty_or_json "$RUN_STDOUT" &&
    { [[ -z "$RUN_STDOUT" ]] || assert_valid_json "$RUN_STDOUT"; }
}

run_case "pre_compact exits cleanly with empty or JSON output" test_pre_compact_is_notification_safe
finish_tests
