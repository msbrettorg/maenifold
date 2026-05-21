#!/usr/bin/env bash
# Runner: plain bash (selected because `bats --version` returned `command not found` on 2026-05-21).
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/test_helpers.sh"

test_agent_stop_blocks_without_wikilinks_and_confession() {
  local sandbox
  sandbox="$(setup_sandbox agent-stop-block)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  run_hook "agent_stop" "$FIXTURES_DIR/payloads/agent_stop_block.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_non_empty "$RUN_STDOUT" &&
    assert_valid_json "$RUN_STDOUT" &&
    assert_json_field_equals "$RUN_STDOUT" "decision" "block"
}

test_agent_stop_allows_when_wikilinks_and_confession_present() {
  local sandbox
  sandbox="$(setup_sandbox agent-stop-allow)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  run_hook "agent_stop" "$FIXTURES_DIR/payloads/agent_stop_allow.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_empty "$RUN_STDOUT"
}

test_agent_stop_blocks_when_confession_exists_but_wikilinks_do_not() {
  local sandbox
  sandbox="$(setup_sandbox agent-stop-missing-links)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  run_hook "agent_stop" "$FIXTURES_DIR/payloads/agent_stop_missing_wikilinks.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_non_empty "$RUN_STDOUT" &&
    assert_valid_json "$RUN_STDOUT" &&
    assert_json_field_equals "$RUN_STDOUT" "decision" "block"
}

run_case "agent_stop blocks on missing WikiLinks and ConfessionReport" test_agent_stop_blocks_without_wikilinks_and_confession
run_case "agent_stop allows compliant transcript" test_agent_stop_allows_when_wikilinks_and_confession_present
run_case "agent_stop still blocks when WikiLinks are missing" test_agent_stop_blocks_when_confession_exists_but_wikilinks_do_not
finish_tests
