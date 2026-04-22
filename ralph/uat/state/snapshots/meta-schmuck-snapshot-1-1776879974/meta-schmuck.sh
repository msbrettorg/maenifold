#!/bin/bash

set -euo pipefail

META_MODEL="gpt-5.4"
META_REASONING_EFFORT="xhigh"
MAX_ITERATIONS=674
INNER_ITERATIONS=1
INNER_MODEL="gpt-5.4"
INNER_REASONING_EFFORT="xhigh"
SNAPSHOT_ROOT="ralph/uat/state/snapshots"

# Files the meta-agent MUST NOT modify — verified by hash after each iteration
IMMUTABLE_FILES=(
  "ralph/uat/meta-prompt.md"
  "meta-schmuck.sh"
  "schmuck.sh"
)

SNAPSHOT_FILES=(
  "ralph/uat/prompt.md"
  "ralph/uat/learnings.md"
  "ralph/uat/meta-prompt.md"
  "meta-schmuck.sh"
  "schmuck.sh"
)

usage() {
  echo "Usage: $0 [--model <model>] [--effort <low|medium|high|xhigh>] [--max-iterations <n>] [--inner-model <model>] [--inner-effort <low|medium|high|xhigh>]"
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "ERROR: missing required command: $1"
    exit 1
  }
}

snapshot_iteration_state() {
  local dir="$1"
  local f

  rm -rf "$dir"
  mkdir -p "$dir/todo" "$dir/done"
  for f in "${SNAPSHOT_FILES[@]}"; do
    mkdir -p "$dir/$(dirname "$f")"
    cp "$f" "$dir/$f"
  done

  find todo -maxdepth 1 -name 'T-205.*' -exec cp {} "$dir/todo/" \;
  find done -maxdepth 1 -name 'T-205.*' -exec cp {} "$dir/done/" \;

  if [[ -d outputs/screenshots/sprint-205 ]]; then
    mkdir -p "$dir/outputs/screenshots"
    cp -R outputs/screenshots/sprint-205 "$dir/outputs/screenshots/"
  fi
}

restore_iteration_state() {
  local dir="$1"
  local f

  for f in "${SNAPSHOT_FILES[@]}"; do
    cp "$dir/$f" "$f"
  done

  find todo -maxdepth 1 -name 'T-205.*' -delete
  find done -maxdepth 1 -name 'T-205.*' -delete

  if compgen -G "$dir/todo/T-205.*" > /dev/null; then
    cp "$dir"/todo/T-205.* todo/
  fi
  if compgen -G "$dir/done/T-205.*" > /dev/null; then
    cp "$dir"/done/T-205.* done/
  fi

  rm -rf outputs/screenshots/sprint-205
  if [[ -d "$dir/outputs/screenshots/sprint-205" ]]; then
    mkdir -p outputs/screenshots
    cp -R "$dir/outputs/screenshots/sprint-205" outputs/screenshots/
  fi

  gate sync --root . --rebuild >/dev/null 2>&1 || true
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --model)            META_MODEL="${2:-}";        shift 2 ;;
    --effort|--reasoning-effort)
                        META_REASONING_EFFORT="${2:-}"; shift 2 ;;
    --max-iterations)   MAX_ITERATIONS="${2:-}";    shift 2 ;;
    --inner-model)      INNER_MODEL="${2:-}";       shift 2 ;;
    --inner-effort|--inner-reasoning-effort)
                        INNER_REASONING_EFFORT="${2:-}"; shift 2 ;;
    *)
      echo "Unknown argument: $1"
      usage
      exit 1
      ;;
  esac
done

mkdir -p ralph/uat/state outputs/screenshots/sprint-205
mkdir -p "$SNAPSHOT_ROOT"

require_command copilot
require_command jq
require_command maenifold
require_command gate
require_command shasum

# --- Lockfile: prevent concurrent meta runs (macOS-compatible) ---
LOCK="ralph/uat/state/meta.lock"
if [ -f "$LOCK" ] && kill -0 "$(cat "$LOCK" 2>/dev/null)" 2>/dev/null; then
  echo "ERROR: meta-schmuck.sh is already running (pid=$(cat "$LOCK"), lockfile=$LOCK)"
  exit 1
