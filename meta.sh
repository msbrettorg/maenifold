#!/bin/bash
#
# meta.sh — AutoAgent-style harness self-optimization for maenifold
#
# Outer loop that optimizes the RALPH harness (workflows, roles, prompts,
# model assignments) by running RALPH inner loops and measuring gate outcomes.
#
# Guards (per duo review 2026-04-19):
#   - Lockfile prevents concurrent runs
#   - Git tag snapshot before each iteration enables shell-enforced revert
#   - Verdict file (meta-verdict.json) is the structured contract — shell reverts
#     if missing, malformed, or kept=false
#   - Immutable file hashes verified after each iteration
#   - set -euo pipefail for fail-fast
#
# Usage:
#   ./meta.sh [--model <model>] [--max-iterations <n>] [--inner-iterations <n>]
#             [--inner-model <model>] [--crate <crate>] [--path <path>]
#

set -euo pipefail

META_MODEL="claude-opus-4.7"
MAX_ITERATIONS=10
INNER_ITERATIONS=20
INNER_MODEL="claude-opus-4.6-1m"
CRATE=""
SCOPE_PATH=""

# Files the meta-agent MUST NOT modify — verified by hash after each iteration
IMMUTABLE_FILES=(
  "ralph/meta-prompt.md"
  "meta.sh"
)

while [[ $# -gt 0 ]]; do
  case "$1" in
    --model)           META_MODEL="${2:-}";       shift 2 ;;
    --max-iterations)  MAX_ITERATIONS="${2:-}";   shift 2 ;;
    --inner-iterations) INNER_ITERATIONS="${2:-}"; shift 2 ;;
    --inner-model)     INNER_MODEL="${2:-}";      shift 2 ;;
    --crate)           CRATE="${2:-}";            shift 2 ;;
    --path)            SCOPE_PATH="${2:-}";       shift 2 ;;
    *)
      echo "Unknown argument: $1"
      echo "Usage: $0 [--model <model>] [--max-iterations <n>] [--inner-iterations <n>] [--inner-model <model>] [--crate <crate>] [--path <path>]"
      exit 1
      ;;
  esac
done

mkdir -p ralph/state

# --- Lockfile: prevent concurrent meta runs ---
LOCK="ralph/state/meta.lock"
exec 9>"$LOCK"
if ! flock -n 9; then
  echo "ERROR: meta.sh is already running (lockfile: $LOCK)"
  exit 1
fi

# --- Compute immutable file hashes ---
declare -A IMMUTABLE_HASHES
for f in "${IMMUTABLE_FILES[@]}"; do
  if [[ -f "$f" ]]; then
    IMMUTABLE_HASHES["$f"]=$(shasum -a 256 "$f" | cut -d' ' -f1)
  fi
done

verify_immutable_files() {
  for f in "${IMMUTABLE_FILES[@]}"; do
    if [[ -f "$f" ]]; then
      current_hash=$(shasum -a 256 "$f" | cut -d' ' -f1)
      if [[ "$current_hash" != "${IMMUTABLE_HASHES[$f]}" ]]; then
        echo "VIOLATION: immutable file $f was modified! Reverting."
        return 1
      fi
    fi
  done
  return 0
}

# --- Main loop ---
consecutive_reverts=0

