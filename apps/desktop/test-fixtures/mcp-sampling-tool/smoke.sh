#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

stdout_file="${TMPDIR:-/tmp}/mcp-sampling-tool-smoke-stdout.txt"
stderr_file="${TMPDIR:-/tmp}/mcp-sampling-tool-smoke-stderr.txt"
rm -f "$stdout_file" "$stderr_file"

/opt/homebrew/bin/npx tsx toolWithSampleServer.ts \
  < /dev/null > "$stdout_file" 2> "$stderr_file" &
pid=$!

sleep 3
kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true

if [ -s "$stdout_file" ]; then
  echo "stdout must be empty for STDIO MCP fixtures" >&2
  sed -n '1,20p' "$stdout_file" >&2
  exit 1
fi

grep -q 'MCP server is running...' "$stderr_file"
echo "ok: stdout empty; startup log is on stderr"
