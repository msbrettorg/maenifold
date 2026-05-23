#!/usr/bin/env bash
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/test_helpers.sh"

write_targeted_gethelp_stub() {
  local sandbox="$1"
  python3 -c 'import pathlib, sys
path = pathlib.Path(sys.argv[1])
path.write_text("""#!/usr/bin/env python3
import json
import sys

payload = ""
for index, arg in enumerate(sys.argv[1:]):
    if arg in ("-d", "--data", "--data-raw", "--data-binary") and index + 2 <= len(sys.argv[1:]):
        payload = sys.argv[1:][index + 1]
        break

try:
    request = json.loads(payload)
except Exception:
    request = {}

arguments = request.get("params", {}).get("arguments", {})
target = arguments.get("toolName", "")
learn = arguments.get("learn", False)

if request.get("params", {}).get("name") == "GetHelp" and target == "GetConfig":
    header = "# GetHelp" if learn else "# GetConfig"
    response_text = json.dumps({"success": True, "data": header + "\\n\\nStub help body", "error": None})
else:
    response_text = json.dumps({"success": True, "data": "stub response", "error": None})

sys.stdout.write(json.dumps({
    "jsonrpc": "2.0",
    "id": 1,
    "result": {"content": [{"type": "text", "text": response_text}]}
}))
""")
' "$sandbox/bin/curl"
  chmod +x "$sandbox/bin/curl"
}

run_hook_with_targeted_gethelp_stub() {
  local mode="$1"
  local sandbox="$2"
  local stdin_file="$sandbox/stdin.json"
  python3 -c 'import json, pathlib, sys
pathlib.Path(sys.argv[1]).write_text(json.dumps({
    "sessionId": "session-alpha",
    "timestamp": 1763681400009,
    "cwd": sys.argv[2],
    "toolName": "GetConfig",
    "toolArgs": {},
    "error": "tool execution failed"
}))
' "$stdin_file" "$PLUGIN_DIR"
  : > "$sandbox/stdout"
  : > "$sandbox/stderr"
  set +e
  (
    cd "$PLUGIN_DIR"
    PATH="$sandbox/bin:$TEST_DIR/bin:$PATH" \
    HOME="$sandbox/home" \
    COPILOT_SESSION_STATE="${COPILOT_SESSION_STATE:-}" \
    CURL_STUB_EXIT_CODE="${CURL_STUB_EXIT_CODE:-0}" \
    "$HOOK_SCRIPT" "$mode" < "$stdin_file" > "$sandbox/stdout" 2> "$sandbox/stderr"
  )
  RUN_STATUS=$?
  set -e
  RUN_STDOUT="$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).read_text(), end="")' "$sandbox/stdout")"
  RUN_STDERR="$(python3 -c 'import pathlib,sys; print(pathlib.Path(sys.argv[1]).read_text(), end="")' "$sandbox/stderr")"
}

assert_json_field_first_line_equals() {
  python3 -c 'import json, sys
obj = json.loads(sys.argv[1])
value = obj.get(sys.argv[2], "")
first_line = value.splitlines()[0] if value else ""
sys.exit(0 if first_line == sys.argv[3] else 1)
' "${1:-}" "$2" "$3"
}

test_tool_failure_gethelp_uses_target_tool_header() {
  local sandbox
  sandbox="$(setup_sandbox tool-failure-getconfig-header)"
  COPILOT_SESSION_STATE="$(copy_session_state_fixture "$sandbox")"
  export COPILOT_SESSION_STATE
  export CURL_STUB_EXIT_CODE=0
  write_targeted_gethelp_stub "$sandbox"
  run_hook_with_targeted_gethelp_stub "tool_failure" "$sandbox"
  assert_status 0 "$RUN_STATUS" &&
    assert_non_empty "$RUN_STDOUT" &&
    assert_valid_json "$RUN_STDOUT" &&
    assert_json_field_first_line_equals "$RUN_STDOUT" "additionalContext" "# GetConfig" &&
    ! assert_json_field_contains "$RUN_STDOUT" "additionalContext" "# GetHelp"
}

run_case "tool_failure uses target tool header for GetHelp" test_tool_failure_gethelp_uses_target_tool_header
finish_tests
