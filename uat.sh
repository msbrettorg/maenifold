#!/bin/bash

set -euo pipefail

MODEL="claude-opus-4.6-1m"
REASONING_EFFORT="high"
MAX_ITERATIONS=20
MAX_RUNTIME_SECONDS=0
STUCK_TIMEOUT_SECONDS=600
AGENT="product-manager"
SESSION_NAME=""
NO_CUSTOM_INSTRUCTIONS=false
NO_ASK_USER=true
NOTIFY=true
EXTRA_COPILOT_ARGS=()

IMMUTABLE_FILES=(
  "ralph.sh"
  "uat.sh"
)

usage() {
  cat <<EOF
Usage: $0 [options]

UAT agent — uses the maenifold app interactively, documents everything,
files gate items for bugs, writes USAGE.md and uat-learnings.md.

Options:
  --model <model>              Copilot model (default: gpt-5.5)
  --effort <level>             Reasoning effort: low|medium|high|xhigh (default: high)
  --max-iterations <n>         Iteration limit (default: 20)
  --max-runtime <seconds>      Wall-clock runtime cap; 0 = unlimited (default: 0)
  --stuck-timeout <seconds>    Abort if no log output for N seconds (default: 600)
  --agent <agent>              Copilot agent (default: product-manager)
  --name <name>                Session name passed to copilot
  --no-notify                  Disable macOS notifications
  --add-dir <dir>              Additional directory for copilot access (repeatable)
EOF
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "ERROR: missing required command: $1"
    exit 1
  }
}

notify() {
  local title="$1"
  local message="$2"
  if [[ "$NOTIFY" == "true" && "$(uname)" == "Darwin" ]]; then
    osascript -e "display notification \"$message\" with title \"$title\"" 2>/dev/null || true
  fi
}

create_thinking_session() {
  local label="$1"
  local thinking_json session_id

  thinking_json=$(maenifold tool SequentialThinking "{\"response\":\"[[UAT]] ${label}\",\"thoughtNumber\":0,\"totalThoughts\":1,\"nextThoughtNeeded\":true}" --json 2>/dev/null || echo '{}')
  session_id=$(echo "$thinking_json" | jq -r '.data.sessionId // empty')
  [[ -n "$session_id" ]] || return 1
  echo "$session_id"
}

build_copilot_prompt() {
  cat <<EOF
Runtime facts from uat.sh:
- UAT_THINKING_SESSION=${UAT_THINKING_SESSION:-}
- If UAT_THINKING_SESSION is set, that session already exists and already contains thought 0 created by the shell. Append exactly one follow-up thought to that existing session, mark it complete, and include a conclusion. Do NOT create a new session and do NOT reuse thought 0.

EOF
  cat ralph/uat.md
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --model)            MODEL="${2:-}";             shift 2 ;;
    --effort|--reasoning-effort)
                        REASONING_EFFORT="${2:-}";  shift 2 ;;
    --max-iterations)   MAX_ITERATIONS="${2:-}";    shift 2 ;;
    --max-runtime)      MAX_RUNTIME_SECONDS="${2:-}"; shift 2 ;;
    --stuck-timeout)    STUCK_TIMEOUT_SECONDS="${2:-}"; shift 2 ;;
    --agent)            AGENT="${2:-}";             shift 2 ;;
    --name)             SESSION_NAME="${2:-}";      shift 2 ;;
    --no-notify)        NOTIFY=false;              shift ;;
    --add-dir)          EXTRA_COPILOT_ARGS+=(--add-dir "${2:-}"); shift 2 ;;
    -h|--help)          usage; exit 0 ;;
    *)
      echo "Unknown argument: $1"
      usage
      exit 1
      ;;
  esac
done

LOOP_START_EPOCH="$(date +%s)"

mkdir -p ralph/state

require_command copilot
require_command jq
require_command maenifold
require_command gate

# --- Lockfile: prevent concurrent runs ---
LOCK="ralph/state/uat.lock"
if [ -f "$LOCK" ] && kill -0 "$(cat "$LOCK" 2>/dev/null)" 2>/dev/null; then
  echo "ERROR: uat.sh is already running (pid=$(cat "$LOCK"), lockfile=$LOCK)"
  exit 1
fi
echo $$ > "$LOCK"
trap 'rm -f "$LOCK"' EXIT

