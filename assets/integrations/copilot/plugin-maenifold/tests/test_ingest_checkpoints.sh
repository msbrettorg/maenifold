#!/usr/bin/env bash
# Runner: plain bash (selected because `bats --version` returned `command not found` on 2026-05-21).
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/test_helpers.sh"

test_ingest_checkpoints_is_idempotent_via_pre_compact() {
  local sandbox
  local session_state
  local first_writes
  local second_writes
  sandbox="$(setup_sandbox ingest-checkpoints)"
  session_state="$(copy_session_state_fixture "$sandbox")"
  COPILOT_SESSION_STATE="$session_state"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  export CURL_STUB_LOG="$sandbox/curl.log"
  export CURL_STUB_STATE="$sandbox/curl-state.json"

  run_hook "pre_compact" "$FIXTURES_DIR/payloads/pre_compact.json.tmpl" "$sandbox"
  first_writes="$(count_write_memory_calls "$CURL_STUB_LOG")"
  run_hook "pre_compact" "$FIXTURES_DIR/payloads/pre_compact.json.tmpl" "$sandbox"
  second_writes="$(count_write_memory_calls "$CURL_STUB_LOG")"

  assert_status 0 "$RUN_STATUS" &&
    [[ "$first_writes" -ge 1 ]] &&
    [[ "$second_writes" == "$first_writes" ]]
}

run_case "ingest_checkpoints writes once across repeated runs" test_ingest_checkpoints_is_idempotent_via_pre_compact
finish_tests
