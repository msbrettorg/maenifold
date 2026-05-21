#!/usr/bin/env bash
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/test_helpers.sh"

test_confession_structural_false_positive_inline_prose_blocks() {
  local sandbox
  sandbox="$(setup_sandbox confession-structural-inline-prose)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  run_hook "subagent_stop" "$FIXTURES_DIR/payloads/subagent_stop_confession_inline_prose.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_non_empty "$RUN_STDOUT" &&
    assert_valid_json "$RUN_STDOUT" &&
    assert_json_field_equals "$RUN_STDOUT" "decision" "block"
}

test_confession_structural_true_positive_headings_and_wikilink_allows() {
  local sandbox
  sandbox="$(setup_sandbox confession-structural-true-positive)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  run_hook "subagent_stop" "$FIXTURES_DIR/payloads/subagent_stop_confession_headings_allow.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_empty "$RUN_STDOUT"
}

test_confession_structural_missing_heading_blocks() {
  local sandbox
  sandbox="$(setup_sandbox confession-structural-missing-heading)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  run_hook "subagent_stop" "$FIXTURES_DIR/payloads/subagent_stop_confession_missing_gaps_heading.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_non_empty "$RUN_STDOUT" &&
    assert_valid_json "$RUN_STDOUT" &&
    assert_json_field_equals "$RUN_STDOUT" "decision" "block"
}

test_confession_structural_missing_wikilinks_reason_is_preserved() {
  local sandbox
  sandbox="$(setup_sandbox confession-structural-missing-wikilinks)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  run_hook "subagent_stop" "$FIXTURES_DIR/payloads/subagent_stop_confession_headings_no_links.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_non_empty "$RUN_STDOUT" &&
    assert_valid_json "$RUN_STDOUT" &&
    assert_json_field_equals "$RUN_STDOUT" "decision" "block" &&
    assert_json_field_contains "$RUN_STDOUT" "reason" "missing durable [[WikiLinks]]"
}

run_case "confession structural rejects inline prose keyword soup" test_confession_structural_false_positive_inline_prose_blocks
run_case "confession structural allows markdown headings plus wikilink" test_confession_structural_true_positive_headings_and_wikilink_allows
run_case "confession structural rejects transcript missing one heading" test_confession_structural_missing_heading_blocks
run_case "confession structural preserves missing wikilinks reason" test_confession_structural_missing_wikilinks_reason_is_preserved
finish_tests
