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

test_ingest_checkpoints_uses_deterministic_title_without_digest_for_same_content() {
  local sandbox
  local session_state
  local titles
  local first_title
  local second_title
  sandbox="$(setup_sandbox ingest-checkpoints-deterministic-same-content)"
  session_state="$(copy_session_state_fixture "$sandbox")"
  COPILOT_SESSION_STATE="$session_state"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  export CURL_STUB_LOG="$sandbox/curl.log"

  export CURL_STUB_STATE="$sandbox/curl-state-run-1.json"
  run_hook "pre_compact" "$FIXTURES_DIR/payloads/pre_compact.json.tmpl" "$sandbox"
  export CURL_STUB_STATE="$sandbox/curl-state-run-2.json"
  run_hook "pre_compact" "$FIXTURES_DIR/payloads/pre_compact.json.tmpl" "$sandbox"

  titles="$(write_memory_titles "$CURL_STUB_LOG")"
  first_title="$(printf '%s\n' "$titles" | sed -n '1p')"
  second_title="$(printf '%s\n' "$titles" | sed -n '2p')"

  if [[ -n "$first_title" && -n "$second_title" && "$first_title" == "$second_title" && ! "$first_title" =~ -[0-9a-f]{12}$ ]]; then
    return 0
  fi
  fail_assert "umbrella L377 requires deterministic sessionId+slug title with no digest/content.hash watermark; saw titles: ${first_title:-<missing>} | ${second_title:-<missing>}"
}

test_ingest_checkpoints_keeps_same_title_when_content_changes_for_same_slug() {
  local sandbox
  local session_state
  local checkpoint_file
  local titles
  local first_title
  local second_title
  sandbox="$(setup_sandbox ingest-checkpoints-deterministic-edited-content)"
  session_state="$(copy_session_state_fixture "$sandbox")"
  COPILOT_SESSION_STATE="$session_state"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  export CURL_STUB_LOG="$sandbox/curl.log"
  checkpoint_file="$session_state/session-alpha/checkpoints/0001-summary.md"

  export CURL_STUB_STATE="$sandbox/curl-state-run-1.json"
  run_hook "pre_compact" "$FIXTURES_DIR/payloads/pre_compact.json.tmpl" "$sandbox"

  python3 -c 'from pathlib import Path
path = Path(__import__("sys").argv[1])
path.write_text("# Compaction summary\n\nEdited [[maenifold]] [[copilot-cli]] [[wikilinks]] content should keep the same deterministic title.\n")
' "$checkpoint_file"

  export CURL_STUB_STATE="$sandbox/curl-state-run-2.json"
  run_hook "pre_compact" "$FIXTURES_DIR/payloads/pre_compact.json.tmpl" "$sandbox"

  titles="$(write_memory_titles "$CURL_STUB_LOG")"
  first_title="$(printf '%s\n' "$titles" | sed -n '1p')"
  second_title="$(printf '%s\n' "$titles" | sed -n '2p')"

  if [[ -n "$first_title" && -n "$second_title" && "$first_title" == "$second_title" ]]; then
    return 0
  fi
  fail_assert "umbrella L377 says checkpoint-sessionId-slug must stay deterministic across edited content; digest-driven title drift is a spec violation: ${first_title:-<missing>} -> ${second_title:-<missing>}"
}

run_case "ingest_checkpoints writes once across repeated runs" test_ingest_checkpoints_is_idempotent_via_pre_compact
run_case "ingest_checkpoints uses deterministic title without digest for same content" test_ingest_checkpoints_uses_deterministic_title_without_digest_for_same_content
run_case "ingest_checkpoints keeps same title when content changes for same slug" test_ingest_checkpoints_keeps_same_title_when_content_changes_for_same_slug
finish_tests
