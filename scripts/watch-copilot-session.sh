#!/usr/bin/env bash

set -euo pipefail

DEFAULT_ROOT="${HOME}/.copilot/session-state"
ROOT="${DEFAULT_ROOT}"
SESSION_ARG=""
LINES=20
POLL_SECONDS=1
LIST_ONLY=false
LIST_LIMIT=50
LIST_ALL=false
STATE_DIR=""
SESSION_DIR=""

usage() {
  cat <<'EOF'
Usage: ./watch-copilot-session.sh [options]

Tail text files in the newest Copilot session-state directory.

Options:
  --root <dir>       Session-state root (default: ~/.copilot/session-state)
  --session <id>     Session directory name or absolute path
  --lines <n>        Initial tail backlog per file (default: 20)
  --poll <seconds>   Rescan interval for new files (default: 1)
  --list             List recent sessions newest-first with a descriptive summary
  --limit <n>        Max sessions to show with --list (default: 50)
  --all              Show all sessions with --list
  -h, --help         Show this help text

Examples:
  ./watch-copilot-session.sh
  ./watch-copilot-session.sh --lines 0
  ./watch-copilot-session.sh --session 11eb12b3-954c-43d6-9cf3-0f94d787babd
  ./watch-copilot-session.sh --root /Users/brett/.copilot/session-state --list
EOF
}

die() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

is_non_negative_integer() {
  case "${1:-}" in
    ''|*[!0-9]*)
      return 1
      ;;
    *)
      return 0
      ;;
  esac
}

mtime_for_dir() {
  local dir="$1"
  if stat -f '%m' "$dir" >/dev/null 2>&1; then
    stat -f '%m' "$dir"
  else
    stat -c '%Y' "$dir"
  fi
}

format_mtime() {
  local epoch="$1"
  if date -r "$epoch" '+%Y-%m-%d %H:%M:%S' >/dev/null 2>&1; then
    date -r "$epoch" '+%Y-%m-%d %H:%M:%S'
  else
    date -d "@$epoch" '+%Y-%m-%d %H:%M:%S'
  fi
}

trim_whitespace() {
  local text="$1"
  text="${text#"${text%%[![:space:]]*}"}"
  text="${text%"${text##*[![:space:]]}"}"
  printf '%s' "$text"
}

truncate_text() {
  local text="$1"
  local max_len="${2:-120}"
  if [[ "${#text}" -le "$max_len" ]]; then
    printf '%s' "$text"
  else
    printf '%s…' "${text:0:max_len-1}"
  fi
}

find_latest_session_dir() {
  local first_dir
  first_dir="$(list_session_dirs | head -n 1)"
  [[ -n "$first_dir" ]] || die "no session directories found under $ROOT"
  printf '%s\n' "$first_dir"
}

resolve_session_dir() {
  [[ -d "$ROOT" ]] || die "session-state root does not exist: $ROOT"

  if [[ -n "$SESSION_ARG" ]]; then
    if [[ -d "$SESSION_ARG" ]]; then
      printf '%s\n' "$SESSION_ARG"
      return 0
    fi
    if [[ -d "$ROOT/$SESSION_ARG" ]]; then
      printf '%s\n' "$ROOT/$SESSION_ARG"
      return 0
    fi
    die "session not found: $SESSION_ARG"
  fi

  find_latest_session_dir
}

discover_text_files() {
  find "$SESSION_DIR" -type f \
    \( -name '*.md' -o -name '*.txt' -o -name '*.log' -o -name '*.json' -o -name '*.jsonl' -o -name '*.yaml' -o -name '*.yml' -o -name '*.csv' -o -name '*.tsv' -o -name '*.xml' \) \
    -print | sort
}

extract_first_user_message_raw() {
  local events_file="$1"
  local first_user_line

  if ! command -v jq >/dev/null 2>&1 || [[ ! -f "$events_file" ]]; then
    return 0
  fi

  first_user_line="$(LC_ALL=C head -c 65536 "$events_file" 2>/dev/null | awk '/"type":"user.message"/ { print; exit }' || true)"
  if [[ -z "$first_user_line" ]]; then
    first_user_line="$(awk '/"type":"user.message"/ { print; exit }' "$events_file" 2>/dev/null || true)"
  fi
  [[ -n "$first_user_line" ]] || return 0

  printf '%s\n' "$first_user_line" | jq -r '.data.content // .data.transformedContent // empty' 2>/dev/null
}

summarize_user_message() {
  local raw="$1"
  local candidate summary

  candidate="$(printf '%s' "$raw" | awk '
    BEGIN { user_request_line = 0 }
    {
      gsub(/\r/, "")
      lines[NR] = $0
      if ($0 == "[User Request]") {
        user_request_line = NR
      }
    }
    END {
      start = (user_request_line > 0) ? user_request_line + 1 : 1
      for (i = start; i <= NR; i++) {
        print lines[i]
      }
    }
  ')"

  summary="$(printf '%s' "$candidate" | awk '
    {
      gsub(/\r/, "")
      line = $0
      if (line ~ /^[[:space:]]*$/) next
      if (line ~ /^<current_datetime>.*<\/current_datetime>$/) next
      if (line ~ /^<sql_tables>.*<\/sql_tables>$/) next
      if (line ~ /^<[^>]+>$/) next
      if (line ~ /^<\/[^>]+>$/) next
      if (line ~ /^\[[^]]+\]$/) next
      print line
      exit
    }
  ')"

  summary="$(trim_whitespace "$summary")"
  truncate_text "$summary" 120
}

