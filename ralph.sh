#!/bin/bash

set -euo pipefail

# --- Subcommand: ralph.sh watch ---
if [[ "${1:-}" == "watch" ]]; then
  shift
  OTEL_FILE="ralph/state/otel/current.jsonl"
  LOG_DIR="ralph/state/logs"
  MODE="otel"
  case "${1:-}" in
    --log)  MODE="log" ;;
    --both) MODE="both" ;;
    -h|--help)
      echo "Usage: $0 watch [--log | --both]"
      echo "  (default)  Launch otel-tui to view traces/metrics/tokens"
      echo "  --log      Tail the latest iteration log"
      echo "  --both     tmux split: otel-tui top, log tail bottom"
      exit 0 ;;
  esac
  if ! command -v otel-tui >/dev/null 2>&1; then
    echo "otel-tui not found. Install: brew install ymtdzzz/tap/otel-tui"
    exit 1
  fi
  latest_log() { ls -t "$LOG_DIR"/iter-*.log 2>/dev/null | head -1; }
  case "$MODE" in
    otel)
      # Clear stale otel-tui holding ports 4317/4318/8888
      for port in 4317 4318 8888; do
        stale_pid=$(lsof -ti :"$port" 2>/dev/null || true)
        if [[ -n "$stale_pid" ]]; then
          echo "Clearing stale process on port $port (pid $stale_pid)..."
          kill "$stale_pid" 2>/dev/null || true
        fi
      done
      sleep 1
      echo "Starting otel-tui collector on ports 4317 (gRPC) + 4318 (HTTP)..."
      echo "Run ralph.sh in another terminal — it auto-detects and sends OTLP to port 4318."
      trap 'exit 0' INT TERM
      otel-tui
      ;;
    log)
      log=$(latest_log); [[ -z "$log" ]] && { echo "No logs yet."; exit 1; }
      exec tail -f "$log" ;;
    both)
      command -v tmux >/dev/null 2>&1 || { echo "tmux required for --both"; exit 1; }
      for port in 4317 4318 8888; do
        stale_pid=$(lsof -ti :"$port" 2>/dev/null || true)
        if [[ -n "$stale_pid" ]]; then kill "$stale_pid" 2>/dev/null || true; fi
      done
      sleep 1
      # Ensure clean shutdown of tmux session on Ctrl+C
      cleanup_tmux() { tmux has-session -t ralph-watch 2>/dev/null && tmux send-keys -t ralph-watch C-c && sleep 1 && tmux kill-session -t ralph-watch 2>/dev/null; }
      trap cleanup_tmux INT TERM EXIT
      tmux new-session -d -s ralph-watch "otel-tui" \; \
        split-window -v "tail -f $(latest_log || echo /dev/null)" \; attach
      ;;
  esac
  exit 0
fi

MODEL="claude-opus-4.7-1m-internal"
REASONING_EFFORT="xhigh"
MAX_ITERATIONS=25
MAX_RUNTIME_SECONDS=0        # 0 = unlimited; e.g. 14400 = 4 hours
STUCK_TIMEOUT_SECONDS=1800    # 30 min without log output = stuck
CRATE=""
SCOPE_PATH=""
AGENT="outer-loop"
SESSION_NAME=""
NO_CUSTOM_INSTRUCTIONS=false
NO_ASK_USER=true
CONTINUE=false
NOTIFY=true
WATCH=false
EXTRA_COPILOT_ARGS=()

usage() {
  cat <<EOF
Usage: $0 [watch] [options]

Subcommands:
  watch [--log|--both]         Launch otel-tui live monitor on its own (sidecar mode)

Options:
  --watch                      Wrap this run in a tmux session with otel-tui in the
                               top pane and ralph in the bottom pane (one terminal,
                               unified live view). Requires tmux + otel-tui.
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
    --watch)            WATCH=true;                shift ;;
    --add-dir)          EXTRA_COPILOT_ARGS+=(--add-dir "${2:-}"); shift 2 ;;
    *)
      echo "Unknown argument: $1"
      usage
      exit 1
      ;;
  esac
done

