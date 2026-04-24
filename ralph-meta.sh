#!/bin/bash
#
# meta.sh — AutoAgent-style harness self-optimization for maenifold
#
# Outer loop that optimizes the RALPH harness (workflows, roles, prompts,
# model assignments) by running RALPH inner loops and measuring gate outcomes.
#
# Guards:
#   - Lockfile prevents concurrent runs (macOS-compatible PID-based)
#   - Revert uses git checkout (no file-based snapshots)
#   - Verdict file (meta-verdict.json) is the structured contract — shell reverts
#     if missing, malformed, or kept=false
#   - Immutable file hashes verified after each iteration
#   - set -euo pipefail for fail-fast
#   - Sequential thinking session per iteration for traceability
#
# Usage:
#   ./meta.sh [--model <model>] [--effort <low|medium|high|xhigh>]
#             [--max-iterations <n>] [--inner-iterations <n>]
#             [--inner-model <model>] [--inner-effort <low|medium|high|xhigh>]
#             [--crate <crate>] [--path <path>]
#

set -euo pipefail

META_MODEL="claude-opus-4.7"
META_REASONING_EFFORT="xhigh"
MAX_ITERATIONS=2
INNER_ITERATIONS=1
INNER_MODEL="claude-opus-4.6-1m"
INNER_REASONING_EFFORT="xhigh"
CRATE=""
SCOPE_PATH=""
META_AGENT="product-manager"
INNER_AGENT="product-manager"
SESSION_NAME=""
NO_CUSTOM_INSTRUCTIONS=false
NO_ASK_USER=true
REASONING_SUMMARIES=true
EXTRA_COPILOT_ARGS=()

OTEL_DIR="ralph/state/otel"
METRICS_LOG="ralph/state/metrics.jsonl"

# Files the meta-agent MUST NOT modify — verified by hash after each iteration
IMMUTABLE_FILES=(
  "ralph/meta-prompt.md"
)

usage() {
  cat <<EOF
Usage: $0 [options]

Options:
  --model <model>              Meta-agent model (default: claude-opus-4.7)
  --effort <level>             Meta reasoning effort: low|medium|high|xhigh (default: xhigh)
  --max-iterations <n>         Outer iteration limit (default: 10)
  --inner-iterations <n>       Inner RALPH iterations per meta iteration (default: 20)
  --inner-model <model>        Inner RALPH model (default: claude-opus-4.6-1m)
  --inner-effort <level>       Inner reasoning effort (default: xhigh)
  --crate <crate>              Scope to a Rust crate
  --path <path>                Scope to a filesystem path
  --agent <agent>              Meta-agent copilot agent (default: product-manager)
  --inner-agent <agent>        Inner RALPH copilot agent (default: product-manager)
  --name <name>                Session name prefix
  --no-custom-instructions     Disable AGENTS.md / custom instructions
  --no-ask-user                Disable ask_user tool
  --enable-reasoning-summaries  Show reasoning summaries (OpenAI models)
  --add-dir <dir>              Additional directory for copilot access (repeatable)
EOF
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "ERROR: missing required command: $1"
    exit 1
  }
}

count_todo() {
  find todo -maxdepth 1 -name 'T-*' 2>/dev/null | wc -l | tr -d ' '
}

count_done() {
  find done -maxdepth 1 -name 'T-*' 2>/dev/null | wc -l | tr -d ' '
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --model)            META_MODEL="${2:-}";              shift 2 ;;
    --effort|--reasoning-effort)
                        META_REASONING_EFFORT="${2:-}";   shift 2 ;;
    --max-iterations)   MAX_ITERATIONS="${2:-}";          shift 2 ;;
    --inner-iterations) INNER_ITERATIONS="${2:-}";        shift 2 ;;
    --inner-model)      INNER_MODEL="${2:-}";             shift 2 ;;
    --inner-effort|--inner-reasoning-effort)
                        INNER_REASONING_EFFORT="${2:-}";  shift 2 ;;
    --crate)            CRATE="${2:-}";                   shift 2 ;;
    --path)             SCOPE_PATH="${2:-}";              shift 2 ;;
    --agent)            META_AGENT="${2:-}";              shift 2 ;;
    --inner-agent)      INNER_AGENT="${2:-}";             shift 2 ;;
    --name)             SESSION_NAME="${2:-}";            shift 2 ;;
    --no-custom-instructions)
                        NO_CUSTOM_INSTRUCTIONS=true;     shift ;;
    --no-ask-user)      NO_ASK_USER=true;                shift ;;
    --enable-reasoning-summaries)
                        REASONING_SUMMARIES=true;        shift ;;
    --no-reasoning-summaries)
                        REASONING_SUMMARIES=false;       shift ;;
    --add-dir)          EXTRA_COPILOT_ARGS+=(--add-dir "${2:-}"); shift 2 ;;
    *)
      echo "Unknown argument: $1"
      usage
      exit 1
      ;;
  esac
