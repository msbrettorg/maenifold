#!/usr/bin/env bash
#
# worktree-create.test.sh - Smoke test for worktree-create.sh
#
# Verifies the script:
#   1. Creates a worktree on the named branch
#   2. Symlinks todo/ + done/ to the main repo
#   3. Makes a real spec file readable through the worktree-relative path
#   4. Detects duplicate worktree creation
#   5. Cleans up properly
#
# Run from repo root: ralph/scripts/worktree-create.test.sh
# Exits 0 on success, non-zero on first failure.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMMON_GIT_DIR="$(git -C "${SCRIPT_DIR}" rev-parse --git-common-dir)"
case "${COMMON_GIT_DIR}" in
  /*) ;;
  *) COMMON_GIT_DIR="$(cd "${SCRIPT_DIR}" && cd "${COMMON_GIT_DIR}" && pwd)" ;;
esac
REPO_ROOT="$(cd "$(dirname "${COMMON_GIT_DIR}")" && pwd)"
HELPER="${SCRIPT_DIR}/worktree-create.sh"
TEST_TASK_ID="T-303.4.23-smoketest-$$"
WORKTREE_DIR="${REPO_ROOT}/worktrees/${TEST_TASK_ID}"
BRANCH_NAME="worker-${TEST_TASK_ID}"
IDEMPOTENCY_TASK_A="T-303.4.27-smoketestA-$$"
IDEMPOTENCY_TASK_B="T-303.4.27-smoketestB-$$"
IDEMPOTENCY_WORKTREE_A="${REPO_ROOT}/worktrees/${IDEMPOTENCY_TASK_A}"
IDEMPOTENCY_WORKTREE_B="${REPO_ROOT}/worktrees/${IDEMPOTENCY_TASK_B}"
IDEMPOTENCY_BRANCH_A="worker-${IDEMPOTENCY_TASK_A}"
IDEMPOTENCY_BRANCH_B="worker-${IDEMPOTENCY_TASK_B}"
SIGTERM_TASK="T-303.4.28-sigterm-fixture-$$"
SIGINT_TASK="T-303.4.28-sigint-fixture-$$"
STALE_LOCK_TASK="T-303.4.28-stale-fixture-$$"
NESTED_PARENT_TASK="T-303.4.29-parent-fixture-$$"
NESTED_CHILD_TASK="T-303.4.29-child-fixture-$$"
SIGTERM_WORKTREE="${REPO_ROOT}/worktrees/${SIGTERM_TASK}"
SIGINT_WORKTREE="${REPO_ROOT}/worktrees/${SIGINT_TASK}"
STALE_LOCK_WORKTREE="${REPO_ROOT}/worktrees/${STALE_LOCK_TASK}"
SIGTERM_BRANCH="worker-${SIGTERM_TASK}"
SIGINT_BRANCH="worker-${SIGINT_TASK}"
STALE_LOCK_BRANCH="worker-${STALE_LOCK_TASK}"
NESTED_PARENT_BRANCH="worker-${NESTED_PARENT_TASK}"
NESTED_CHILD_BRANCH="worker-${NESTED_CHILD_TASK}"
SIGNAL_TEST_PIDS=()
if [[ -L "${REPO_ROOT}/todo" ]]; then
  MAIN_REPO_ROOT="$(cd "$(dirname "$(readlink "${REPO_ROOT}/todo")")" && pwd)"
else
  MAIN_REPO_ROOT="${REPO_ROOT}"
fi
COMMON_GIT_DIR="$(git -C "${MAIN_REPO_ROOT}" rev-parse --git-common-dir)"
case "${COMMON_GIT_DIR}" in
  /*) ;;
  *) COMMON_GIT_DIR="${MAIN_REPO_ROOT}/${COMMON_GIT_DIR}" ;;
esac
EXCLUDE_FILE="${COMMON_GIT_DIR}/info/exclude"
NESTED_PARENT_WORKTREE="${MAIN_REPO_ROOT}/worktrees/${NESTED_PARENT_TASK}"
NESTED_CHILD_EXPECTED_WORKTREE="${MAIN_REPO_ROOT}/worktrees/${NESTED_CHILD_TASK}"
NESTED_CHILD_WRONG_WORKTREE="${NESTED_PARENT_WORKTREE}/worktrees/${NESTED_CHILD_TASK}"

if [[ -f "${EXCLUDE_FILE}" ]]; then
  ORIGINAL_EXCLUDE_EXISTS=1
  ORIGINAL_EXCLUDE_SHA="$(shasum -a 256 "${EXCLUDE_FILE}" | awk '{print $1}')"
  ORIGINAL_EXCLUDE_HEX="$(python3 -c 'import pathlib, sys; print(pathlib.Path(sys.argv[1]).read_bytes().hex())' "${EXCLUDE_FILE}")"
else
  ORIGINAL_EXCLUDE_EXISTS=0
  ORIGINAL_EXCLUDE_SHA=""
  ORIGINAL_EXCLUDE_HEX=""
fi

# Pick an existing real task spec to verify spec readability with.
REAL_TASK_SPEC=""
while IFS= read -r candidate; do
  REAL_TASK_SPEC="${candidate}"
  break
done < <(find "${REPO_ROOT}/todo/" -maxdepth 1 -name 'T-*.md' -type f | sort)
if [[ -z "${REAL_TASK_SPEC}" ]]; then
  echo "FAIL: no T-*.md spec to test against (cannot exercise spec-readable assertion)"
  exit 1
fi
REAL_TASK_SPEC_BASENAME="$(basename "${REAL_TASK_SPEC}")"

cleanup() {
  local rc=$?
  local pid
  for pid in "${SIGNAL_TEST_PIDS[@]}"; do
    if kill -0 "${pid}" >/dev/null 2>&1; then
      terminate_process_tree "${pid}" TERM
      wait "${pid}" >/dev/null 2>&1 || true
    fi
  done
  if [[ -e "${WORKTREE_DIR}" ]]; then
    git -C "${REPO_ROOT}" worktree remove --force "${WORKTREE_DIR}" >/dev/null 2>&1 || true
  fi
  if [[ -e "${IDEMPOTENCY_WORKTREE_A}" ]]; then
    git -C "${REPO_ROOT}" worktree remove --force "${IDEMPOTENCY_WORKTREE_A}" >/dev/null 2>&1 || true
  fi
  if [[ -e "${IDEMPOTENCY_WORKTREE_B}" ]]; then
    git -C "${REPO_ROOT}" worktree remove --force "${IDEMPOTENCY_WORKTREE_B}" >/dev/null 2>&1 || true
  fi
  if [[ -e "${SIGTERM_WORKTREE}" ]]; then
    git -C "${REPO_ROOT}" worktree remove --force "${SIGTERM_WORKTREE}" >/dev/null 2>&1 || true
  fi
  if [[ -e "${SIGINT_WORKTREE}" ]]; then
    git -C "${REPO_ROOT}" worktree remove --force "${SIGINT_WORKTREE}" >/dev/null 2>&1 || true
  fi
  if [[ -e "${STALE_LOCK_WORKTREE}" ]]; then
    git -C "${REPO_ROOT}" worktree remove --force "${STALE_LOCK_WORKTREE}" >/dev/null 2>&1 || true
  fi
  if [[ -e "${NESTED_CHILD_WRONG_WORKTREE}" ]]; then
    git -C "${MAIN_REPO_ROOT}" worktree remove --force "${NESTED_CHILD_WRONG_WORKTREE}" >/dev/null 2>&1 || true
  fi
  if [[ -e "${NESTED_CHILD_EXPECTED_WORKTREE}" ]]; then
    git -C "${MAIN_REPO_ROOT}" worktree remove --force "${NESTED_CHILD_EXPECTED_WORKTREE}" >/dev/null 2>&1 || true
  fi
  if [[ -e "${NESTED_PARENT_WORKTREE}" ]]; then
    git -C "${MAIN_REPO_ROOT}" worktree remove --force "${NESTED_PARENT_WORKTREE}" >/dev/null 2>&1 || true
  fi
  if git -C "${REPO_ROOT}" show-ref --verify --quiet "refs/heads/${BRANCH_NAME}"; then
    git -C "${REPO_ROOT}" branch -D "${BRANCH_NAME}" >/dev/null 2>&1 || true
  fi
  if git -C "${REPO_ROOT}" show-ref --verify --quiet "refs/heads/${IDEMPOTENCY_BRANCH_A}"; then
    git -C "${REPO_ROOT}" branch -D "${IDEMPOTENCY_BRANCH_A}" >/dev/null 2>&1 || true
  fi
  if git -C "${REPO_ROOT}" show-ref --verify --quiet "refs/heads/${IDEMPOTENCY_BRANCH_B}"; then
    git -C "${REPO_ROOT}" branch -D "${IDEMPOTENCY_BRANCH_B}" >/dev/null 2>&1 || true
  fi
  if git -C "${REPO_ROOT}" show-ref --verify --quiet "refs/heads/${SIGTERM_BRANCH}"; then
    git -C "${REPO_ROOT}" branch -D "${SIGTERM_BRANCH}" >/dev/null 2>&1 || true
  fi
  if git -C "${REPO_ROOT}" show-ref --verify --quiet "refs/heads/${SIGINT_BRANCH}"; then
    git -C "${REPO_ROOT}" branch -D "${SIGINT_BRANCH}" >/dev/null 2>&1 || true
  fi
  if git -C "${REPO_ROOT}" show-ref --verify --quiet "refs/heads/${STALE_LOCK_BRANCH}"; then
    git -C "${REPO_ROOT}" branch -D "${STALE_LOCK_BRANCH}" >/dev/null 2>&1 || true
  fi
  if git -C "${MAIN_REPO_ROOT}" show-ref --verify --quiet "refs/heads/${NESTED_CHILD_BRANCH}"; then
    git -C "${MAIN_REPO_ROOT}" branch -D "${NESTED_CHILD_BRANCH}" >/dev/null 2>&1 || true
  fi
  if git -C "${MAIN_REPO_ROOT}" show-ref --verify --quiet "refs/heads/${NESTED_PARENT_BRANCH}"; then
    git -C "${MAIN_REPO_ROOT}" branch -D "${NESTED_PARENT_BRANCH}" >/dev/null 2>&1 || true
  fi
  rmdir "${EXCLUDE_FILE}.lock" >/dev/null 2>&1 || true
  if [[ "${ORIGINAL_EXCLUDE_EXISTS}" == "1" ]]; then
    python3 -c 'import pathlib, sys; pathlib.Path(sys.argv[1]).write_bytes(bytes.fromhex(sys.argv[2]))' \
      "${EXCLUDE_FILE}" "${ORIGINAL_EXCLUDE_HEX}" || true
  else
    rm -f "${EXCLUDE_FILE}" || true
  fi
  exit ${rc}
}
trap cleanup EXIT

fail() {
  echo "FAIL: $1" >&2
  exit 1
}

TEST_FAILURES=0

record_failure() {
  echo "FAIL: $1" >&2
  TEST_FAILURES=$((TEST_FAILURES + 1))
}

wait_for_lock() {
  local lock_path="$1"
  local timeout_ms="${2:-2000}"
  local elapsed_ms=0
  local poll_ms=50
  while [[ ! -d "${lock_path}" ]]; do
    sleep 0.05
    elapsed_ms=$((elapsed_ms + poll_ms))
    if [[ ${elapsed_ms} -ge ${timeout_ms} ]]; then
      return 1
    fi
  done
  return 0
}

wait_for_process_exit() {
  local pid="$1"
  local timeout_ms="${2:-2000}"
  local elapsed_ms=0
  local poll_ms=50
  while kill -0 "${pid}" >/dev/null 2>&1; do
    sleep 0.05
    elapsed_ms=$((elapsed_ms + poll_ms))
    if [[ ${elapsed_ms} -ge ${timeout_ms} ]]; then
      return 1
    fi
  done
  return 0
}

terminate_process_tree() {
  local root_pid="$1"
  local signal_name="${2:-KILL}"
  local child_pid
  while IFS= read -r child_pid; do
    [[ -n "${child_pid}" ]] || continue
    terminate_process_tree "${child_pid}" "${signal_name}"
  done < <(pgrep -P "${root_pid}" 2>/dev/null || true)
  kill "-${signal_name}" "${root_pid}" >/dev/null 2>&1 || true
}

run_helper_holding_exclude_lock() {
  local task_id="$1"
  exec python3 -c '
import os
import signal
import sys

signal.signal(signal.SIGINT, signal.SIG_DFL)
signal.signal(signal.SIGTERM, signal.SIG_DFL)

bash_script = r"""
set -euo pipefail
helper="$1"
exclude_file="$2"
task_id="$3"
touch() {
  command touch "$@"
  local touch_rc=$?
  if [[ ${touch_rc} -eq 0 ]] && [[ "${1:-}" == "${exclude_file}" ]]; then
    while [[ -d "${1}.lock" ]]; do
      :
    done
  fi
  return "${touch_rc}"
}
source "${helper}" "${task_id}"
"""

os.execv("/bin/bash", ["bash", "-c", bash_script, "bash", sys.argv[1], sys.argv[2], sys.argv[3]])
' "${HELPER}" "${EXCLUDE_FILE}" "${task_id}"
}

run_signal_lock_cleanup_test() {
  local test_number="$1"
  local signal_name="$2"
  local task_id="$3"
  local lock_path="${EXCLUDE_FILE}.lock"
  local pid

  rmdir "${lock_path}" >/dev/null 2>&1 || true
  run_helper_holding_exclude_lock "${task_id}" >/dev/null 2>&1 &
  pid=$!
  SIGNAL_TEST_PIDS+=("${pid}")

  if ! wait_for_lock "${lock_path}" 60000; then
    kill -TERM "${pid}" >/dev/null 2>&1 || true
    wait "${pid}" >/dev/null 2>&1 || true
    record_failure "Test ${test_number}: could not observe lock acquisition at ${lock_path}"
    return
  fi

  kill "-${signal_name}" "${pid}" >/dev/null 2>&1 || true

  if ! wait_for_process_exit "${pid}" 2000; then
    terminate_process_tree "${pid}" KILL
    wait "${pid}" >/dev/null 2>&1 || true
    if [[ -d "${lock_path}" ]]; then
      record_failure "Test ${test_number}: SIG${signal_name} did not terminate cleanly and left stale lock dir at ${lock_path}"
    else
      record_failure "Test ${test_number}: worktree-create.sh did not terminate after SIG${signal_name}"
    fi
    rmdir "${lock_path}" >/dev/null 2>&1 || true
    return
  fi
  wait "${pid}" >/dev/null 2>&1 || true

  if [[ -d "${lock_path}" ]]; then
    record_failure "Test ${test_number}: SIG${signal_name} left stale lock dir at ${lock_path}"
    rmdir "${lock_path}" >/dev/null 2>&1 || true
    return
  fi

  echo "PASS: Test ${test_number} SIG${signal_name} released ${lock_path}"
}

assert_no_status_entry() {
  local path_name="$1"
  local status_output="$2"
  if grep -qx "?? ${path_name}" <<<"${status_output}"; then
    fail "'${path_name}' was not ignored — git status reported '?? ${path_name}'"
  fi
}

assert_not_other_file() {
  local path_name="$1"
  local others_output="$2"
  if grep -qx "${path_name}" <<<"${others_output}"; then
    fail "'${path_name}' was not ignored — git ls-files --others --exclude-standard reported '${path_name}'"
  fi
}

assert_not_staged() {
  local path_name="$1"
  local staged_output="$2"
  if grep -qx "${path_name}" <<<"${staged_output}"; then
    fail "'${path_name}' symlink was staged by git add -A"
  fi
}

assert_exclude_preserved_with_patterns() {
  python3 - "${EXCLUDE_FILE}" "${ORIGINAL_EXCLUDE_HEX}" <<'PY'
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
original = bytes.fromhex(sys.argv[2])
current = path.read_bytes()
required = [b"/todo", b"/done"]

if current == original:
    missing = [line.decode() for line in required if line not in original.splitlines()]
    if missing:
        print(f"unchanged exclude file is missing required patterns: {', '.join(missing)}", file=sys.stderr)
        sys.exit(1)
    sys.exit(0)

lines = current.splitlines()
for required_line in required:
    if lines.count(required_line) != 1:
        print(f"expected exactly one {required_line.decode()} line in .git/info/exclude", file=sys.stderr)
        sys.exit(1)

remaining = lines.copy()
for required_line in required:
    remaining.remove(required_line)

if remaining != original.splitlines():
    print("pre-existing .git/info/exclude content was not preserved", file=sys.stderr)
    sys.exit(1)
PY
}

# Test 1: script exists and is executable
[[ -x "${HELPER}" ]] || fail "${HELPER} is not executable"

# Test 2: usage error when called with no args
if "${HELPER}" >/dev/null 2>&1; then
  fail "expected non-zero exit when called with no args"
fi

# Test 3: creates worktree successfully
"${HELPER}" "${TEST_TASK_ID}" >/dev/null 2>&1 || fail "worktree-create.sh failed for ${TEST_TASK_ID}"

# Test 4: worktree directory exists
[[ -d "${WORKTREE_DIR}" ]] || fail "worktree dir not created at ${WORKTREE_DIR}"

# Test 5: branch was created
git -C "${REPO_ROOT}" show-ref --verify --quiet "refs/heads/${BRANCH_NAME}" \
  || fail "branch ${BRANCH_NAME} was not created"

# Test 6: todo/ symlink exists and resolves to main repo
[[ -L "${WORKTREE_DIR}/todo" ]] || fail "todo/ in worktree is not a symlink"
TODO_TARGET="$(readlink "${WORKTREE_DIR}/todo")"
[[ "${TODO_TARGET}" == "${REPO_ROOT}/todo" ]] \
  || fail "todo/ symlink points to ${TODO_TARGET}, expected ${REPO_ROOT}/todo"

# Test 7: done/ symlink exists and resolves to main repo
[[ -L "${WORKTREE_DIR}/done" ]] || fail "done/ in worktree is not a symlink"
DONE_TARGET="$(readlink "${WORKTREE_DIR}/done")"
[[ "${DONE_TARGET}" == "${REPO_ROOT}/done" ]] \
  || fail "done/ symlink points to ${DONE_TARGET}, expected ${REPO_ROOT}/done"

# Test 8: a real spec file is readable through the worktree symlink
SPEC_THROUGH_SYMLINK="${WORKTREE_DIR}/todo/${REAL_TASK_SPEC_BASENAME}"
[[ -r "${SPEC_THROUGH_SYMLINK}" ]] \
  || fail "spec ${REAL_TASK_SPEC_BASENAME} is not readable from worktree path ${SPEC_THROUGH_SYMLINK}"

# Test 9: contents through the symlink match the main repo file (sanity)
diff -q "${SPEC_THROUGH_SYMLINK}" "${REAL_TASK_SPEC}" >/dev/null \
  || fail "spec contents differ between worktree path and main repo path"

# Test 10: duplicate creation fails cleanly
if "${HELPER}" "${TEST_TASK_ID}" >/dev/null 2>&1; then
  fail "expected duplicate worktree creation to fail"
fi

# Test 11: node_modules symlink (only if main repo has it)
if [[ -d "${REPO_ROOT}/apps/desktop/node_modules" ]]; then
  [[ -L "${WORKTREE_DIR}/apps/desktop/node_modules" ]] \
    || fail "apps/desktop/node_modules in worktree is not a symlink (main repo has it)"
fi

echo "PASS: all 11 worktree-create.sh smoke checks passed"

# Test 12: todo/done symlinks are ignored in the created worktree
WORKTREE_STATUS="$(git -C "${WORKTREE_DIR}" status --short)"
assert_no_status_entry "todo" "${WORKTREE_STATUS}"
assert_no_status_entry "done" "${WORKTREE_STATUS}"
WORKTREE_OTHERS="$(git -C "${WORKTREE_DIR}" ls-files --others --exclude-standard)"
assert_not_other_file "todo" "${WORKTREE_OTHERS}"
assert_not_other_file "done" "${WORKTREE_OTHERS}"

# Test 13: git add -A does not stage todo/done symlinks
git -C "${WORKTREE_DIR}" add -A
STAGED_NAMES="$(git -C "${WORKTREE_DIR}" diff --cached --name-only)"
git -C "${WORKTREE_DIR}" reset -- todo done >/dev/null 2>&1 || true
assert_not_staged "todo" "${STAGED_NAMES}"
assert_not_staged "done" "${STAGED_NAMES}"

# Test 14: main repo still does not report todo/done as untracked
MAIN_STATUS="$(git -C "${MAIN_REPO_ROOT}" status --short)"
assert_no_status_entry "todo" "${MAIN_STATUS}"
assert_no_status_entry "done" "${MAIN_STATUS}"

# Test 15: two worktree creations add exactly one common exclude entry for each symlink
"${HELPER}" "${IDEMPOTENCY_TASK_A}" >/dev/null 2>&1 \
  || fail "worktree-create.sh failed for ${IDEMPOTENCY_TASK_A}"
"${HELPER}" "${IDEMPOTENCY_TASK_B}" >/dev/null 2>&1 \
  || fail "worktree-create.sh failed for ${IDEMPOTENCY_TASK_B}"
TODO_EXCLUDE_COUNT="$(grep -xc '/todo' "${EXCLUDE_FILE}" || true)"
DONE_EXCLUDE_COUNT="$(grep -xc '/done' "${EXCLUDE_FILE}" || true)"
[[ "${TODO_EXCLUDE_COUNT}" == "1" ]] \
  || fail "expected exactly one /todo line in common .git/info/exclude, found ${TODO_EXCLUDE_COUNT}"
[[ "${DONE_EXCLUDE_COUNT}" == "1" ]] \
  || fail "expected exactly one /done line in common .git/info/exclude, found ${DONE_EXCLUDE_COUNT}"

# Test 16: pre-existing common exclude content is preserved
if ! assert_exclude_preserved_with_patterns; then
  fail "common .git/info/exclude was not preserved with only missing /todo and /done lines appended"
fi
CURRENT_EXCLUDE_SHA="$(shasum -a 256 "${EXCLUDE_FILE}" | awk '{print $1}')"
if [[ "${CURRENT_EXCLUDE_SHA}" == "${ORIGINAL_EXCLUDE_SHA}" ]]; then
  grep -qx '/todo' "${EXCLUDE_FILE}" \
    || fail "common .git/info/exclude stayed byte-identical but lacks /todo"
  grep -qx '/done' "${EXCLUDE_FILE}" \
    || fail "common .git/info/exclude stayed byte-identical but lacks /done"
fi

echo "PASS: all 16 worktree-create.sh smoke checks passed"

# Test 17: SIGTERM during lock cleanup leaves no .lock dir
run_signal_lock_cleanup_test 17 TERM "${SIGTERM_TASK}"

# Test 18: SIGINT during lock cleanup leaves no .lock dir
run_signal_lock_cleanup_test 18 INT "${SIGINT_TASK}"

# Test 19: stale lock timeout tells the operator how to recover
rmdir "${EXCLUDE_FILE}.lock" >/dev/null 2>&1 || true
mkdir "${EXCLUDE_FILE}.lock"
set +e
STALE_LOCK_STDERR="$("${HELPER}" "${STALE_LOCK_TASK}" 2>&1 >/dev/null)"
STALE_LOCK_RC=$?
set -e
if [[ ${STALE_LOCK_RC} -eq 0 ]]; then
  record_failure "Test 19: expected stale lock invocation to exit non-zero"
fi
if [[ "${STALE_LOCK_STDERR}" != *rmdir* ]] || [[ "${STALE_LOCK_STDERR}" != *"${EXCLUDE_FILE}.lock"* ]]; then
  record_failure "Test 19: stale lock stderr did not mention both rmdir and ${EXCLUDE_FILE}.lock; stderr was: ${STALE_LOCK_STDERR}"
else
  echo "PASS: Test 19 stale lock stderr includes rmdir recovery instructions"
fi
rmdir "${EXCLUDE_FILE}.lock" >/dev/null 2>&1 || true

# Test 20: creating a worktree from inside an existing worktree still uses the main repo worktrees dir
(cd "${MAIN_REPO_ROOT}" && "${HELPER}" "${NESTED_PARENT_TASK}") >/dev/null 2>&1 \
  || fail "Test 20: failed to create parent fixture worktree ${NESTED_PARENT_TASK}"
[[ -d "${NESTED_PARENT_WORKTREE}" ]] \
  || fail "Test 20: parent fixture worktree was not created at ${NESTED_PARENT_WORKTREE}"

set +e
(cd "${NESTED_PARENT_WORKTREE}" && "${HELPER}" "${NESTED_CHILD_TASK}") >/dev/null 2>&1
NESTED_CHILD_RC=$?
set -e
if [[ ${NESTED_CHILD_RC} -ne 0 ]]; then
  record_failure "Test 20: worktree-create.sh failed from inside ${NESTED_PARENT_WORKTREE}"
elif [[ -d "${NESTED_CHILD_EXPECTED_WORKTREE}" ]]; then
  if [[ -e "${NESTED_CHILD_WRONG_WORKTREE}" ]]; then
    record_failure "Test 20: expected only ${NESTED_CHILD_EXPECTED_WORKTREE}, but nested path also exists at ${NESTED_CHILD_WRONG_WORKTREE}"
  else
    echo "PASS: Test 20 created child worktree at main repo path ${NESTED_CHILD_EXPECTED_WORKTREE}"
  fi
elif [[ -e "${NESTED_CHILD_WRONG_WORKTREE}" ]]; then
  record_failure "Test 20: expected new worktree at ${NESTED_CHILD_EXPECTED_WORKTREE} but found ${NESTED_CHILD_WRONG_WORKTREE}"
else
  record_failure "Test 20: expected new worktree at ${NESTED_CHILD_EXPECTED_WORKTREE} but neither it nor ${NESTED_CHILD_WRONG_WORKTREE} exists"
fi

if [[ ${TEST_FAILURES} -gt 0 ]]; then
  echo "FAIL: ${TEST_FAILURES} worktree-create.sh regression checks failed" >&2
  exit 1
fi

echo "PASS: all 20 worktree-create.sh checks passed"
