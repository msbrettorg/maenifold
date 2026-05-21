#!/usr/bin/env bash
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/test_helpers.sh"

test_checkpoint_response_shape_matching_title_skips_write() {
  local sandbox
  local writes
  sandbox="$(setup_sandbox checkpoint-shape-match)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  export CURL_STUB_LOG="$sandbox/curl.log"
  export CURL_STUB_STATE="$sandbox/curl-state.json"
  export CURL_STUB_SEARCH_TEXT_FILE="$FIXTURES_DIR/daemon/searchmemories-real-match-with-legacy-miss.json"
  run_hook "pre_compact" "$FIXTURES_DIR/payloads/pre_compact.json.tmpl" "$sandbox"
  writes="$(count_write_memory_calls "$CURL_STUB_LOG")"
  assert_status 0 "$RUN_STATUS" &&
    [[ "$writes" == "0" ]]
}

test_checkpoint_response_shape_title_mismatch_writes() {
  local sandbox
  local writes
  sandbox="$(setup_sandbox checkpoint-shape-mismatch)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  export CURL_STUB_LOG="$sandbox/curl.log"
  export CURL_STUB_STATE="$sandbox/curl-state.json"
  export CURL_STUB_SEARCH_TEXT_FILE="$FIXTURES_DIR/daemon/searchmemories-real-mismatch-with-legacy-match.json"
  run_hook "pre_compact" "$FIXTURES_DIR/payloads/pre_compact.json.tmpl" "$sandbox"
  writes="$(count_write_memory_calls "$CURL_STUB_LOG")"
  assert_status 0 "$RUN_STATUS" &&
    [[ "$writes" == "1" ]]
}

test_checkpoint_response_shape_empty_results_writes() {
  local sandbox
  local writes
  sandbox="$(setup_sandbox checkpoint-shape-empty)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  export CURL_STUB_LOG="$sandbox/curl.log"
  export CURL_STUB_STATE="$sandbox/curl-state.json"
  export CURL_STUB_SEARCH_TEXT_FILE="$FIXTURES_DIR/daemon/searchmemories-real-empty-with-legacy-match.json"
  run_hook "pre_compact" "$FIXTURES_DIR/payloads/pre_compact.json.tmpl" "$sandbox"
  writes="$(count_write_memory_calls "$CURL_STUB_LOG")"
  assert_status 0 "$RUN_STATUS" &&
    [[ "$writes" == "1" ]]
}

run_case "checkpoint response shape skips write when real title matches" test_checkpoint_response_shape_matching_title_skips_write
run_case "checkpoint response shape writes when real title mismatches" test_checkpoint_response_shape_title_mismatch_writes
run_case "checkpoint response shape writes when real results are empty" test_checkpoint_response_shape_empty_results_writes
finish_tests
