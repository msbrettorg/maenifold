#!/bin/bash

set -euo pipefail

MODEL="claude-opus-4.6-1m"
REASONING_EFFORT="xhigh"
MAX_ITERATIONS=50
MAX_RUNTIME_SECONDS=0        # 0 = unlimited; e.g. 14400 = 4 hours
STUCK_TIMEOUT_SECONDS=600    # 10 min without log output = stuck
CRATE=""
SCOPE_PATH=""
AGENT="product-manager"
SESSION_NAME=""
NO_CUSTOM_INSTRUCTIONS=false
NO_ASK_USER=false
CONTINUE=false
NOTIFY=true
EXTRA_COPILOT_ARGS=()

# Files the agent MUST NOT modify — verified by hash after each iteration
IMMUTABLE_FILES=(
  "ralph.sh"
)

usage() {
  cat <<EOF
Usage: $0 [options]

Options:
  --model <model>              Copilot model (default: claude-opus-4.6-1m)
  --effort <level>             Reasoning effort: low|medium|high|xhigh (default: xhigh)
  --max-iterations <n>         Iteration limit (default: 50)
  --max-runtime <seconds>      Wall-clock runtime cap; 0 = unlimited (default: 0)
  --stuck-timeout <seconds>    Abort if no log output for N seconds (default: 600)
  --crate <crate>              Scope to a Rust crate
  --path <path>                Scope to a filesystem path
  --agent <agent>              Copilot agent (default: product-manager)
  --name <name>                Session name passed to copilot
  --no-custom-instructions     Disable AGENTS.md / custom instructions
  --no-ask-user                Disable ask_user tool (fully autonomous)
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

# --- OS notification (macOS) ---
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

  thinking_json=$(maenifold tool SequentialThinking "{\"response\":\"[[Ralph]] ${label}\",\"thoughtNumber\":0,\"totalThoughts\":1,\"nextThoughtNeeded\":true}" --json 2>/dev/null || echo '{}')
  session_id=$(echo "$thinking_json" | jq -r '.data.sessionId // empty')
  [[ -n "$session_id" ]] || return 1
  echo "$session_id"
}

build_copilot_prompt() {
  cat <<EOF
Runtime facts from ralph.sh:
- RALPH_THINKING_SESSION=${RALPH_THINKING_SESSION:-}
- If RALPH_THINKING_SESSION is set, that session already exists and already contains thought 0 created by the shell. Append exactly one follow-up thought to that existing session, mark it complete, and include a conclusion. Do NOT create a new session and do NOT reuse thought 0.

EOF
  cat ralph/prompt.md
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --model)            MODEL="${2:-}";             shift 2 ;;
    --effort|--reasoning-effort)
                        REASONING_EFFORT="${2:-}";  shift 2 ;;
    --max-iterations)   MAX_ITERATIONS="${2:-}";    shift 2 ;;
    --max-runtime)      MAX_RUNTIME_SECONDS="${2:-}"; shift 2 ;;
    --stuck-timeout)    STUCK_TIMEOUT_SECONDS="${2:-}"; shift 2 ;;
    --crate)            CRATE="${2:-}";             shift 2 ;;
    --path)             SCOPE_PATH="${2:-}";        shift 2 ;;
    --agent)            AGENT="${2:-}";             shift 2 ;;
    --name)             SESSION_NAME="${2:-}";      shift 2 ;;
    --no-custom-instructions)
                        NO_CUSTOM_INSTRUCTIONS=true; shift ;;
    --no-ask-user)      NO_ASK_USER=true;          shift ;;
    --no-notify)        NOTIFY=false;              shift ;;
    --add-dir)          EXTRA_COPILOT_ARGS+=(--add-dir "${2:-}"); shift 2 ;;
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
require_command shasum

PARENT_THINKING_SESSION="${RALPH_THINKING_SESSION:-}"
export RALPH_TARGET_CRATE="${CRATE}"
export RALPH_TARGET_PATH="${SCOPE_PATH}"

