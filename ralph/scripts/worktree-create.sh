#!/usr/bin/env bash
#
# worktree-create.sh - Create an isolated git worktree for a dispatched SWE/blue/red worker
#
# Usage: ralph/scripts/worktree-create.sh <task-id> [base-branch]
#   <task-id>      The gate task ID (e.g. T-303.4.22)
#   [base-branch]  Optional base branch (default: dev)
#
# What it does:
#   1. Creates worktree at ./worktrees/<task-id> on branch worker-<task-id>
#   2. Symlinks worktrees/<task-id>/todo -> main repo todo/
#   3. Symlinks worktrees/<task-id>/done -> main repo done/
#   4. Symlinks worktrees/<task-id>/apps/desktop/node_modules -> main repo's apps/desktop/node_modules
#      (only if apps/desktop/node_modules exists in the main repo)
#   5. Verifies the spec file (todo/<task-id>*.md) is reachable from inside the worktree
#
# Why symlinks for todo/done:
#   Per AGENTS.md (2026-04-28), todo/ and done/ are gitignored to avoid parallel-worktree
#   merge conflicts on gate state. A naive `git worktree add` therefore creates an empty
#   todo/ inside the worktree, leaving the SWE without its spec. Symlinking restores spec
#   readability without introducing the merge-conflict class this gitignore was meant to
#   prevent (the symlinks point at the main repo's live state).
#
# Closes T-303.4.23.

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <task-id> [base-branch]" >&2
  exit 64
fi

TASK_ID="$1"
BASE_BRANCH="${2:-dev}"

# Find repo root regardless of CWD
REPO_ROOT="$(git rev-parse --show-toplevel)"
WORKTREE_DIR="${REPO_ROOT}/worktrees/${TASK_ID}"
BRANCH_NAME="worker-${TASK_ID}"

if [[ -e "${WORKTREE_DIR}" ]]; then
  echo "error: worktree already exists at ${WORKTREE_DIR}" >&2
  echo "  remove with: git worktree remove ${WORKTREE_DIR} && git branch -D ${BRANCH_NAME}" >&2
  exit 1
fi

if git -C "${REPO_ROOT}" show-ref --verify --quiet "refs/heads/${BRANCH_NAME}"; then
  echo "error: branch ${BRANCH_NAME} already exists" >&2
  echo "  delete with: git branch -D ${BRANCH_NAME}" >&2
  exit 1
fi

echo "Creating worktree at ${WORKTREE_DIR} on branch ${BRANCH_NAME} (base ${BASE_BRANCH})"
git -C "${REPO_ROOT}" worktree add "${WORKTREE_DIR}" -b "${BRANCH_NAME}" "${BASE_BRANCH}" 1>/dev/null

ln -s "${REPO_ROOT}/todo" "${WORKTREE_DIR}/todo"
ln -s "${REPO_ROOT}/done" "${WORKTREE_DIR}/done"
echo "  symlinked todo/ + done/ -> main repo"

if [[ -d "${REPO_ROOT}/apps/desktop/node_modules" ]]; then
  rm -rf "${WORKTREE_DIR}/apps/desktop/node_modules"
  ln -s "${REPO_ROOT}/apps/desktop/node_modules" "${WORKTREE_DIR}/apps/desktop/node_modules"
  echo "  symlinked apps/desktop/node_modules -> main repo (skip npm ci unless package.json changed)"
fi

# Self-verify: the spec must be readable through the symlink.
SPEC_GLOB="${WORKTREE_DIR}/todo/${TASK_ID}*.md"
SPEC_MATCHES=( $(ls $SPEC_GLOB 2>/dev/null || true) )
if [[ ${#SPEC_MATCHES[@]} -eq 0 ]]; then
  echo "warning: no spec file matches ${SPEC_GLOB}" >&2
  echo "         (check that todo/${TASK_ID}*.md exists and the YAML id matches the filename stem)" >&2
else
  for SPEC in "${SPEC_MATCHES[@]}"; do
    [[ -r "${SPEC}" ]] || { echo "error: spec ${SPEC} not readable from worktree" >&2; exit 2; }
  done
  echo "  verified spec readable: ${SPEC_MATCHES[*]}"
fi

echo ""
echo "Worktree ready. SWE prompt should reference:"
echo "  cwd: ${WORKTREE_DIR}"
echo "  spec: ${WORKTREE_DIR}/todo/${TASK_ID}-*.md  (or main repo absolute path)"
echo "  branch: ${BRANCH_NAME}"