# --- --watch: re-exec self inside a tmux session with otel-tui in top pane ---
if [[ "$WATCH" == "true" ]]; then
  if [[ -n "${RALPH_WATCH_INNER:-}" ]]; then
    : # already inside the tmux pane; fall through to normal run
  else
    command -v tmux >/dev/null 2>&1 || { echo "ERROR: --watch requires tmux"; exit 1; }
    command -v otel-tui >/dev/null 2>&1 || { echo "ERROR: --watch requires otel-tui (brew install ymtdzzz/tap/otel-tui)"; exit 1; }
    if [[ -n "${TMUX:-}" ]]; then
      echo "ERROR: --watch cannot run from inside an existing tmux session. Run from a plain terminal, or run \`./ralph.sh\` directly in this pane and \`./ralph.sh watch\` in another."
      exit 1
    fi
    # Clear stale OTEL collector ports
    for port in 4317 4318 8888; do
      stale_pid=$(lsof -ti :"$port" 2>/dev/null || true)
      [[ -n "$stale_pid" ]] && kill "$stale_pid" 2>/dev/null || true
    done
    sleep 1
    SCRIPT_PATH="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
    # Rebuild the inner command, dropping --watch and re-quoting every other arg.
    INNER_CMD=("$SCRIPT_PATH")
    [[ -n "$MODEL"            ]] && INNER_CMD+=(--model "$MODEL")
    [[ -n "$REASONING_EFFORT" ]] && INNER_CMD+=(--effort "$REASONING_EFFORT")
    [[ -n "$MAX_ITERATIONS"   ]] && INNER_CMD+=(--max-iterations "$MAX_ITERATIONS")
    [[ "$MAX_RUNTIME_SECONDS" -ne 0 ]]   && INNER_CMD+=(--max-runtime "$MAX_RUNTIME_SECONDS")
    [[ -n "$STUCK_TIMEOUT_SECONDS" ]]    && INNER_CMD+=(--stuck-timeout "$STUCK_TIMEOUT_SECONDS")
    [[ -n "$CRATE"            ]] && INNER_CMD+=(--crate "$CRATE")
    [[ -n "$SCOPE_PATH"       ]] && INNER_CMD+=(--path "$SCOPE_PATH")
    [[ -n "$AGENT"            ]] && INNER_CMD+=(--agent "$AGENT")
    [[ -n "$SESSION_NAME"     ]] && INNER_CMD+=(--name "$SESSION_NAME")
    [[ "$NO_CUSTOM_INSTRUCTIONS" == "true" ]] && INNER_CMD+=(--no-custom-instructions)
    [[ "$NO_ASK_USER" == "true" ]] && INNER_CMD+=(--no-ask-user)
    [[ "$NOTIFY" == "false" ]] && INNER_CMD+=(--no-notify)
    for a in "${EXTRA_COPILOT_ARGS[@]+"${EXTRA_COPILOT_ARGS[@]}"}"; do
      INNER_CMD+=("$a")
    done
    INNER_QUOTED=$(printf '%q ' "${INNER_CMD[@]}")
    cleanup_tmux() { tmux has-session -t ralph-watch 2>/dev/null && tmux kill-session -t ralph-watch 2>/dev/null; }
    trap cleanup_tmux INT TERM EXIT
    # Top pane: otel-tui collector. Bottom pane: ralph itself.
    # Sleep 2s before launching ralph so the collector binds to 4318 first.
    tmux new-session -d -s ralph-watch "otel-tui" \; \
      split-window -v "sleep 2; RALPH_WATCH_INNER=1 ${INNER_QUOTED}; echo; echo '--- ralph exited; press any key to close ---'; read -n 1" \; \
      select-pane -t 1 \; \
      attach
    exit 0
  fi
fi


LOOP_START_EPOCH="$(date +%s)"

mkdir -p ralph/state

require_command copilot
require_command jq
require_command maenifold
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

# --- Baseline snapshot / rollback helpers (T-240.4) ---

snapshot_baseline() {
  local iteration="$1"
  local baseline_dir="ralph/state/baseline/${iteration}"
  mkdir -p "$baseline_dir"

  git rev-parse HEAD > "$baseline_dir/HEAD"
  git ls-files --others --exclude-standard -z > "$baseline_dir/untracked-pre.z"
  git status --porcelain=v1 -z > "$baseline_dir/status-pre.z"
}

