#!/usr/bin/env bash

set -euo pipefail

TASK_ID="T-test-99"
BRANCH_NAME="worker-${TASK_ID}"
COMMON_GIT_DIR="$(git rev-parse --git-common-dir)"
case "${COMMON_GIT_DIR}" in
  /*) ;;
  *) COMMON_GIT_DIR="$(cd "$(git rev-parse --show-toplevel)" && cd "${COMMON_GIT_DIR}" && pwd)" ;;
esac
REPO_ROOT="$(cd "$(dirname "${COMMON_GIT_DIR}")" && pwd)"
WORKTREE_DIR="${REPO_ROOT}/worktrees/${TASK_ID}"
ORIGINAL_WORKTREE_CONFIG_SET=0
ORIGINAL_WORKTREE_CONFIG=""
if ORIGINAL_WORKTREE_CONFIG="$(git -C "${REPO_ROOT}" config --get extensions.worktreeConfig)"; then
  ORIGINAL_WORKTREE_CONFIG_SET=1
fi

cleanup() {
  set +e
  git -C "${REPO_ROOT}" worktree remove "${WORKTREE_DIR}" --force >/dev/null 2>&1
  git -C "${REPO_ROOT}" branch -D "${BRANCH_NAME}" >/dev/null 2>&1
  if [[ ${ORIGINAL_WORKTREE_CONFIG_SET} -eq 1 ]]; then
    git -C "${REPO_ROOT}" config extensions.worktreeConfig "${ORIGINAL_WORKTREE_CONFIG}"
  else
    git -C "${REPO_ROOT}" config --unset extensions.worktreeConfig >/dev/null 2>&1
  fi
}
trap cleanup EXIT

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_file_exists() {
  [[ -f "$1" ]] || fail "expected file to exist: $1"
}

assert_executable() {
  [[ -x "$1" ]] || fail "expected file to be executable: $1"
}

assert_contains() {
  local haystack="$1"
  local needle="$2"
  [[ "${haystack}" == *"${needle}"* ]] || fail "expected output to contain: ${needle}"
}

main() {
  cleanup

  bash ralph/scripts/worktree-create.sh "${TASK_ID}" dev

  local worktree_gitdir
  worktree_gitdir="$(git -C "${WORKTREE_DIR}" rev-parse --git-dir)"
  case "${worktree_gitdir}" in
    /*) ;;
    *) worktree_gitdir="${WORKTREE_DIR}/${worktree_gitdir}" ;;
  esac
  local configured_hooks_dir
  configured_hooks_dir="$(git -C "${WORKTREE_DIR}" config --worktree --get core.hooksPath)"
  [[ "${configured_hooks_dir}" == "${worktree_gitdir}/hooks" ]] || fail "expected core.hooksPath to point at the worktree hook directory"

  local hook_path="${configured_hooks_dir}/pre-commit"
  assert_file_exists "${hook_path}"
  assert_executable "${hook_path}"
  grep -q 'PM_OWNS_THIS' "${hook_path}" || fail "expected hook to contain PM_OWNS_THIS"

  set +e
  local blocked_output
  blocked_output="$(cd "${WORKTREE_DIR}" && git -c user.name='RALPH Test' -c user.email='ralph-test@example.invalid' commit --allow-empty -m 'should be blocked' 2>&1)"
  local blocked_status=$?
  set -e

  [[ ${blocked_status} -ne 0 ]] || fail "expected unauthorized commit to fail"
  assert_contains "${blocked_output}" "PM_OWNS_THIS"

  set +e
  local authorized_output
  authorized_output="$(cd "${WORKTREE_DIR}" && PM_OWNS_THIS=1 git -c user.name='RALPH Test' -c user.email='ralph-test@example.invalid' commit --allow-empty -m 'authorized by PM' 2>&1)"
  local authorized_status=$?
  set -e

  [[ ${authorized_status} -eq 0 ]] || fail "expected PM-authorized commit to succeed: ${authorized_output}"

  echo "PASS: worktree-create installs and enforces PM_OWNS_THIS pre-commit hook"
}

main "$@"