# --- Lockfile: prevent concurrent runs (macOS-compatible) ---
LOCK="ralph/state/ralph.lock"
if [ -f "$LOCK" ] && kill -0 "$(cat "$LOCK" 2>/dev/null)" 2>/dev/null; then
  echo "ERROR: ralph.sh is already running (pid=$(cat "$LOCK"), lockfile=$LOCK)"
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
  if [[ -f "ralph/state/converged" ]]; then
    echo "Converged marker found before iteration ${iteration}; exiting loop."
    break
  fi

  # Thinking session: reuse parent or create new
  if [[ -n "$PARENT_THINKING_SESSION" ]]; then
    RALPH_THINKING_SESSION="$PARENT_THINKING_SESSION"
  else
    RALPH_THINKING_SESSION="$(create_thinking_session "iteration ${iteration}")" || {
      echo "WARNING: Failed to create thinking session via CLI, using fallback ID"
      RALPH_THINKING_SESSION="ralph-${iteration}-$(date +%s)"
    }
    echo "Created thinking session: $RALPH_THINKING_SESSION"
  fi
  export RALPH_THINKING_SESSION
  echo "$RALPH_THINKING_SESSION" > ralph/state/thinking-session-id.txt

  start_epoch="$(date +%s)"
  echo ""
  echo "╔══════════════════════════════════════════════════════════════════╗"
  echo "║  RALPH iteration ${iteration}/${MAX_ITERATIONS}"
  echo "║  Model: ${MODEL}  Effort: ${REASONING_EFFORT}"
  echo "║  Agent: ${AGENT}"
  echo "║  Crate: ${CRATE:-all}  Path: ${SCOPE_PATH:-all}"
  echo "║  Thinking: ${RALPH_THINKING_SESSION}"
  echo "║  Consecutive reverts: ${consecutive_reverts}/3"
  echo "╚══════════════════════════════════════════════════════════════════╝"
  echo ""

  # --- Max runtime check ---
  if [[ "$MAX_RUNTIME_SECONDS" -gt 0 ]]; then
    elapsed=$(( $(date +%s) - LOOP_START_EPOCH ))
    if [[ "$elapsed" -ge "$MAX_RUNTIME_SECONDS" ]]; then
      echo "MAX RUNTIME (${MAX_RUNTIME_SECONDS}s) reached after ${elapsed}s — stopping."
      notify "RALPH" "Max runtime reached after ${elapsed}s"
      break
    fi
  fi

  # Export iteration config
  export RALPH_ITERATION="$iteration"
  export RALPH_MAX_ITERATIONS="$MAX_ITERATIONS"

  # --- Logging and session capture ---
  LOG_DIR="ralph/state/logs"
  SESSION_DIR="ralph/state/sessions"
  mkdir -p "$LOG_DIR" "$SESSION_DIR"
  iter_log="${LOG_DIR}/iter-${iteration}.log"
  iter_session="${SESSION_DIR}/iter-${iteration}.md"

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
  )
  [[ -n "$SESSION_NAME" ]] && COPILOT_ARGS+=(--name "${SESSION_NAME}-iter${iteration}")
  [[ "$NO_CUSTOM_INSTRUCTIONS" == "true" ]] && COPILOT_ARGS+=(--no-custom-instructions)
  [[ "$NO_ASK_USER" == "true" ]] && COPILOT_ARGS+=(--no-ask-user)
  [[ "$CONTINUE" == "true" ]] && COPILOT_ARGS+=(--continue)
  COPILOT_ARGS+=("${EXTRA_COPILOT_ARGS[@]+"${EXTRA_COPILOT_ARGS[@]}"}")

  # --- Run with stuck detection ---
  # Background: monitor log file for activity. If no new output for
  # STUCK_TIMEOUT_SECONDS, the copilot PID is sent SIGTERM.
  set +e
  copilot "${COPILOT_ARGS[@]}" 2>&1 | tee "$iter_log" &
  copilot_tee_pid=$!

  if [[ "$STUCK_TIMEOUT_SECONDS" -gt 0 ]]; then
    (
      while true; do
        sleep 30
        # Check if copilot is still running
        if ! ps -p $copilot_tee_pid > /dev/null 2>&1; then
          break
        fi
        # Check log file modification time
        if [[ -f "$iter_log" ]]; then
          last_mod=$(stat -f %m "$iter_log" 2>/dev/null || stat -c %Y "$iter_log" 2>/dev/null || echo 0)
          now=$(date +%s)
          idle=$(( now - last_mod ))
          if [[ "$idle" -ge "$STUCK_TIMEOUT_SECONDS" ]]; then
            echo ""
            echo "⚠ STUCK DETECTED: no output for ${idle}s (threshold: ${STUCK_TIMEOUT_SECONDS}s)"
            # Send SIGTERM to the tee pipeline (which propagates to copilot)
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

  # Clean up stuck monitor if still running
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

  # 1. Check immutable files
  if ! verify_immutable_files; then
    should_revert=true
    revert_reason="immutable file modified"
  fi

  # 2. Check copilot exit code
  if [[ "$should_revert" == "false" && "$exit_code" -ne 0 ]]; then
    should_revert=true
    revert_reason="copilot exited with code $exit_code"
  fi

  # --- Revert or keep ---
  if [[ "$should_revert" == "true" ]]; then
    echo "REVERTING iteration ${iteration}: ${revert_reason}"
    git checkout -- . 2>/dev/null || true
    consecutive_reverts=$((consecutive_reverts + 1))

    if [[ $consecutive_reverts -ge 3 ]]; then
      echo "3 consecutive reverts — auto-converging."
      touch ralph/state/converged
    fi
  else
    echo "KEEPING iteration ${iteration}: changes accepted"
    consecutive_reverts=0
  fi

  echo "Ralph iteration ${iteration} complete: duration=${duration}s revert=${should_revert}"

  if [[ -f "ralph/state/converged" ]]; then
    echo "Converged marker found after iteration ${iteration}; exiting loop."
    break
  fi
done

echo ""
echo "Ralph loop complete after ${iteration} iterations."