done

mkdir -p ralph/state "$OTEL_DIR"

require_command copilot
require_command jq
require_command maenifold
require_command gate
require_command shasum

# --- Lockfile: prevent concurrent meta runs (macOS-compatible) ---
LOCK="ralph/state/meta.lock"
if [ -f "$LOCK" ] && kill -0 "$(cat "$LOCK" 2>/dev/null)" 2>/dev/null; then
  echo "ERROR: meta.sh is already running (pid=$(cat "$LOCK"), lockfile=$LOCK)"
  exit 1
fi
echo $$ > "$LOCK"
trap 'rm -f "$LOCK"' EXIT

# --- Compute immutable file hashes ---
IMMUTABLE_HASH_FILE="ralph/state/immutable-hashes.txt"
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
  if [[ -f "ralph/state/meta-converged" ]]; then
    echo "Meta-converged marker found before iteration ${iteration}; exiting."
    break
  fi

  start_epoch="$(date +%s)"
  echo ""
  echo "╔══════════════════════════════════════════════════════════════════╗"
  echo "║  META iteration ${iteration}/${MAX_ITERATIONS}"
  echo "║  Meta:  model=${META_MODEL} effort=${META_REASONING_EFFORT} agent=${META_AGENT}"
  echo "║  Inner: ${INNER_ITERATIONS} RALPH iterations  model=${INNER_MODEL} effort=${INNER_REASONING_EFFORT} agent=${INNER_AGENT}"
  echo "║  Gate:  todo=$(count_todo) done=$(count_done)"
  echo "║  Scope: crate=${CRATE:-all}  path=${SCOPE_PATH:-all}"
  echo "║  Consecutive reverts: ${consecutive_reverts}/3"
  echo "╚══════════════════════════════════════════════════════════════════╝"
  echo ""

  otel_file="${OTEL_DIR}/iter-${iteration}.jsonl"
  todo_before=$(count_todo)
  done_before=$(count_done)

  # Clear inner convergence marker and previous verdict
  rm -f ralph/state/converged
  rm -f ralph/state/meta-verdict.json
  rm -f ralph/state/thinking-session-id.txt

  # Create a sequential-thinking session for this iteration via maenifold CLI
  THINKING_JSON=$(maenifold tool SequentialThinking '{"response":"[[Ralph]] meta-iteration '"${iteration}"'","thoughtNumber":0,"totalThoughts":1,"nextThoughtNeeded":true}' --json 2>/dev/null || echo '{}')
  THINKING_SESSION=$(echo "$THINKING_JSON" | jq -r '.data.sessionId // empty')
  if [[ -z "$THINKING_SESSION" ]]; then
    echo "WARNING: Failed to create thinking session via CLI, using fallback ID"
    THINKING_SESSION="meta-${iteration}-$(date +%s)"
  fi
  echo "$THINKING_SESSION" > ralph/state/thinking-session-id.txt
  export RALPH_THINKING_SESSION="$THINKING_SESSION"
  echo "Thinking session: $THINKING_SESSION"

  # Export inner-loop config for the meta-prompt to reference
  export META_ITERATION="$iteration"
  export META_MAX_ITERATIONS="$MAX_ITERATIONS"
  export META_INNER_ITERATIONS="$INNER_ITERATIONS"
  export META_INNER_MODEL="$INNER_MODEL"
  export META_INNER_EFFORT="$INNER_REASONING_EFFORT"
  export RALPH_TARGET_CRATE="$CRATE"
  export RALPH_TARGET_PATH="$SCOPE_PATH"

  # Capture gate baseline
  gate status --root . --json > ralph/state/meta-baseline.json 2>/dev/null || true
  done_count_before=$(find done -maxdepth 1 -type f 2>/dev/null | wc -l | tr -d ' ')
  export META_DONE_COUNT_BEFORE="$done_count_before"

  # --- PHASE 1: Run inner RALPH loop ---
  echo "Phase 1: Running ralph.sh (${INNER_ITERATIONS} iterations)..."
  INNER_ARGS=(
    --max-iterations "$INNER_ITERATIONS"
    --model "$INNER_MODEL"
    --effort "$INNER_REASONING_EFFORT"
    --agent "$INNER_AGENT"
  )
  [[ -n "$CRATE" ]] && INNER_ARGS+=(--crate "$CRATE")
  [[ -n "$SCOPE_PATH" ]] && INNER_ARGS+=(--path "$SCOPE_PATH")
  [[ -n "$SESSION_NAME" ]] && INNER_ARGS+=(--name "${SESSION_NAME}-inner")
  [[ "$NO_CUSTOM_INSTRUCTIONS" == "true" ]] && INNER_ARGS+=(--no-custom-instructions)
  [[ "$NO_ASK_USER" == "true" ]] && INNER_ARGS+=(--no-ask-user)
  for arg in "${EXTRA_COPILOT_ARGS[@]+"${EXTRA_COPILOT_ARGS[@]}"}"; do
    INNER_ARGS+=("$arg")
  done

  inner_start="$(date +%s)"
  set +e
  COPILOT_OTEL_FILE_EXPORTER_PATH="$otel_file" \
    ./ralph.sh "${INNER_ARGS[@]}"
  inner_exit=$?
  set -e
  inner_end="$(date +%s)"
  inner_duration=$((inner_end - inner_start))
  done_after=$(count_done)
  todo_after=$(count_todo)
  tasks_completed=$((done_after - done_before))
  echo "Phase 1 complete: ${inner_duration}s, exit=${inner_exit}"
  echo "  Gate delta: todo ${todo_before}→${todo_after}, done ${done_before}→${done_after} (+${tasks_completed})"
  export META_DONE_COUNT_BEFORE="$done_before"
  export META_DONE_COUNT_AFTER="$done_after"
  export META_INNER_DURATION="$inner_duration"
  export META_INNER_EXIT="$inner_exit"

  # --- Extract OTEL metrics ---
  otel_tokens=0
  otel_tool_calls=0
  otel_turns=0
  if [[ -f "$otel_file" ]]; then
    otel_tokens=$(jq -r 'select(.resourceSpans) | .resourceSpans[]?.scopeSpans[]?.spans[]?.attributes[]? | select(.key == "gen_ai.usage.output_tokens" or .key == "gen_ai.usage.input_tokens") | .value.intValue // 0' "$otel_file" 2>/dev/null | paste -sd+ - | bc 2>/dev/null || echo 0)
    otel_tool_calls=$(jq -r 'select(.resourceSpans) | [.resourceSpans[]?.scopeSpans[]?.spans[]? | select(.name | startswith("execute_tool"))] | length' "$otel_file" 2>/dev/null | paste -sd+ - | bc 2>/dev/null || echo 0)
    otel_turns=$(jq -r 'select(.resourceSpans) | [.resourceSpans[]?.scopeSpans[]?.spans[]? | select(.name | startswith("chat "))] | length' "$otel_file" 2>/dev/null | paste -sd+ - | bc 2>/dev/null || echo 0)
  fi
  echo "  OTEL: tokens=${otel_tokens} tools=${otel_tool_calls} turns=${otel_turns}"

  # --- PHASE 2: Meta-agent forensics + edit (short-lived) ---
  echo "Phase 2: Meta-agent forensics..."
  META_COPILOT_ARGS=(
    -p "$(cat ralph/meta-prompt.md)"
    --agent "$META_AGENT"
    --yolo
    --model "$META_MODEL"
    --reasoning-effort "$META_REASONING_EFFORT"
    --no-custom-instructions
    --no-ask-user
  )
  [[ -n "$SESSION_NAME" ]] && META_COPILOT_ARGS+=(--name "${SESSION_NAME}-meta${iteration}")
  [[ "$REASONING_SUMMARIES" == "true" ]] && META_COPILOT_ARGS+=(--enable-reasoning-summaries)
  META_COPILOT_ARGS+=("${EXTRA_COPILOT_ARGS[@]+"${EXTRA_COPILOT_ARGS[@]}"}")

  set +e
  copilot "${META_COPILOT_ARGS[@]}"
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
    if [[ ! -f "ralph/state/meta-verdict.json" ]]; then
      should_revert=true
      revert_reason="no verdict file written"
    elif ! jq -e '.kept' ralph/state/meta-verdict.json > /dev/null 2>&1; then
      should_revert=true
      revert_reason="verdict file malformed (missing .kept)"
    fi
  fi

  # 4. Check verdict: did meta-agent decide to keep?
  if [[ "$should_revert" == "false" ]]; then
    kept=$(jq -r '.kept' ralph/state/meta-verdict.json)
    if [[ "$kept" != "true" ]]; then
      should_revert=true
      revert_reason="meta-agent chose to revert: $(jq -r '.reason // "no reason"' ralph/state/meta-verdict.json)"
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
    git checkout -- . 2>/dev/null || true
    rm -f ralph/state/meta-verdict.json
    consecutive_reverts=$((consecutive_reverts + 1))

    if [[ $consecutive_reverts -ge 3 ]]; then
      echo "3 consecutive reverts — auto-converging."
      touch ralph/state/meta-converged
    fi
  else
    echo "KEEPING iteration ${iteration}: harness change accepted"
    consecutive_reverts=0
  fi

  # --- Record metrics ---
  meta_end="$(date +%s)"
  meta_duration=$((meta_end - meta_start))

  jq -n \
    --argjson iter "$iteration" \
    --argjson inner_exit "$inner_exit" \
    --argjson inner_dur "$inner_duration" \
    --argjson tasks "$tasks_completed" \
    --argjson todo_before "$todo_before" \
    --argjson todo_after "$todo_after" \
    --argjson done_before "$done_before" \
    --argjson done_after "$done_after" \
    --argjson tokens "$otel_tokens" \
    --argjson tools "$otel_tool_calls" \
    --argjson turns "$otel_turns" \
    --argjson meta_dur "$meta_duration" \
    --arg reverted "$should_revert" \
    --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    '{
      timestamp: $ts,
      iteration: $iter,
      reverted: ($reverted == "true"),
      inner_exit: $inner_exit,
      inner_duration_s: $inner_dur,
      meta_duration_s: $meta_dur,
      tasks_completed: $tasks,
      todo: { before: $todo_before, after: $todo_after },
      done: { before: $done_before, after: $done_after },
      otel: { total_tokens: $tokens, tool_calls: $tools, llm_turns: $turns }
    }' >> "$METRICS_LOG"

  # Clean up OTEL file (metrics extracted)
  rm -f "$otel_file"

  echo "Meta iteration ${iteration} complete: ${meta_duration}s revert=${should_revert}"
  echo "────────────────────────────────────────────────────────────────────"

  if [[ -f "ralph/state/meta-converged" ]]; then
    echo "Meta-converged after iteration ${iteration}; exiting."
    break
  fi
done

echo ""
echo "╔══════════════════════════════════════════════════════════════════╗"
echo "║  META COMPLETE — ${iteration} iterations"
echo "║  Gate: todo=$(count_todo) done=$(count_done)"
echo "╚══════════════════════════════════════════════════════════════════╝"

if [[ -f "$METRICS_LOG" ]]; then
  echo ""
  echo "Iteration metrics:"
  jq -r '"\(.iteration): +\(.tasks_completed) tasks, \(.otel.total_tokens) tokens, \(.inner_duration_s)s \(if .reverted then "[REVERTED]" else "" end)"' "$METRICS_LOG" 2>/dev/null || true
  echo ""
  total_tasks=$(jq -s '[.[].tasks_completed] | add // 0' "$METRICS_LOG" 2>/dev/null || echo 0)
  total_tokens=$(jq -s '[.[].otel.total_tokens] | add // 0' "$METRICS_LOG" 2>/dev/null || echo 0)
  total_duration=$(jq -s '[.[].inner_duration_s] | add // 0' "$METRICS_LOG" 2>/dev/null || echo 0)
  echo "Totals: ${total_tasks} tasks, ${total_tokens} tokens, ${total_duration}s"
fi