# Compute set difference of NUL-delimited lists: post \ pre.
# Refuses to operate (returns non-zero) if any path contains an embedded newline.
_iteration_new_untracked() {
  local pre_z="$1" post_z="$2" out="$3"
  local pre_txt post_txt
  pre_txt="$(mktemp)"
  post_txt="$(mktemp)"
  # NUL → newline. If a path itself contained a newline this would corrupt the diff,
  # so guard explicitly: count NUL records vs newline records and refuse on mismatch.
  local pre_nul pre_nl post_nul post_nl
  pre_nul=$(tr -cd '\0' < "$pre_z"  | wc -c | tr -d ' ')
  post_nul=$(tr -cd '\0' < "$post_z" | wc -c | tr -d ' ')
  tr '\0' '\n' < "$pre_z"  | LC_ALL=C sort -u > "$pre_txt"
  tr '\0' '\n' < "$post_z" | LC_ALL=C sort -u > "$post_txt"
  pre_nl=$(grep -c '' "$pre_txt"  || true)
  post_nl=$(grep -c '' "$post_txt" || true)
  # After sort -u, duplicate-empty paths collapse, so allow nl <= nul.
  if (( pre_nl > pre_nul )) || (( post_nl > post_nul )); then
    echo "ERROR: untracked path contains embedded newline; refusing to diff" >&2
    rm -f "$pre_txt" "$post_txt"
    return 2
  fi
  comm -13 "$pre_txt" "$post_txt" > "$out"
  rm -f "$pre_txt" "$post_txt"
}

validate_clean_baseline() {
  local iteration="$1"
  local baseline_dir="ralph/state/baseline/${iteration}"
  local expected_head current_head ok=true
  expected_head="$(cat "$baseline_dir/HEAD")"
  current_head="$(git rev-parse HEAD)"

  if [[ "$current_head" != "$expected_head" ]]; then
    echo "  ✗ HEAD mismatch: expected $expected_head, got $current_head"
    ok=false
  fi
  if ! git diff --quiet; then
    echo "  ✗ worktree dirty after rollback"
    ok=false
  fi
  if ! git diff --cached --quiet; then
    echo "  ✗ index dirty after rollback"
    ok=false
  fi

  local current_z residue
  current_z="$(mktemp)"
  residue="$(mktemp)"
  git ls-files --others --exclude-standard -z > "$current_z"
  if ! _iteration_new_untracked "$baseline_dir/untracked-pre.z" "$current_z" "$residue"; then
    rm -f "$current_z" "$residue"
    return 1
  fi
  if [[ -s "$residue" ]]; then
    echo "  ✗ iteration-created untracked residue still present:"
    sed 's/^/      /' "$residue"
    ok=false
  fi
  rm -f "$current_z" "$residue"

  $ok
}

perform_rollback() {
  local iteration="$1" reason="$2"
  local baseline_dir="ralph/state/baseline/${iteration}"
  local baseline_head; baseline_head="$(cat "$baseline_dir/HEAD")"
  local quarantine="ralph/state/quarantine/${iteration}-$(date +%s)"

  # Explicitly remove convergence markers — these are gitignored so rollback
  # cannot detect them as iteration residue.
  rm -f ralph/state/converged ralph/state/rollback-failed

  echo "REVERTING iteration ${iteration}: ${reason}"
  echo "  baseline HEAD: ${baseline_head}"
  notify "RALPH ⚠️" "Iteration ${iteration} reverting: ${reason}"

  # 1. Quarantine untracked files created during this iteration.
  local post_z new_untracked
  post_z="$(mktemp)"
  new_untracked="$(mktemp)"
  git ls-files --others --exclude-standard -z > "$post_z"
  if ! _iteration_new_untracked "$baseline_dir/untracked-pre.z" "$post_z" "$new_untracked"; then
    echo "  ERROR: cannot compute iteration-created untracked set"
    rm -f "$post_z" "$new_untracked"
    return 1
  fi

  if [[ -s "$new_untracked" ]]; then
    mkdir -p "$quarantine"
    local count
    count=$(grep -c '' "$new_untracked" || true)
    echo "  Quarantining ${count} iteration-created untracked file(s) → $quarantine"
    while IFS= read -r path; do
      [[ -z "$path" ]] && continue
      if [[ -e "$path" || -L "$path" ]]; then
        mkdir -p "$quarantine/$(dirname "$path")"
        mv -- "$path" "$quarantine/$path"
      fi
    done < "$new_untracked"
    cp "$new_untracked" "$quarantine/MANIFEST.txt"
    printf '%s\n' "$reason" > "$quarantine/REASON.txt"
    printf '%s\n' "$baseline_head" > "$quarantine/BASELINE_HEAD.txt"
  fi
  rm -f "$post_z" "$new_untracked"

  # 2. Hard-reset index + worktree to baseline HEAD.
  if ! git reset --hard "$baseline_head"; then
    echo "  ERROR: git reset --hard ${baseline_head} failed"
    return 1
  fi

  # 3. Validate post-conditions.
  validate_clean_baseline "$iteration"
}

