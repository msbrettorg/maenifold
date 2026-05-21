#!/usr/bin/env bash
set -euo pipefail

TEST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

total_files=0
failing_files=0
total_cases=0
failing_cases=0

for test_file in "$TEST_DIR"/test_*.sh; do
  [[ -f "$test_file" ]] || continue
  [[ "$(basename "$test_file")" == "test_helpers.sh" ]] && continue
  total_files=$((total_files + 1))
  echo "==> $(basename "$test_file")"
  set +e
  output="$(bash "$test_file" 2>&1)"
  status=$?
  set -e
  printf '%s\n' "$output"
  summary_line="$(printf '%s\n' "$output" | grep '^SUMMARY ' | tail -n 1 || true)"
  if [[ -n "$summary_line" ]]; then
    cases="$(printf '%s\n' "$summary_line" | sed -E 's/.*total=([0-9]+).*/\1/')"
    failed="$(printf '%s\n' "$summary_line" | sed -E 's/.*failed=([0-9]+).*/\1/')"
    total_cases=$((total_cases + cases))
    failing_cases=$((failing_cases + failed))
  fi
  if [[ $status -ne 0 ]]; then
    failing_files=$((failing_files + 1))
  fi
  echo
done

echo "SUITE SUMMARY total_files=$total_files failing_files=$failing_files total_cases=$total_cases failing_cases=$failing_cases"

if [[ $failing_files -eq 0 ]]; then
  exit 0
fi

exit 1
