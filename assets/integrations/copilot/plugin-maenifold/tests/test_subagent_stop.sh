#!/usr/bin/env bash
# Runner: plain bash (selected because `bats --version` returned `command not found` on 2026-05-21).
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/test_helpers.sh"

test_subagent_stop_blocks_without_wikilinks_and_confession() {
  local sandbox
  local writes
  sandbox="$(setup_sandbox subagent-stop-block)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  export CURL_STUB_LOG="$sandbox/curl.log"
  run_hook "subagent_stop" "$FIXTURES_DIR/payloads/subagent_stop_block.json.tmpl" "$sandbox"
  writes="$(count_write_memory_calls "$CURL_STUB_LOG")"
  assert_status 0 "$RUN_STATUS" &&
    assert_non_empty "$RUN_STDOUT" &&
    assert_valid_json "$RUN_STDOUT" &&
    assert_json_field_equals "$RUN_STDOUT" "decision" "block" &&
    [[ "$writes" -ge 1 ]]
}

test_subagent_stop_allows_when_wikilinks_and_confession_present() {
  local sandbox
  local writes
  sandbox="$(setup_sandbox subagent-stop-allow)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  export CURL_STUB_LOG="$sandbox/curl.log"
  run_hook "subagent_stop" "$FIXTURES_DIR/payloads/subagent_stop_allow.json.tmpl" "$sandbox"
  writes="$(count_write_memory_calls "$CURL_STUB_LOG")"
  assert_status 0 "$RUN_STATUS" &&
    assert_empty "$RUN_STDOUT" &&
    [[ "$writes" -ge 1 ]]
}

test_subagent_stop_blocks_when_confession_exists_but_wikilinks_do_not() {
  local sandbox
  sandbox="$(setup_sandbox subagent-stop-missing-links)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  export CURL_STUB_LOG="$sandbox/curl.log"
  run_hook "subagent_stop" "$FIXTURES_DIR/payloads/subagent_stop_missing_wikilinks.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_non_empty "$RUN_STDOUT" &&
    assert_valid_json "$RUN_STDOUT" &&
    assert_json_field_equals "$RUN_STDOUT" "decision" "block"
}

run_case "subagent_stop blocks on missing WikiLinks and ConfessionReport" test_subagent_stop_blocks_without_wikilinks_and_confession
run_case "subagent_stop allows compliant transcript" test_subagent_stop_allows_when_wikilinks_and_confession_present
run_case "subagent_stop still blocks when WikiLinks are missing" test_subagent_stop_blocks_when_confession_exists_but_wikilinks_do_not
finish_tests
