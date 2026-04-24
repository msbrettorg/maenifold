#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
OTEL_DIR="${SCRIPT_DIR}/state/otel"

if ! command -v otel-tui >/dev/null 2>&1; then
  printf 'otel-tui is required but not installed.\n' >&2
  printf 'Install it with:\n  brew install ymtdzzz/tap/otel-tui\n' >&2
  exit 1
fi

jsonl_files=("${OTEL_DIR}"/*.jsonl)
if [[ ! -e "${jsonl_files[0]}" ]]; then
  printf 'No OTEL data found. Start a ralph iteration first.\n'
  exit 1
fi

latest_otel_file="$(ls -t "${jsonl_files[@]}" | head -n 1)"

if command -v tmux >/dev/null 2>&1 && [[ -n "${TMUX:-}" ]]; then
  tmux split-window -h "watch -n5 gate status --root \"$REPO_ROOT\""
fi

exec otel-tui --from-json-file "$latest_otel_file"
