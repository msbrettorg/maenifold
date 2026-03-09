#!/usr/bin/env bash
set -u
set -o pipefail

: <<'TRACEABILITY'
// T-1.6: RTM NFR-1
// T-1.6: RTM NFR-2
TRACEABILITY

CSHARP_BIN="/Users/brett/src/ma-collective/maenifold/src/bin/Debug/net9.0/maenifold"
RUST_BIN="/Users/brett/src/ma-collective/maenifold-rs/target/debug/maenifold"
CSHARP_REPO="/Users/brett/src/ma-collective/maenifold"
RUST_REPO="/Users/brett/src/ma-collective/maenifold-rs"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TMPROOT="$(mktemp -d "${TMPDIR:-/tmp}/maenifold-parity.XXXXXX")"
RESULTS_DIR="$TMPROOT/results"
CSHARP_SEED_ROOT="$TMPROOT/baseline/csharp"
RUST_SEED_ROOT="$TMPROOT/baseline/rust"
CSHARP_ROOT="$TMPROOT/csharp"
RUST_ROOT="$TMPROOT/rust"

TOTAL=0
PASS_EXACT=0
PASS_STRUCT=0
FAIL_COUNT=0
FAILURES=()
TEST_IDS=()
TEST_TOOLS=()
TEST_PAYLOADS=()
SELECTED_TESTS=("$@")

cleanup() {
  if [ -n "${TMPROOT:-}" ] && [ -d "$TMPROOT" ]; then
    rm -rf "$TMPROOT"
  fi
}
trap cleanup EXIT

mkdir -p "$RESULTS_DIR" "$TMPROOT/baseline"

require_prereqs() {
  if ! command -v jq >/dev/null 2>&1; then
    echo "ERROR: jq is required but was not found in PATH." >&2
    exit 1
  fi

  if [ ! -x "$CSHARP_BIN" ]; then
    echo "ERROR: C# binary is missing or not executable: $CSHARP_BIN" >&2
    exit 1
  fi

  if [ ! -x "$RUST_BIN" ]; then
    echo "ERROR: Rust binary is missing or not executable: $RUST_BIN" >&2
    exit 1
  fi
}

