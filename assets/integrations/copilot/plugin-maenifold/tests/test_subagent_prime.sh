#!/usr/bin/env bash
# Runner: plain bash (selected because `bats --version` returned `command not found` on 2026-05-21).
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/test_helpers.sh"

test_subagent_prime_emits_valid_json() {
  local sandbox
  sandbox="$(setup_sandbox subagent-prime)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  run_hook "subagent_prime" "$FIXTURES_DIR/payloads/subagent_start.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_non_empty "$RUN_STDOUT" &&
    assert_valid_json "$RUN_STDOUT" &&
    assert_json_field_contains "$RUN_STDOUT" "additionalContext" "[["
}

run_case "subagent_prime injects community context JSON" test_subagent_prime_emits_valid_json
finish_tests
