#!/usr/bin/env bash

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "usage: $0 <task-id> [file-path ...]" >&2
  exit 64
fi

TASK_ID="$1"
shift

COMMON_GIT_DIR="$(git rev-parse --git-common-dir)"
case "${COMMON_GIT_DIR}" in
  /*) ;;
  *) COMMON_GIT_DIR="$(cd "$(git rev-parse --show-toplevel)" && cd "${COMMON_GIT_DIR}" && pwd)" ;;
esac
REPO_ROOT="$(cd "$(dirname "${COMMON_GIT_DIR}")" && pwd)"

BRANCH_NAME="worker-${TASK_ID}"
if ! git -C "${REPO_ROOT}" show-ref --verify --quiet "refs/heads/${BRANCH_NAME}"; then
  echo "error: branch ${BRANCH_NAME} does not exist under ${REPO_ROOT}" >&2
  exit 1
fi

BASE_SHA="$(git -C "${REPO_ROOT}" merge-base dev "${BRANCH_NAME}")"
MERGE_LIST="$(git -C "${REPO_ROOT}" --no-pager log --oneline "${BASE_SHA}..dev")"
if [[ -z "${MERGE_LIST}" ]]; then
  MERGE_LIST="(none)"
fi

if [[ $# -gt 0 ]]; then
  DIFF_COMMAND="git diff dev --"
  for path in "$@"; do
    printf -v QUOTED_PATH '%q' "${path}"
    DIFF_COMMAND+=" ${QUOTED_PATH}"
  done
  PATH_REMINDER=""
else
  DIFF_COMMAND="git diff dev -- <ADD-SPEC-FILE-PATHS-HERE>"
  PATH_REMINDER="> - Reminder: replace \`<ADD-SPEC-FILE-PATHS-HERE>\` with ONLY the files named in the spec before dispatching the red-team worker.
>
"
fi

printf '> **Branch-base artifact warning (mandatory for %s red-team dispatch)**\n' "${TASK_ID}"
printf '>\n'
printf '> - Worker branch: `%s`\n' "${BRANCH_NAME}"
printf '> - Branch base SHA: `%s`\n' "${BASE_SHA}"
printf '>   - Compute with: `git merge-base dev %s`\n' "${BRANCH_NAME}"
printf '> - Merges landed on `dev` since the branch base:\n'
printf '>   - Compute with: `git log --oneline %s..dev`\n' "${BASE_SHA}"
printf '>   - Current output:\n'
printf '>     ```text\n'
while IFS= read -r line; do
  printf '>     %s\n' "${line}"
done <<< "${MERGE_LIST}"
printf '>     ```\n'
printf '> - Audit diff command (REQUIRED; path-restricted to ONLY the spec-named files):\n'
printf '>   - `%s`\n' "${DIFF_COMMAND}"
printf '%s' "${PATH_REMINDER}"
printf '> - Auditor instruction: **Any file removals or changes outside the path filter are branch-base artifacts, NOT scope violations by the SWE. Ignore them. Your audit scope is ONLY the files listed after `--`.**\n'