session_summary() {
  local session_dir="$1"
  local raw summary

  raw="$(extract_first_user_message_raw "$session_dir/events.jsonl")"
  summary="$(summarize_user_message "$raw")"

  if [[ -n "$summary" ]]; then
    printf '%s' "$summary"
  else
    printf '%s' "(no user message found)"
  fi
}

print_session_listing() {
  local session_dir session_id mtime formatted_mtime summary

  while IFS= read -r session_dir; do
    [[ -n "$session_dir" ]] || continue
    session_id="$(basename "$session_dir")"
    mtime="$(mtime_for_dir "$session_dir")"
    formatted_mtime="$(format_mtime "$mtime")"
    summary="$(session_summary "$session_dir")"
    printf '%s  %s  %s\n' "$formatted_mtime" "$session_id" "$summary"
  done < <(list_session_dirs)
}

list_session_dirs() {
  local session_dirs=()
  shopt -s nullglob
  session_dirs=( "$ROOT"/*/ )
  shopt -u nullglob

  if [[ "${#session_dirs[@]}" -eq 0 ]]; then
    return 0
  fi

  if [[ "$LIST_ALL" == "true" ]]; then
    ls -td "${session_dirs[@]}" 2>/dev/null | sed 's:/$::'
  else
    ls -td "${session_dirs[@]}" 2>/dev/null | head -n "$LIST_LIMIT" | sed 's:/$::'
  fi
}

tail_key() {
  local file="$1"
  printf '%s' "$file" | cksum | awk '{print $1 "-" $2}'
}

cleanup() {
  local pid_file pid
  if [[ -n "$STATE_DIR" && -d "$STATE_DIR" ]]; then
    shopt -s nullglob
    for pid_file in "$STATE_DIR"/*.pid; do
      pid="$(cat "$pid_file" 2>/dev/null || true)"
      if [[ -n "$pid" ]]; then
        kill "$pid" 2>/dev/null || true
      fi
    done
    rm -rf "$STATE_DIR"
  fi
}

start_tailer() {
  local file="$1" rel key pid_file

  key="$(tail_key "$file")"
  pid_file="$STATE_DIR/$key.pid"
  [[ -f "$pid_file" ]] && return 0

  rel="${file#$SESSION_DIR/}"
  if [[ "$rel" == "$file" ]]; then
    rel="$(basename "$file")"
  fi

  printf '==> following %s\n' "$rel"
  (
    tail -n "$LINES" -F "$file" 2>/dev/null | while IFS= read -r line || [[ -n "$line" ]]; do
      printf '[%s] %s\n' "$rel" "$line"
    done
  ) &
  printf '%s\n' "$!" > "$pid_file"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --root)
      ROOT="${2:-}"
      shift 2
      ;;
    --session)
      SESSION_ARG="${2:-}"
      shift 2
      ;;
    --lines)
      LINES="${2:-}"
      shift 2
      ;;
    --poll)
      POLL_SECONDS="${2:-}"
      shift 2
      ;;
    --limit)
      LIST_LIMIT="${2:-}"
      shift 2
      ;;
    --all)
      LIST_ALL=true
      shift
      ;;
    --list)
      LIST_ONLY=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

is_non_negative_integer "$LINES" || die "--lines must be a non-negative integer"
is_non_negative_integer "$POLL_SECONDS" || die "--poll must be a non-negative integer"
is_non_negative_integer "$LIST_LIMIT" || die "--limit must be a non-negative integer"
[[ "$POLL_SECONDS" -ge 1 ]] || die "--poll must be at least 1 second"
[[ "$LIST_LIMIT" -ge 1 ]] || die "--limit must be at least 1"

if [[ "$LIST_ONLY" == "true" ]]; then
  if [[ -n "$SESSION_ARG" ]]; then
    SESSION_DIR="$(resolve_session_dir)"
    printf 'Session: %s\n' "$SESSION_DIR"
    printf 'Summary: %s\n' "$(session_summary "$SESSION_DIR")"
    printf '\nFiles:\n'
    discover_text_files
  else
    print_session_listing
  fi
  exit 0
fi

SESSION_DIR="$(resolve_session_dir)"

STATE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/watch-copilot-session.XXXXXX")"
trap cleanup EXIT INT TERM

printf 'Watching session: %s\n' "$SESSION_DIR"
printf 'Polling every %ss for new text files. Press Ctrl+C to stop.\n' "$POLL_SECONDS"

while true; do
  while IFS= read -r file; do
    [[ -n "$file" ]] || continue
    start_tailer "$file"
  done < <(discover_text_files)
  sleep "$POLL_SECONDS"
done