# --- Compute immutable file hashes ---
IMMUTABLE_HASH_FILE="ralph/state/uat-immutable-hashes.txt"
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
  # UAT doesn't converge — it runs until the budget runs out
  # But respect an explicit stop file
  if [[ -f "ralph/state/uat-stop" ]]; then
    echo "Stop marker found before iteration ${iteration}; exiting loop."
    rm -f "ralph/state/uat-stop"
    break
  fi

  # Thinking session
  UAT_THINKING_SESSION="$(create_thinking_session "uat iteration ${iteration}")" || {
    echo "WARNING: Failed to create thinking session, using fallback ID"
    UAT_THINKING_SESSION="uat-${iteration}-$(date +%s)"
  }
  export UAT_THINKING_SESSION
  echo "$UAT_THINKING_SESSION" > ralph/state/uat-thinking-session-id.txt

  start_epoch="$(date +%s)"
  echo ""
  echo "╔══════════════════════════════════════════════════════════════════╗"
  echo "║  UAT iteration ${iteration}/${MAX_ITERATIONS}"
  echo "║  Model: ${MODEL}  Effort: ${REASONING_EFFORT}"
  echo "║  Agent: ${AGENT}"
  echo "║  Thinking: ${UAT_THINKING_SESSION}"
  echo "║  Consecutive reverts: ${consecutive_reverts}/3"
  echo "╚══════════════════════════════════════════════════════════════════╝"
  echo ""

  # --- Max runtime check ---
  if [[ "$MAX_RUNTIME_SECONDS" -gt 0 ]]; then
    elapsed=$(( $(date +%s) - LOOP_START_EPOCH ))
    if [[ "$elapsed" -ge "$MAX_RUNTIME_SECONDS" ]]; then
      echo "MAX RUNTIME (${MAX_RUNTIME_SECONDS}s) reached after ${elapsed}s — stopping."
      notify "UAT" "Max runtime reached after ${elapsed}s"
      break
    fi
  fi

  export UAT_ITERATION="$iteration"
  export UAT_MAX_ITERATIONS="$MAX_ITERATIONS"

  # --- Logging and session capture ---
  LOG_DIR="ralph/state/logs"
  SESSION_DIR="ralph/state/sessions"
  OTEL_DIR="ralph/state/otel"
  mkdir -p "$LOG_DIR" "$SESSION_DIR" "$OTEL_DIR"
  iter_log="${LOG_DIR}/uat-iter-${iteration}.log"
  iter_session="${SESSION_DIR}/uat-iter-${iteration}.md"
  otel_file="${OTEL_DIR}/uat-current.jsonl"

  # OTEL: send to collector if running, else file fallback
  if lsof -ti :4318 >/dev/null 2>&1; then
    export OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:4318"
    unset COPILOT_OTEL_FILE_EXPORTER_PATH 2>/dev/null || true
  else
    export COPILOT_OTEL_FILE_EXPORTER_PATH="$otel_file"
    unset OTEL_EXPORTER_OTLP_ENDPOINT 2>/dev/null || true
  fi

  # Build copilot args
  COPILOT_ARGS=(
    -p "$(build_copilot_prompt)"
    --agent "$AGENT"
    --yolo
    --autopilot
    --model "$MODEL"
    --reasoning-effort "$REASONING_EFFORT"
    --enable-reasoning-summaries
    --share "$iter_session"
    --no-ask-user
  )
  [[ -n "$SESSION_NAME" ]] && COPILOT_ARGS+=(--name "${SESSION_NAME}-uat${iteration}")
  [[ "$NO_CUSTOM_INSTRUCTIONS" == "true" ]] && COPILOT_ARGS+=(--no-custom-instructions)
  COPILOT_ARGS+=("${EXTRA_COPILOT_ARGS[@]+"${EXTRA_COPILOT_ARGS[@]}"}")

  # --- Run with stuck detection ---
  set +e
  copilot "${COPILOT_ARGS[@]}" 2>&1 | tee "$iter_log" &
  copilot_tee_pid=$!

  if [[ "$STUCK_TIMEOUT_SECONDS" -gt 0 ]]; then
    (
      while true; do
        sleep 30
        if ! ps -p $copilot_tee_pid > /dev/null 2>&1; then
          break
        fi
        if [[ -f "$iter_log" ]]; then
          last_mod=$(stat -f %m "$iter_log" 2>/dev/null || stat -c %Y "$iter_log" 2>/dev/null || echo 0)
          now=$(date +%s)
          idle=$(( now - last_mod ))
          if [[ "$idle" -ge "$STUCK_TIMEOUT_SECONDS" ]]; then
            echo ""
            echo "⚠ STUCK DETECTED: no output for ${idle}s (threshold: ${STUCK_TIMEOUT_SECONDS}s)"
            kill $copilot_tee_pid 2>/dev/null || true
            break
          fi
        fi
      done
    ) &
    stuck_monitor_pid=$!
  fi

  wait $copilot_tee_pid
  exit_code=$?

  if [[ -n "${stuck_monitor_pid:-}" ]]; then
    kill $stuck_monitor_pid 2>/dev/null || true
    wait $stuck_monitor_pid 2>/dev/null || true
  fi
  set -e

  end_epoch="$(date +%s)"
  duration=$((end_epoch - start_epoch))

  # --- Post-iteration verification ---
  should_revert=false
  revert_reason=""

  if ! verify_immutable_files; then
    should_revert=true
    revert_reason="immutable file modified"
  fi

  if [[ "$should_revert" == "false" && "$exit_code" -ne 0 ]]; then
    should_revert=true
    revert_reason="copilot exited with code $exit_code"
  fi

  if [[ "$should_revert" == "true" ]]; then
    echo "REVERTING iteration ${iteration}: ${revert_reason}"
    notify "UAT ⚠️" "Iteration ${iteration} reverted: ${revert_reason}"
    git checkout -- . 2>/dev/null || true
    consecutive_reverts=$((consecutive_reverts + 1))

    if [[ $consecutive_reverts -ge 3 ]]; then
      echo "3 consecutive reverts — stopping UAT."
      notify "UAT 🛑" "3 consecutive reverts — stopped"
      break
    fi
  else
    echo "KEEPING iteration ${iteration}: changes accepted"
    notify "UAT ✅" "Iteration ${iteration} complete (${duration}s)"
    consecutive_reverts=0
  fi

  echo "UAT iteration ${iteration} complete: duration=${duration}s revert=${should_revert}"
done

echo ""
echo "UAT loop complete after ${iteration} iterations."
notify "UAT 🏁" "Loop complete — ${iteration} iterations"
