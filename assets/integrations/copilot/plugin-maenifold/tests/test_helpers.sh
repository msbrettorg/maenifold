#!/usr/bin/env bash
set -euo pipefail

TEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_DIR="$(cd "$TEST_DIR/.." && pwd)"
HOOK_SCRIPT="$PLUGIN_DIR/scripts/hooks.sh"
FIXTURES_DIR="$TEST_DIR/fixtures"
SANDBOX_ROOT="$TEST_DIR/.sandbox"
TEST_CASES_TOTAL=0
TEST_CASES_FAILED=0

mkdir -p "$SANDBOX_ROOT"

log() {
  printf '%s\n' "$*"
}

fail_assert() {
  log "ASSERTION FAILED: $*"
  return 1
}

run_case() {
  local name="$1"
  shift
  TEST_CASES_TOTAL=$((TEST_CASES_TOTAL + 1))
  if "$@"; then
    log "ok - $name"
  else
    TEST_CASES_FAILED=$((TEST_CASES_FAILED + 1))
    log "not ok - $name"
  fi
}

finish_tests() {
  local passed=$((TEST_CASES_TOTAL - TEST_CASES_FAILED))
  log "SUMMARY total=$TEST_CASES_TOTAL passed=$passed failed=$TEST_CASES_FAILED"
  [[ $TEST_CASES_FAILED -eq 0 ]]
}

setup_sandbox() {
  local name="$1"
  local dir="$SANDBOX_ROOT/$name"
  rm -rf "$dir"
  mkdir -p "$dir/bin" "$dir/home/maenifold/auth"
  printf 'stub-token\n' > "$dir/home/maenifold/auth/localhost.token"
  printf '%s' "$dir"
}

copy_session_state_fixture() {
  local sandbox="$1"
  cp -R "$FIXTURES_DIR/session-state" "$sandbox/session-state"
  printf '%s' "$sandbox/session-state"
}

render_fixture() {
  local template="$1"
  local destination="$2"
  python3 -c 'import pathlib,sys
src = pathlib.Path(sys.argv[1]).read_text()
src = src.replace("__FIXTURE_ROOT__", sys.argv[2]).replace("__PLUGIN_ROOT__", sys.argv[3])
pathlib.Path(sys.argv[4]).write_text(src)
' "$template" "$FIXTURES_DIR" "$PLUGIN_DIR" "$destination"
}

run_hook() {
  local mode="$1"
  local template="$2"
  local sandbox="$3"
  local stdin_file="$sandbox/stdin.json"
  render_fixture "$template" "$stdin_file"
  : > "$sandbox/stdout"
  : > "$sandbox/stderr"
  set +e
  (
    cd "$PLUGIN_DIR"
    PATH="$TEST_DIR/bin:$PATH" \
    HOME="$sandbox/home" \
    COPILOT_SESSION_STATE="${COPILOT_SESSION_STATE:-}" \
    CURL_STUB_LOG="${CURL_STUB_LOG:-$sandbox/curl.log}" \
    CURL_STUB_STATE="${CURL_STUB_STATE:-$sandbox/curl-state.json}" \
    CURL_STUB_EXIT_CODE="${CURL_STUB_EXIT_CODE:-0}" \
    "$HOOK_SCRIPT" "$mode" < "$stdin_file" > "$sandbox/stdout" 2> "$sandbox/stderr"
  )
  RUN_STATUS=$?
  set -e
  RUN_STDOUT="$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).read_text(), end="")' "$sandbox/stdout")"
  RUN_STDERR="$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).read_text(), end="")' "$sandbox/stderr")"
}

assert_status() {
  local expected="$1"
  local actual="$2"
  [[ "$actual" == "$expected" ]]
}

assert_empty() {
  [[ -z "${1:-}" ]]
}

assert_non_empty() {
  [[ -n "${1:-}" ]]
}

assert_valid_json() {
  python3 -c 'import json,sys; json.loads(sys.argv[1])' "${1:-}" >/dev/null
}

assert_json_field_equals() {
  python3 -c 'import json,sys
obj = json.loads(sys.argv[1])
value = obj.get(sys.argv[2])
sys.exit(0 if value == sys.argv[3] else 1)
' "${1:-}" "$2" "$3"
}

assert_json_field_contains() {
  python3 -c 'import json,sys
obj = json.loads(sys.argv[1])
value = obj.get(sys.argv[2], "")
sys.exit(0 if sys.argv[3] in value else 1)
' "${1:-}" "$2" "$3"
}

assert_json_field_empty_or_missing() {
  python3 -c 'import json,sys
obj = json.loads(sys.argv[1])
value = obj.get(sys.argv[2], "")
sys.exit(0 if value in ("", None, [], {}) else 1)
' "${1:-}" "$2"
}

assert_output_is_empty_or_json() {
  local value="${1:-}"
  if [[ -z "$value" ]]; then
    return 0
  fi
  assert_valid_json "$value"
}

assert_file_matches_multiline_regex() {
  python3 -c 'import pathlib,re,sys
text = pathlib.Path(sys.argv[1]).read_text()
sys.exit(0 if re.search(sys.argv[2], text, re.S) else 1)
' "$1" "$2"
}

count_write_memory_calls() {
  local log_file="$1"
  if [[ ! -f "$log_file" ]]; then
    echo 0
    return 0
  fi
  grep -c 'WriteMemory' "$log_file" || true
}

write_memory_titles() {
  python3 -c 'import json,pathlib,sys
path = pathlib.Path(sys.argv[1])
if not path.exists():
    raise SystemExit(0)
for line in path.read_text().splitlines():
    line = line.strip()
    if not line:
        continue
    payload = json.loads(line)
    params = payload.get("params", {})
    title = None
    if params.get("name") == "WriteMemory":
        title = params.get("arguments", {}).get("title")
    elif params.get("name") == "GatewayExecute" and params.get("arguments", {}).get("command") == "WriteMemory":
        title = params.get("arguments", {}).get("parameters", {}).get("title")
    if title is not None:
        print(title)
' "$1"
}
