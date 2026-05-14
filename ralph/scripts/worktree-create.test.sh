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
REPO_ROOT="$(git -C "${SCRIPT_DIR}" rev-parse --show-toplevel)"
HELPER="${REPO_ROOT}/ralph/scripts/worktree-create.sh"
TEST_TASK_ID="T-303.4.23-smoketest-$$"
WORKTREE_DIR="${REPO_ROOT}/worktrees/${TEST_TASK_ID}"
BRANCH_NAME="worker-${TEST_TASK_ID}"
IDEMPOTENCY_TASK_A="T-303.4.27-smoketestA-$$"
IDEMPOTENCY_TASK_B="T-303.4.27-smoketestB-$$"
IDEMPOTENCY_WORKTREE_A="${REPO_ROOT}/worktrees/${IDEMPOTENCY_TASK_A}"
IDEMPOTENCY_WORKTREE_B="${REPO_ROOT}/worktrees/${IDEMPOTENCY_TASK_B}"
IDEMPOTENCY_BRANCH_A="worker-${IDEMPOTENCY_TASK_A}"
IDEMPOTENCY_BRANCH_B="worker-${IDEMPOTENCY_TASK_B}"
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
  if [[ -e "${WORKTREE_DIR}" ]]; then
    git -C "${REPO_ROOT}" worktree remove --force "${WORKTREE_DIR}" >/dev/null 2>&1 || true
  fi
  if [[ -e "${IDEMPOTENCY_WORKTREE_A}" ]]; then
    git -C "${REPO_ROOT}" worktree remove --force "${IDEMPOTENCY_WORKTREE_A}" >/dev/null 2>&1 || true
  fi
  if [[ -e "${IDEMPOTENCY_WORKTREE_B}" ]]; then
    git -C "${REPO_ROOT}" worktree remove --force "${IDEMPOTENCY_WORKTREE_B}" >/dev/null 2>&1 || true
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