for ((iteration=1; iteration<=MAX_ITERATIONS; iteration++)); do
  if [[ -f "ralph/state/meta-converged" ]]; then
    echo "Meta-converged marker found before iteration ${iteration}; exiting."
    break
  fi

  start_epoch="$(date +%s)"
  echo ""
  echo "╔══════════════════════════════════════════════════════════════════╗"
  echo "║  META iteration ${iteration}/${MAX_ITERATIONS}  (model=${META_MODEL})"
  echo "║  Inner: ${INNER_ITERATIONS} RALPH iterations  (model=${INNER_MODEL})"
  echo "║  Scope: crate=${CRATE:-all}  path=${SCOPE_PATH:-all}"
  echo "║  Consecutive reverts: ${consecutive_reverts}/3"
  echo "╚══════════════════════════════════════════════════════════════════╝"
  echo ""

  # Snapshot: git tag before the meta-agent touches anything
  SNAPSHOT_TAG="meta-snapshot-${iteration}-$(date +%s)"
  git tag "$SNAPSHOT_TAG"
  echo "Snapshot tagged: $SNAPSHOT_TAG"

  # Clear inner convergence marker and previous verdict
  rm -f ralph/state/converged
  rm -f ralph/state/meta-verdict.json

  # Export inner-loop config for the meta-prompt to reference
  export META_ITERATION="$iteration"
  export META_MAX_ITERATIONS="$MAX_ITERATIONS"
  export META_INNER_ITERATIONS="$INNER_ITERATIONS"
  export META_INNER_MODEL="$INNER_MODEL"
  export META_SNAPSHOT_TAG="$SNAPSHOT_TAG"
  export RALPH_TARGET_CRATE="$CRATE"
  export RALPH_TARGET_PATH="$SCOPE_PATH"

  # Capture gate baseline before meta-agent runs
  gate status --root . --json > ralph/state/meta-baseline.json 2>/dev/null || true
  done_count_before=$(ls done/ 2>/dev/null | wc -l | tr -d ' ')
  export META_DONE_COUNT_BEFORE="$done_count_before"

  copilot -p "$(cat ralph/meta-prompt.md)" --agent product-manager --yolo --model "$META_MODEL"
  exit_code=$?

  end_epoch="$(date +%s)"
  duration=$((end_epoch - start_epoch))

  # --- Post-iteration verification ---
  should_revert=false
  revert_reason=""

  # 1. Check immutable files
  if ! verify_immutable_files; then
    should_revert=true
    revert_reason="immutable file modified"
  fi

  # 2. Check verdict file exists and is valid JSON
  if [[ "$should_revert" == "false" ]]; then
    if [[ ! -f "ralph/state/meta-verdict.json" ]]; then
      should_revert=true
      revert_reason="no verdict file written"
    elif ! jq -e '.kept' ralph/state/meta-verdict.json > /dev/null 2>&1; then
      should_revert=true
      revert_reason="verdict file malformed (missing .kept)"
    fi
  fi

  # 3. Check verdict: did meta-agent decide to keep?
  if [[ "$should_revert" == "false" ]]; then
    kept=$(jq -r '.kept' ralph/state/meta-verdict.json)
    if [[ "$kept" != "true" ]]; then
      should_revert=true
      revert_reason="meta-agent chose to revert: $(jq -r '.reason // "no reason"' ralph/state/meta-verdict.json)"
    fi
  fi

  # 4. Check copilot exit code
  if [[ "$should_revert" == "false" && "$exit_code" -ne 0 ]]; then
    should_revert=true
    revert_reason="copilot exited with code $exit_code"
  fi

  # --- Revert or keep ---
  if [[ "$should_revert" == "true" ]]; then
    echo "REVERTING iteration ${iteration}: ${revert_reason}"
    git reset --hard "$SNAPSHOT_TAG"
    consecutive_reverts=$((consecutive_reverts + 1))

    if [[ $consecutive_reverts -ge 3 ]]; then
      echo "3 consecutive reverts — auto-converging."
      touch ralph/state/meta-converged
    fi
  else
    echo "KEEPING iteration ${iteration}: harness change accepted"
    consecutive_reverts=0
    # Tag the accepted state
    git tag "meta-accepted-${iteration}-$(date +%s)"
  fi

  # Clean up snapshot tag (accepted state tag persists)
  git tag -d "$SNAPSHOT_TAG" 2>/dev/null || true

  echo "Meta iteration ${iteration} complete: duration=${duration}s kept=${should_revert:+false}${should_revert:+}${should_revert:-true}"

  if [[ -f "ralph/state/meta-converged" ]]; then
    echo "Meta-converged after iteration ${iteration}; exiting."
    break
  fi
done

echo ""
echo "Meta loop complete after ${iteration} iterations."
