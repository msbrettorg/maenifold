#!/bin/bash
# ralph-watch.sh — Live monitoring for RALPH loop runs
#
# Opens otel-tui pointing at the OTEL JSONL file that ralph.sh writes.
# Run this in a second terminal pane while ralph.sh runs in the first.
#
# Prerequisites:
#   brew install ymtdzzz/tap/otel-tui
#
# Usage:
#   ./ralph-watch.sh              # watch current OTEL data
#   ./ralph-watch.sh --log        # tail the log file instead
#   ./ralph-watch.sh --both       # tmux split: otel-tui top, log tail bottom

set -euo pipefail

OTEL_FILE="ralph/state/otel/current.jsonl"
LOG_FILE="ralph/state/logs"
MODE="otel"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --log)   MODE="log";  shift ;;
    --both)  MODE="both"; shift ;;
    -h|--help)
      echo "Usage: $0 [--log | --both]"
      echo "  (default)  Launch otel-tui to view traces/metrics/tokens"
      echo "  --log      Tail the latest iteration log"
      echo "  --both     tmux split: otel-tui top, log tail bottom"
      exit 0
      ;;
    *)       echo "Unknown: $1"; exit 1 ;;
  esac
done

if ! command -v otel-tui >/dev/null 2>&1; then
  echo "otel-tui not found. Install with:"
  echo "  brew install ymtdzzz/tap/otel-tui"
  exit 1
fi

latest_log() {
  ls -t "$LOG_FILE"/iter-*.log 2>/dev/null | head -1
}

case "$MODE" in
  otel)
    if [[ ! -f "$OTEL_FILE" ]]; then
      echo "No OTEL data yet at $OTEL_FILE"
      echo "Start ralph.sh first — it writes OTEL data automatically."
      echo "Waiting for file..."
      while [[ ! -f "$OTEL_FILE" ]]; do sleep 2; done
    fi
    echo "Launching otel-tui on $OTEL_FILE"
    exec otel-tui --from-json-file "$OTEL_FILE"
    ;;
  log)
    log=$(latest_log)
    if [[ -z "$log" ]]; then
      echo "No log files yet. Start ralph.sh first."
      exit 1
    fi
    echo "Tailing $log"
    exec tail -f "$log"
    ;;
  both)
    if ! command -v tmux >/dev/null 2>&1; then
      echo "tmux required for --both mode"
      exit 1
    fi
    tmux new-session -d -s ralph-watch "otel-tui --from-json-file $OTEL_FILE" \; \
      split-window -v "tail -f $(latest_log || echo /dev/null)" \; \
      attach
    ;;
esac
