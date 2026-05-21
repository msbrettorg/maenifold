#!/usr/bin/env bash
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/test_helpers.sh"

resource_call_count() {
  python3 -c 'import json,pathlib,sys
path = pathlib.Path(sys.argv[1])
uri = sys.argv[2]
count = 0
if path.exists():
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            payload = json.loads(line)
        except Exception:
            continue
        if payload.get("params", {}).get("uri") == uri:
            count += 1
print(count)
' "$1" "$2"
}

assert_resource_call_count() {
  [[ "$(resource_call_count "$1" "$2")" == "$3" ]]
}

assert_json_field_not_contains() {
  python3 -c 'import json,sys
obj = json.loads(sys.argv[1])
value = obj.get(sys.argv[2], "")
sys.exit(0 if sys.argv[3] not in value else 1)
' "${1:-}" "$2" "$3"
}

test_session_start_uses_primer_without_snapshot_or_stats() {
  local sandbox
  sandbox="$(setup_sandbox session-start-primer-success)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  export CURL_STUB_LOG="$sandbox/curl.log"
  export CURL_STUB_RESOURCE_MAP_FILE="$FIXTURES_DIR/daemon/session_start_primer_success.json"
  run_hook "session_start" "$FIXTURES_DIR/payloads/session_start.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_non_empty "$RUN_STDOUT" &&
    assert_valid_json "$RUN_STDOUT" &&
    assert_json_field_contains "$RUN_STDOUT" "additionalContext" "<communities>" &&
    assert_json_field_contains "$RUN_STDOUT" "additionalContext" "**[[concept-a]]** [[concept-b]]" &&
    assert_json_field_contains "$RUN_STDOUT" "additionalContext" "**[[concept-c]]**" &&
    assert_resource_call_count "$CURL_STUB_LOG" "graph://corpus/primer" "1" &&
    assert_resource_call_count "$CURL_STUB_LOG" "graph://corpus/snapshot" "0" &&
    assert_resource_call_count "$CURL_STUB_LOG" "graph://stats" "0"
}

test_session_start_no_cascade_when_primer_fails() {
  local sandbox
  sandbox="$(setup_sandbox session-start-primer-no-cascade)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  export CURL_STUB_LOG="$sandbox/curl.log"
  export CURL_STUB_RESOURCE_MAP_FILE="$FIXTURES_DIR/daemon/session_start_snapshot_fallback.json"
  run_hook "session_start" "$FIXTURES_DIR/payloads/session_start.json.tmpl" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_resource_call_count "$CURL_STUB_LOG" "graph://corpus/primer" "1" &&
    assert_resource_call_count "$CURL_STUB_LOG" "graph://corpus/snapshot" "0" &&
    assert_resource_call_count "$CURL_STUB_LOG" "graph://stats" "0"
}

run_case "session_start uses corpus primer before any fallback" test_session_start_uses_primer_without_snapshot_or_stats
run_case "session_start does not cascade to snapshot or stats when primer fails" test_session_start_no_cascade_when_primer_fails
finish_tests
