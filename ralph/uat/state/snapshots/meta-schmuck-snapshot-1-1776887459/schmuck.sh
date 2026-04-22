#!/bin/bash

set -euo pipefail

MODEL="gpt-5.4"
REASONING_EFFORT="xhigh"
MAX_ITERATIONS=1
APP_BINARY="./apps/desktop/out/maenifold-darwin-arm64/maenifold.app/Contents/MacOS/maenifold"
AGENT_BROWSER_SESSION="${AGENT_BROWSER_SESSION:-schmuck-uat}"

usage() {
  echo "Usage: $0 [--model <model>] [--effort <low|medium|high|xhigh>] [--max-iterations <n>]"
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "ERROR: missing required command: $1"
    exit 1
  }
}

create_thinking_session() {
  local label="$1"
  local thinking_json session_id

  thinking_json=$(maenifold tool SequentialThinking "{\"response\":\"[[UAT]] sprint 205 ${label}\",\"thoughtNumber\":0,\"totalThoughts\":1,\"nextThoughtNeeded\":true}" --json 2>/dev/null || echo '{}')
  session_id=$(echo "$thinking_json" | jq -r '.data.sessionId // empty')
  [[ -n "$session_id" ]] || return 1
  echo "$session_id"
}

kill_cdp_listener() {
  local pid
  while IFS= read -r pid; do
    [[ -n "$pid" ]] || continue
    kill "$pid" 2>/dev/null || true
  done < <(lsof -nP -iTCP:9222 -sTCP:LISTEN -t 2>/dev/null || true)
  sleep 2
}

launch_packaged_app() {
  if [[ ! -x "$APP_BINARY" ]]; then
    echo "ERROR: packaged Electron app not found at $APP_BINARY"
    echo "Run: cd apps/desktop && npm run package"
    return 1
  fi

  "$APP_BINARY" --remote-debugging-port=9222 >/dev/null 2>&1 &
  sleep 5
}

cdp_ready() {
  local url

  agent-browser connect 9222 >/dev/null 2>&1 || return 1
  url="$(agent-browser get url 2>/dev/null || true)"
  [[ -n "$url" && "$url" != "about:blank" ]] || return 1
  agent-browser snapshot -i >/dev/null 2>&1
}

ensure_cdp_ready() {
  agent-browser skills get core >/dev/null 2>&1 || true
  agent-browser skills get electron >/dev/null 2>&1 || true

  if cdp_ready; then
    return 0
  fi

  kill_cdp_listener
  launch_packaged_app || return 1
  cdp_ready || {
    echo "ERROR: CDP preflight failed after relaunch"
    return 1
  }
}

build_copilot_prompt() {
  cat <<EOF
Runtime facts from schmuck.sh:
- CDP preflight already succeeded before this Copilot run. Only attempt recovery if a fresh \`agent-browser snapshot -i\` now fails.
- AGENT_BROWSER_SESSION=${AGENT_BROWSER_SESSION}
- SCHMUCK_THINKING_SESSION=${SCHMUCK_THINKING_SESSION:-}
- If SCHMUCK_THINKING_SESSION is set, that session already exists and already contains thought 0 created by the shell. Append exactly one follow-up thought to that existing session, mark it complete, and include a conclusion. Do NOT create a new session and do NOT reuse thought 0.

EOF
  cat ralph/uat/prompt.md
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --model)
      MODEL="${2:-}"
      shift 2
      ;;
    --effort|--reasoning-effort)
      REASONING_EFFORT="${2:-}"
      shift 2
      ;;
    --max-iterations)
      MAX_ITERATIONS="${2:-}"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1"
      usage
      exit 1
      ;;
  esac
done

require_command copilot
require_command maenifold
require_command jq
require_command gate
require_command agent-browser
PARENT_THINKING_SESSION="${SCHMUCK_THINKING_SESSION:-}"
export AGENT_BROWSER_SESSION

mkdir -p ralph/uat/state outputs/screenshots/sprint-205

for ((iteration=1; iteration<=MAX_ITERATIONS; iteration++)); do
  if [[ -f "ralph/uat/state/converged" ]]; then
    echo "Converged marker found before iteration ${iteration}; exiting loop."
    break
  fi

  if [[ -n "$PARENT_THINKING_SESSION" ]]; then
    SCHMUCK_THINKING_SESSION="$PARENT_THINKING_SESSION"
  else
    SCHMUCK_THINKING_SESSION="$(create_thinking_session "schmuck iteration ${iteration}")" || {
      echo "ERROR: Failed to create thinking session via maenifold CLI"
      exit 1
    }
    echo "Created thinking session: $SCHMUCK_THINKING_SESSION"
  fi
  export SCHMUCK_THINKING_SESSION

  start_epoch="$(date +%s)"
  echo "=== UAT iteration ${iteration}/${MAX_ITERATIONS} (model=${MODEL} effort=${REASONING_EFFORT} thinking=${SCHMUCK_THINKING_SESSION}) ==="

  ensure_cdp_ready

  set +e
  copilot -p "$(build_copilot_prompt)" --yolo --model "$MODEL" --reasoning-effort "$REASONING_EFFORT" --no-custom-instructions --no-ask-user
  exit_code=$?
  set -e

  end_epoch="$(date +%s)"
  duration=$((end_epoch - start_epoch))
  echo "Iteration ${iteration} complete: duration=${duration}s exit_code=${exit_code}"

  if [[ "$exit_code" -ne 0 ]]; then
    echo "Aborting after failed iteration ${iteration}."
    exit "$exit_code"
  fi

  if [[ -f "ralph/uat/state/converged" ]]; then
    echo "Converged marker found after iteration ${iteration}; exiting loop."
    break
  fi
done