register_test() {
  local index=${#TEST_IDS[@]}
  TEST_IDS[$index]="$1"
  TEST_TOOLS[$index]="$2"
  TEST_PAYLOADS[$index]="$3"
}

should_run_test() {
  local test_id="$1"
  local selected

  if [ ${#SELECTED_TESTS[@]} -eq 0 ]; then
    return 0
  fi

  for selected in "${SELECTED_TESTS[@]}"; do
    if [ "$selected" = "$test_id" ]; then
      return 0
    fi
  done

  return 1
}

copy_tree() {
  local source_dir="$1"
  local target_dir="$2"

  rm -rf "$target_dir"
  mkdir -p "$target_dir"
  cp -R "$source_dir/." "$target_dir/"
}

python_cmd() {
  if command -v python3 >/dev/null 2>&1; then
    echo "python3"
  else
    echo "python"
  fi
}

run_cli_command() {
  local binary="$1"
  local root="$2"
  local tool_name="$3"
  local payload_json="$4"
  local stdout_file="$5"
  local stderr_file="$6"
  local status=0

  mkdir -p "$root"

  MAENIFOLD_ROOT="$root" \
  MAENIFOLD_DATABASE_PATH="$root/memory.db" \
    "$binary" --tool "$tool_name" --payload "$payload_json" --json \
    >"$stdout_file" 2>"$stderr_file" || status=$?

  echo "$status"
}

seed_write_memory() {
  local binary="$1"
  local root="$2"
  local payload_json="$3"
  local label="$4"
  local stdout_file="$RESULTS_DIR/setup.${label}.stdout.json"
  local stderr_file="$RESULTS_DIR/setup.${label}.stderr.log"
  local exit_code

  exit_code=$(run_cli_command "$binary" "$root" "WriteMemory" "$payload_json" "$stdout_file" "$stderr_file")
  if [ "$exit_code" -ne 0 ]; then
    echo "ERROR: failed seeding $label with exit code $exit_code" >&2
    cat "$stderr_file" >&2
    exit 1
  fi

  if ! jq -e '.success == true' "$stdout_file" >/dev/null 2>&1; then
    echo "ERROR: seeding $label did not return success JSON" >&2
    cat "$stdout_file" >&2
    exit 1
  fi
}

setup_test_data() {
  local seed_payloads
  local payload
  local index=1

  mkdir -p "$CSHARP_SEED_ROOT" "$RUST_SEED_ROOT"

  seed_payloads=$(cat <<'SEEDS'
{"title":"Parity Seed 1","content":"Seed [[parity]] data for [[cli-tests]]"}
{"title":"Parity Seed 2","content":"Baseline [[memory]] and [[graph]] content for parity"}
{"title":"Parity Seed 3","content":"Session [[workflow]] and [[assumptions]] parity seed"}
SEEDS
)

  while IFS= read -r payload; do
    [ -z "$payload" ] && continue
    seed_write_memory "$CSHARP_BIN" "$CSHARP_SEED_ROOT" "$payload" "csharp.$index"
    seed_write_memory "$RUST_BIN" "$RUST_SEED_ROOT" "$payload" "rust.$index"
    index=$((index + 1))
  done <<EOF_SEEDS
$seed_payloads
EOF_SEEDS
}

reset_test_roots() {
  copy_tree "$CSHARP_SEED_ROOT" "$CSHARP_ROOT"
  copy_tree "$RUST_SEED_ROOT" "$RUST_ROOT"
}

normalize_json() {
  local input_file="$1"
  local output_file="$2"

  jq -S \
    --arg tmproot "$TMPROOT" \
    --arg csharp_root "$CSHARP_ROOT" \
    --arg rust_root "$RUST_ROOT" \
    --arg csharp_bin "$CSHARP_BIN" \
    --arg rust_bin "$RUST_BIN" \
    --arg csharp_repo "$CSHARP_REPO" \
    --arg rust_repo "$RUST_REPO" '
      def scrub_string:
        split($csharp_root) | join("<ROOT>")
        | split($rust_root) | join("<ROOT>")
        | split($tmproot) | join("<ROOT>")
        | split($csharp_bin) | join("<ROOT>")
        | split($rust_bin) | join("<ROOT>")
        | split($csharp_repo) | join("<ROOT>")
        | split($rust_repo) | join("<ROOT>");

      def normalize:
        if type == "object" then
          to_entries
          | map(select(.key != "created" and .key != "modified" and .key != "last_indexed" and .key != "checksum"))
          | map(.value |= normalize)
          | sort_by(.key)
          | from_entries
        elif type == "array" then
          map(normalize) | sort_by(tojson)
        elif type == "string" then
          scrub_string
        else
          .
        end;

      normalize
    ' "$input_file" >"$output_file"
}

normalize_text() {
  local input_file="$1"
  local output_file="$2"

  "$(python_cmd)" - "$input_file" "$output_file" \
    "$TMPROOT" "$CSHARP_ROOT" "$RUST_ROOT" "$CSHARP_BIN" "$RUST_BIN" "$CSHARP_REPO" "$RUST_REPO" <<'PY'
import pathlib
import re
import sys

input_path = pathlib.Path(sys.argv[1])
output_path = pathlib.Path(sys.argv[2])
replacements = [value for value in sys.argv[3:] if value]

text = input_path.read_text(encoding="utf-8")
for value in replacements:
    text = text.replace(value, "<ROOT>")

text = re.sub(r"\b[0-9a-fA-F]{12,}\b", "<HEX>", text)
text = re.sub(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|\+00:00)?", "<ISO>", text)
text = re.sub(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} UTC", "<UTC>", text)
output_path.write_text(text, encoding="utf-8")
PY
}

extract_data_structure() {
  local input_file="$1"
  local output_file="$2"

  jq -S '
    def structure:
      if type == "object" then
        to_entries
        | sort_by(.key)
        | map({key: .key, value: (.value | structure)})
        | from_entries
      elif type == "array" then
        {
          type: "array",
          length: length,
          items: (map(structure) | sort_by(tojson) | unique)
        }
      elif type == "null" then
        "null"
      else
        type
      end;

    (.data // null) | structure
  ' "$input_file" >"$output_file"
}

extract_json_structure() {
  local input_file="$1"
  local output_file="$2"

  jq -S '
    def structure:
      if type == "object" then
        to_entries
        | sort_by(.key)
        | map({key: .key, value: (.value | structure)})
        | from_entries
      elif type == "array" then
        {
          type: "array",
          length: length,
          items: (map(structure) | sort_by(tojson) | unique)
        }
      elif type == "null" then
        "null"
      else
        type
      end;

    structure
  ' "$input_file" >"$output_file"
}

extract_error_structure() {
  local input_file="$1"
  local output_file="$2"

  jq -S '{
      success: (.success // null),
      errorCode: (.error.code // null),
      topLevelKeys: (keys | sort),
      errorKeys: ((.error // {}) | keys | sort)
    }' "$input_file" >"$output_file"
}

record_failure() {
  FAILURES[${#FAILURES[@]}]="$1: $2"
}

compare_results() {
  local test_id="$1"
  local csharp_file="$2"
  local rust_file="$3"
  local csharp_exit="$4"
  local rust_exit="$5"
  local csharp_normalized="$RESULTS_DIR/$test_id.csharp.normalized.json"
  local rust_normalized="$RESULTS_DIR/$test_id.rust.normalized.json"
  local csharp_structure="$RESULTS_DIR/$test_id.csharp.structure.json"
  local rust_structure="$RESULTS_DIR/$test_id.rust.structure.json"
  local csharp_text="$RESULTS_DIR/$test_id.csharp.normalized.txt"
  local rust_text="$RESULTS_DIR/$test_id.rust.normalized.txt"
  local csharp_success
  local rust_success
  local csharp_error_code
  local rust_error_code
  local csharp_type
  local rust_type
  local csharp_has_success
  local rust_has_success

  TOTAL=$((TOTAL + 1))

  if [ "$csharp_exit" != "$rust_exit" ]; then
    echo "FAIL $test_id (exit code mismatch: C#=$csharp_exit Rust=$rust_exit)"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    record_failure "$test_id" "exit code mismatch (C#=$csharp_exit Rust=$rust_exit)"
    return
  fi

  if ! jq empty "$csharp_file" >/dev/null 2>&1 || ! jq empty "$rust_file" >/dev/null 2>&1; then
    if jq empty "$csharp_file" >/dev/null 2>&1 || jq empty "$rust_file" >/dev/null 2>&1; then
      echo "FAIL $test_id (one side returned JSON and the other returned text)"
      FAIL_COUNT=$((FAIL_COUNT + 1))
      record_failure "$test_id" "one side returned JSON and the other returned text"
      return
    fi

    normalize_text "$csharp_file" "$csharp_text"
    normalize_text "$rust_file" "$rust_text"

    if cmp -s "$csharp_text" "$rust_text"; then
      echo "PASS $test_id (text match)"
      PASS_EXACT=$((PASS_EXACT + 1))
      return
    fi

    echo "FAIL $test_id (text output mismatch)"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    record_failure "$test_id" "text output mismatch"
    return
  fi

  normalize_json "$csharp_file" "$csharp_normalized"
  normalize_json "$rust_file" "$rust_normalized"

  if cmp -s "$csharp_normalized" "$rust_normalized"; then
    echo "PASS $test_id (exact)"
    PASS_EXACT=$((PASS_EXACT + 1))
    return
  fi

  csharp_type=$(jq -r 'type' "$csharp_normalized")
  rust_type=$(jq -r 'type' "$rust_normalized")

  if [ "$csharp_type" != "$rust_type" ]; then
    echo "FAIL $test_id (JSON root type mismatch: C#=$csharp_type Rust=$rust_type)"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    record_failure "$test_id" "JSON root type mismatch (C#=$csharp_type Rust=$rust_type)"
    return
  fi

  csharp_has_success=$(jq -r 'if type == "object" and has("success") then "yes" else "no" end' "$csharp_normalized")
  rust_has_success=$(jq -r 'if type == "object" and has("success") then "yes" else "no" end' "$rust_normalized")

  if [ "$csharp_has_success" = "no" ] || [ "$rust_has_success" = "no" ]; then
    if [ "$csharp_has_success" != "$rust_has_success" ]; then
      echo "FAIL $test_id (JSON envelope mismatch: only one side has success/error wrapper)"
      FAIL_COUNT=$((FAIL_COUNT + 1))
      record_failure "$test_id" "JSON envelope mismatch"
      return
    fi

    extract_json_structure "$csharp_normalized" "$csharp_structure"
    extract_json_structure "$rust_normalized" "$rust_structure"

    if cmp -s "$csharp_structure" "$rust_structure"; then
      echo "DIFF $test_id (generic JSON structural match)"
      PASS_STRUCT=$((PASS_STRUCT + 1))
      return
    fi

    echo "FAIL $test_id (generic JSON mismatch)"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    record_failure "$test_id" "generic JSON mismatch"
    return
  fi

  csharp_success=$(jq -r '.success' "$csharp_normalized")
  rust_success=$(jq -r '.success' "$rust_normalized")

  if [ "$csharp_success" != "$rust_success" ]; then
    echo "FAIL $test_id (success field mismatch: C#=$csharp_success Rust=$rust_success)"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    record_failure "$test_id" "success field mismatch (C#=$csharp_success Rust=$rust_success)"
    return
  fi

  csharp_error_code=$(jq -r '.error.code // ""' "$csharp_normalized")
  rust_error_code=$(jq -r '.error.code // ""' "$rust_normalized")

  if [ "$csharp_error_code" != "$rust_error_code" ]; then
    echo "FAIL $test_id (error.code mismatch: C#=${csharp_error_code:-<none>} Rust=${rust_error_code:-<none>})"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    record_failure "$test_id" "error.code mismatch (C#=${csharp_error_code:-<none>} Rust=${rust_error_code:-<none>})"
    return
  fi

  if [ "$csharp_success" = "true" ]; then
    extract_data_structure "$csharp_normalized" "$csharp_structure"
    extract_data_structure "$rust_normalized" "$rust_structure"

    if cmp -s "$csharp_structure" "$rust_structure"; then
      echo "DIFF $test_id (structural match)"
      PASS_STRUCT=$((PASS_STRUCT + 1))
      return
    fi

    echo "FAIL $test_id (data structure mismatch)"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    record_failure "$test_id" "data structure mismatch"
    return
  fi

  extract_error_structure "$csharp_normalized" "$csharp_structure"
  extract_error_structure "$rust_normalized" "$rust_structure"

  if cmp -s "$csharp_structure" "$rust_structure"; then
    echo "DIFF $test_id (error envelope structural match)"
    PASS_STRUCT=$((PASS_STRUCT + 1))
    return
  fi

  echo "FAIL $test_id (error envelope mismatch)"
  FAIL_COUNT=$((FAIL_COUNT + 1))
  record_failure "$test_id" "error envelope mismatch"
}

run_parity_test() {
  local test_id="$1"
  local tool_name="$2"
  local payload_json="$3"
  local csharp_stdout="$RESULTS_DIR/$test_id.csharp.json"
  local rust_stdout="$RESULTS_DIR/$test_id.rust.json"
  local csharp_stderr="$RESULTS_DIR/$test_id.csharp.stderr.log"
  local rust_stderr="$RESULTS_DIR/$test_id.rust.stderr.log"
  local csharp_exit
  local rust_exit

  reset_test_roots

  csharp_exit=$(run_cli_command "$CSHARP_BIN" "$CSHARP_ROOT" "$tool_name" "$payload_json" "$csharp_stdout" "$csharp_stderr")
  rust_exit=$(run_cli_command "$RUST_BIN" "$RUST_ROOT" "$tool_name" "$payload_json" "$rust_stdout" "$rust_stderr")

  compare_results "$test_id" "$csharp_stdout" "$rust_stdout" "$csharp_exit" "$rust_exit"
}

register_matrix_tests() {
  # T-1.6: RTM NFR-1
  # Memory Operations (CLI)
  register_test "MEM-W01" "WriteMemory" '{"title":"Parity Test 1","content":"Testing [[parity]] with [[concepts]]"}'
  register_test "MEM-W04" "WriteMemory" '{"content":"no title [[test]]"}'
  register_test "MEM-W05" "WriteMemory" '{"title":"no content"}'
  register_test "MEM-W06" "WriteMemory" '{"title":"plain","content":"no wikilinks here"}'
  register_test "MEM-R04" "ReadMemory" '{"identifier":"memory://does-not-exist/fake"}'
  register_test "MEM-D02" "DeleteMemory" '{"identifier":"nonexistent"}'
  register_test "MEM-D03" "DeleteMemory" '{"identifier":"nonexistent","confirm":true}'

  # Graph Operations (CLI)
  register_test "GRP-S02" "Sync" '{"learn":true}'
  register_test "GRP-B03" "BuildContext" '{"conceptName":"nonexistentconceptxyz123"}'

  # System Tools (CLI)
  register_test "SYS-H01" "GetHelp" '{"toolName":"WriteMemory"}'
  register_test "SYS-H02" "GetHelp" '{"toolName":"FakeTool"}'
  register_test "SYS-C01" "GetConfig" '{}'
  register_test "SYS-L01" "ListMemories" '{"learn":true}'

  # Session Tools (CLI)
  register_test "SEQ-01" "SequentialThinking" '{"response":"Testing [[parity]] for [[sequential-thinking]]","thoughtNumber":1,"totalThoughts":3,"nextThoughtNeeded":true}'
  register_test "WF-01" "Workflow" '{"learn":true}'
  register_test "AL-01" "AssumptionLedger" '{"action":"read","learn":true}'

  # Asset Tools (CLI)
  register_test "AST-L01" "ListAssets" '{}'
  register_test "AST-C01" "ReadMcpResource" '{"uri":"asset://catalog"}'
  register_test "AST-A01" "Adopt" '{"type":"role","identifier":"test","learn":true}'
  register_test "AST-U01" "UpdateAssets" '{"learn":true}'

  # Concept Tools (CLI)
  register_test "REP-01" "RepairConcepts" '{"learn":true}'
  register_test "ACC-01" "AnalyzeConceptCorruption" '{"learn":true}'

  # Error Handling (CLI)
  register_test "ERR-01" "NonExistentTool" '{"foo":"bar"}'
  register_test "ERR-02" "WriteMemory" 'not-json'
}

run_registered_tests() {
  local index

  for ((index = 0; index < ${#TEST_IDS[@]}; index++)); do
    if should_run_test "${TEST_IDS[$index]}"; then
      run_parity_test "${TEST_IDS[$index]}" "${TEST_TOOLS[$index]}" "${TEST_PAYLOADS[$index]}"
    fi
  done
}

print_summary() {
  local failure

  echo
  echo "=== PARITY TEST RESULTS ==="
  echo "Total: $TOTAL"
  echo "Pass (exact): $PASS_EXACT"
  echo "Pass (structural): $PASS_STRUCT"
  echo "Fail: $FAIL_COUNT"
  echo
  echo "FAILURES:"

  if [ ${#FAILURES[@]} -eq 0 ]; then
    echo "- None"
  else
    for failure in "${FAILURES[@]}"; do
      echo "- $failure"
    done
  fi
}

main() {
  cd "$REPO_ROOT" || exit 1
  require_prereqs
  register_matrix_tests
  setup_test_data
  run_registered_tests
  print_summary

  if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
  fi
}

main "$@"