fi
echo $$ > "$LOCK"
trap 'rm -f "$LOCK"' EXIT

# --- Compute immutable file hashes ---
IMMUTABLE_HASH_FILE="ralph/uat/state/immutable-hashes.txt"
rm -f "$IMMUTABLE_HASH_FILE"
for f in "${IMMUTABLE_FILES[@]}"; do
  if [[ -f "$f" ]]; then
    shasum -a 256 "$f" >> "$IMMUTABLE_HASH_FILE"
  fi
done

verify_immutable_files() {
  for f in "${IMMUTABLE_FILES[@]}"; do
    if [[ -f "$f" ]]; then
      expected=$(grep -F "  $f" "$IMMUTABLE_HASH_FILE" | head -1 | cut -d' ' -f1)
      actual=$(shasum -a 256 "$f" | cut -d' ' -f1)
      if [[ "$actual" != "$expected" ]]; then
        echo "VIOLATION: immutable file $f was modified!"
        echo "  expected: $expected"
        echo "  actual:   $actual"
        return 1
      fi
    fi
  done
  return 0
}

# --- Main loop ---
consecutive_reverts=0

for ((iteration=1; iteration<=MAX_ITERATIONS; iteration++)); do
  if [[ -f "ralph/uat/state/meta-converged" ]]; then
    echo "Meta-converged marker found before iteration ${iteration}; exiting."
    break
  fi

  start_epoch="$(date +%s)"
  echo ""
  echo "╔══════════════════════════════════════════════════════════════════╗"
  echo "║  META-SCHMUCK iteration ${iteration}/${MAX_ITERATIONS}"
  echo "║  Meta:  model=${META_MODEL} effort=${META_REASONING_EFFORT}"
  echo "║  Inner: model=${INNER_MODEL} effort=${INNER_REASONING_EFFORT}  |  1 task per iteration"
  echo "║  Consecutive reverts: ${consecutive_reverts}/3"
  echo "╚══════════════════════════════════════════════════════════════════╝"
  echo ""

  SNAPSHOT_TAG="meta-schmuck-snapshot-${iteration}-$(date +%s)"
  SNAPSHOT_DIR="${SNAPSHOT_ROOT}/${SNAPSHOT_TAG}"
  snapshot_iteration_state "$SNAPSHOT_DIR"
  echo "Snapshot saved: $SNAPSHOT_DIR"

  # Clear inner convergence marker and previous verdict
  rm -f ralph/uat/state/converged
  rm -f ralph/uat/state/meta-verdict.json
  rm -f ralph/uat/state/thinking-session-id.txt

  # Create a sequential-thinking session for this iteration via maenifold CLI
  THINKING_JSON=$(maenifold tool SequentialThinking '{"response":"[[UAT]] sprint 205 meta-iteration '"${iteration}"'","thoughtNumber":0,"totalThoughts":1,"nextThoughtNeeded":true}' --json 2>/dev/null || echo '{}')
  THINKING_SESSION=$(echo "$THINKING_JSON" | jq -r '.data.sessionId // empty')
  if [[ -z "$THINKING_SESSION" ]]; then
    echo "WARNING: Failed to create thinking session via CLI, using fallback ID"
    THINKING_SESSION="uat-meta-${iteration}-$(date +%s)"
  fi
  echo "$THINKING_SESSION" > ralph/uat/state/thinking-session-id.txt
  export SCHMUCK_THINKING_SESSION="$THINKING_SESSION"
  echo "Thinking session: $THINKING_SESSION"

  # Export inner-loop config for the meta-prompt to reference
  export META_ITERATION="$iteration"
  export META_MAX_ITERATIONS="$MAX_ITERATIONS"
  export META_INNER_ITERATIONS="$INNER_ITERATIONS"
  export META_INNER_MODEL="$INNER_MODEL"
  export META_INNER_EFFORT="$INNER_REASONING_EFFORT"
  export META_SNAPSHOT_TAG="$SNAPSHOT_TAG"

  # Capture gate baseline
  gate status --root . --json > ralph/uat/state/meta-baseline.json 2>/dev/null || true
  done_count_before=$(find done -maxdepth 1 -name 'T-205.*' 2>/dev/null | wc -l | tr -d ' ')
  export META_DONE_COUNT_BEFORE="$done_count_before"

  # --- PHASE 1: Run ONE inner iteration ---
  echo "Phase 1: Running schmuck.sh (1 task)..."
  inner_start="$(date +%s)"
  set +e
  ./schmuck.sh --max-iterations "$INNER_ITERATIONS" --model "$INNER_MODEL" --effort "$INNER_REASONING_EFFORT"
  inner_exit=$?
  set -e
  inner_end="$(date +%s)"
  inner_duration=$((inner_end - inner_start))
  done_count_after=$(find done -maxdepth 1 -name 'T-205.*' 2>/dev/null | wc -l | tr -d ' ')
  echo "Phase 1 complete: ${inner_duration}s, done ${done_count_before}→${done_count_after}, exit=${inner_exit}"
  export META_DONE_COUNT_AFTER="$done_count_after"
  export META_INNER_DURATION="$inner_duration"
  export META_INNER_EXIT="$inner_exit"

  # --- PHASE 2: Meta-agent forensics + edit (short-lived) ---
  echo "Phase 2: Meta-agent forensics..."
  set +e
  copilot -p "$(< ralph/uat/meta-prompt.md)" --yolo --model "$META_MODEL" --reasoning-effort "$META_REASONING_EFFORT" --no-custom-instructions --no-ask-user
  exit_code=$?
  set -e

  end_epoch="$(date +%s)"
  duration=$((end_epoch - start_epoch))

  # --- Post-iteration verification ---
  should_revert=false
  revert_reason=""

  # 1. Check inner-loop exit
  if [[ "$inner_exit" -ne 0 ]]; then
    should_revert=true
    revert_reason="inner loop exited with code $inner_exit"
  fi

  # 2. Check immutable files
  if ! verify_immutable_files; then
    should_revert=true
    revert_reason="immutable file modified"
  fi

  # 3. Check verdict file exists and is valid JSON
  if [[ "$should_revert" == "false" ]]; then
    if [[ ! -f "ralph/uat/state/meta-verdict.json" ]]; then
      should_revert=true
      revert_reason="no verdict file written"
    elif ! jq -e '.kept' ralph/uat/state/meta-verdict.json > /dev/null 2>&1; then
      should_revert=true
      revert_reason="verdict file malformed (missing .kept)"
    fi
  fi

  # 4. Check verdict: did meta-agent decide to keep?
  if [[ "$should_revert" == "false" ]]; then
    kept=$(jq -r '.kept' ralph/uat/state/meta-verdict.json)
    if [[ "$kept" != "true" ]]; then
      should_revert=true
      revert_reason="meta-agent chose to revert: $(jq -r '.reason // "no reason"' ralph/uat/state/meta-verdict.json)"
    fi
  fi

  # 5. Check copilot exit code
  if [[ "$should_revert" == "false" && "$exit_code" -ne 0 ]]; then
    should_revert=true
    revert_reason="copilot exited with code $exit_code"
  fi

  # --- Revert or keep ---
  if [[ "$should_revert" == "true" ]]; then
    echo "REVERTING iteration ${iteration}: ${revert_reason}"
    restore_iteration_state "$SNAPSHOT_DIR"
    rm -f ralph/uat/state/meta-verdict.json
    consecutive_reverts=$((consecutive_reverts + 1))

    if [[ $consecutive_reverts -ge 3 ]]; then
      echo "3 consecutive reverts — auto-converging."
      touch ralph/uat/state/meta-converged
    fi
  else
    echo "KEEPING iteration ${iteration}: changes accepted"
    consecutive_reverts=0
  fi

  rm -rf "$SNAPSHOT_DIR"

  echo "Meta-schmuck iteration ${iteration} complete: duration=${duration}s revert=${should_revert}"

  if [[ -f "ralph/uat/state/meta-converged" ]]; then
    echo "Meta-converged after iteration ${iteration}; exiting."
    break
  fi
done

echo ""
echo "Meta-schmuck loop complete after ${iteration} iterations."