print_iteration_summary() {
  local iteration="$1" outcome="$2" baseline_head="$3"
  local final_head tracked_clean staged_clean untracked_count convergence_state
  final_head="$(git rev-parse HEAD)"
  git diff --quiet         && tracked_clean="clean" || tracked_clean="DIRTY"
  git diff --cached --quiet && staged_clean="clean"  || staged_clean="DIRTY"
  untracked_count="$(git ls-files --others --exclude-standard | wc -l | tr -d ' ')"
  if   [[ -f ralph/state/rollback-failed ]]; then convergence_state="rollback-failed"
  elif [[ -f ralph/state/converged       ]]; then convergence_state="converged"
  else                                            convergence_state="running"
  fi

  cat <<EOF
─── Iteration ${iteration} summary ───
  outcome:        ${outcome}
  base commit:    ${baseline_head}
  final commit:   ${final_head}
  tracked diff:   ${tracked_clean}
  staged diff:    ${staged_clean}
  untracked:      ${untracked_count}
  convergence:    ${convergence_state}
EOF
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

  # --- Quiesce dirty workspace BEFORE snapshot ---
  # Ralph's rollback model is `git reset --hard $baseline_head`. If the
  # workspace is dirty at iteration start, capture it as a checkpoint commit
  # so the baseline includes the user's in-flight work. The next iteration's
  # rollback then preserves that work; only the iteration's own changes are
  # undone on failure.
  if ! git diff --quiet || ! git diff --cached --quiet || \
     [[ -n "$(git ls-files --others --exclude-standard)" ]]; then
    echo "Workspace not clean — capturing checkpoint commit before iteration ${iteration}."
    git add -A
    git commit -m "ralph: pre-iter-${iteration} quiesce" --no-verify
    echo "  ✓ checkpoint committed at $(git rev-parse --short HEAD)"
  fi

  # --- Snapshot pre-iteration baseline (HEAD, untracked set) ---
  snapshot_baseline "$iteration"
  baseline_head="$(cat "ralph/state/baseline/${iteration}/HEAD")"

  # Defensive double-check: snapshot_baseline must observe a clean tree.
  # If this fires, the quiesce above failed and rollback safety is broken.
  if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "ERROR: workspace still dirty after quiesce — refusing to run iteration."
    notify "RALPH ⚠️" "Iteration ${iteration} skipped: quiesce failed"
    continue
  fi

  # --- Logging, OTEL, and session capture ---
  LOG_DIR="ralph/state/logs"
  SESSION_DIR="ralph/state/sessions"
  OTEL_DIR="ralph/state/otel"
  mkdir -p "$LOG_DIR" "$SESSION_DIR" "$OTEL_DIR"
  iter_log="${LOG_DIR}/iter-${iteration}.log"
  iter_session="${SESSION_DIR}/iter-${iteration}.md"
  otel_file="${OTEL_DIR}/current.jsonl"

  # OTEL: send to otel-tui collector if running (port 4318), else file fallback
  if lsof -ti :4318 >/dev/null 2>&1; then
    export OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:4318"
    unset COPILOT_OTEL_FILE_EXPORTER_PATH 2>/dev/null || true
  else
    export COPILOT_OTEL_FILE_EXPORTER_PATH="$otel_file"
    unset OTEL_EXPORTER_OTLP_ENDPOINT 2>/dev/null || true
  fi

  # Build copilot args
  COPILOT_ARGS=(
    --prompt "$(build_copilot_prompt)"
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

  iter_status_file="ralph/state/iter-status.json"
  iter_started_iso="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

  emit_iter_status() {
    local now_epoch idle log_lines log_size head_sha
    now_epoch=$(date +%s)
    if [[ -f "$iter_log" ]]; then
      log_lines=$(wc -l < "$iter_log" 2>/dev/null | tr -d ' ' || echo 0)
      log_size=$(wc -c < "$iter_log" 2>/dev/null | tr -d ' ' || echo 0)
      local last_mod
      last_mod=$(stat -f %m "$iter_log" 2>/dev/null || stat -c %Y "$iter_log" 2>/dev/null || echo 0)
      idle=$(( now_epoch - last_mod ))
    else
      log_lines=0
      log_size=0
      idle=0
    fi
    head_sha="$(git rev-parse HEAD 2>/dev/null || echo unknown)"
    cat > "$iter_status_file" <<EOF
{
  "iteration": ${iteration},
  "max_iterations": ${MAX_ITERATIONS},
  "agent": "${AGENT}",
  "model": "${MODEL}",
  "started_at": "${iter_started_iso}",
  "elapsed_seconds": $(( now_epoch - start_epoch )),
  "log_file": "${iter_log}",
  "log_lines": ${log_lines},
  "log_size_bytes": ${log_size},
  "last_log_activity_seconds_ago": ${idle},
  "stuck_threshold_seconds": ${STUCK_TIMEOUT_SECONDS},
  "is_stuck": $( [[ "$STUCK_TIMEOUT_SECONDS" -gt 0 && "$idle" -ge "$STUCK_TIMEOUT_SECONDS" ]] && echo true || echo false ),
  "copilot_tee_pid": ${copilot_tee_pid},
  "head_sha": "${head_sha}"
}
EOF
  }

  emit_iter_status

  if [[ "$STUCK_TIMEOUT_SECONDS" -gt 0 ]]; then
    (
      while true; do
        sleep 30
        # Check if copilot is still running
        if ! ps -p $copilot_tee_pid > /dev/null 2>&1; then
          break
        fi
        emit_iter_status
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

  # Final status emit before cleanup
  emit_iter_status

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

  # 1. Check copilot exit code
  if [[ "$should_revert" == "false" && "$exit_code" -ne 0 ]]; then
    should_revert=true
    revert_reason="copilot exited with code $exit_code"
  fi

  # --- Revert or keep ---
  iteration_outcome="KEPT"
  if [[ "$should_revert" == "true" ]]; then
    iteration_outcome="REVERTED"
    if perform_rollback "$iteration" "$revert_reason"; then
      consecutive_reverts=$((consecutive_reverts + 1))
      if [[ $consecutive_reverts -ge 3 ]]; then
        if validate_clean_baseline "$iteration" >/dev/null 2>&1; then
          echo "3 consecutive clean reverts — auto-converging."
          notify "RALPH 🛑" "3 consecutive reverts — auto-converged"
          touch ralph/state/converged
        else
          echo "3 consecutive reverts but tree dirty — refusing to converge."
          rm -f ralph/state/converged
        fi
      fi
    else
      echo "ROLLBACK FAILED iteration ${iteration}: tree not restored to baseline"
      notify "RALPH 🛑" "Rollback FAILED iteration ${iteration} — manual intervention required"
      rm -f ralph/state/converged
      touch ralph/state/rollback-failed
      print_iteration_summary "$iteration" "ROLLBACK_FAILED" "$baseline_head"
      echo "Ralph iteration ${iteration} complete: duration=${duration}s revert=true rollback=failed"
      exit 2
    fi
  else
    echo "KEEPING iteration ${iteration}: changes accepted"
    notify "RALPH ✅" "Iteration ${iteration} complete (${duration}s)"
    consecutive_reverts=0
  fi

  print_iteration_summary "$iteration" "$iteration_outcome" "$baseline_head"
  echo "Ralph iteration ${iteration} complete: duration=${duration}s revert=${should_revert}"

  if [[ -f "ralph/state/converged" ]]; then
    echo "Converged marker found after iteration ${iteration}; exiting loop."
    notify "RALPH 🏁" "Converged after ${iteration} iterations"
    break
  fi
done

echo ""
if [[ -f ralph/state/rollback-failed ]]; then
  echo "Ralph loop ended with ROLLBACK FAILURE — see ralph/state/rollback-failed and ralph/state/quarantine/."
fi
echo "Ralph loop complete after ${iteration} iterations."
notify "RALPH 🏁" "Loop complete — ${iteration} iterations"
