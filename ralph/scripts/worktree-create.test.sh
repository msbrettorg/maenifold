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

# Pick an existing real task spec to verify spec readability with.
REAL_TASK_SPEC=$(ls "${REPO_ROOT}/todo/T-303.4.22"*.md 2>/dev/null | head -1)
if [[ -z "${REAL_TASK_SPEC}" ]]; then
  echo "FAIL: no T-303.4.22 spec to test against (cannot exercise spec-readable assertion)"
  exit 1
fi
REAL_TASK_SPEC_BASENAME="$(basename "${REAL_TASK_SPEC}")"

cleanup() {
  local rc=$?
  if [[ -e "${WORKTREE_DIR}" ]]; then
    git -C "${REPO_ROOT}" worktree remove --force "${WORKTREE_DIR}" >/dev/null 2>&1 || true
  fi
  if git -C "${REPO_ROOT}" show-ref --verify --quiet "refs/heads/${BRANCH_NAME}"; then
    git -C "${REPO_ROOT}" branch -D "${BRANCH_NAME}" >/dev/null 2>&1 || true
  fi
  exit ${rc}
}
trap cleanup EXIT

fail() {
  echo "FAIL: $1" >&2
  exit 1
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
