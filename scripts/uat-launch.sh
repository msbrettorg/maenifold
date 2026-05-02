#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: scripts/uat-launch.sh <task-id> [port]" >&2
}

if [ "$#" -ne 1 ] && [ "$#" -ne 2 ]; then
  usage
  exit 2
fi

task_id="$1"
if [ -z "$task_id" ]; then
  usage
  exit 2
fi
if [ "$#" -eq 2 ]; then
  port="$2"
else
  port="$(printf '%s' "$task_id" | shasum | cut -c1-4 | xargs -I{} printf '%d' 0x{} | awk '{print 9200 + ($1 % 400)}')"
  echo "computed port: $port"
fi

if ! [[ "$port" =~ ^[0-9]+$ ]]; then
  echo "error: port must be numeric: $port" >&2
  exit 2
fi

safe_task_id="$(printf '%s' "$task_id" | tr -c '[:alnum:]._' '-')"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
binary="$repo_root/apps/desktop/out/maenifold-darwin-arm64/maenifold.app/Contents/MacOS/maenifold"
pkg_tmp="/tmp/mae-pkg-$safe_task_id"
user_data_dir="$pkg_tmp/user-data"
log_path="$pkg_tmp/mae-cdp-$port.log"

if [ ! -x "$binary" ]; then
  echo "error: packaged binary not found or not executable: $binary" >&2
  echo "hint: run packaging once after parallel SWEs finish: cd apps/desktop && npm run package" >&2
  exit 1
fi

mkdir -p "$user_data_dir"

pids="$(lsof -ti :"$port" 2>/dev/null || true)"
if [ -n "$pids" ]; then
  printf '%s\n' "$pids" | xargs kill -9
fi

export TMPDIR="$pkg_tmp"
nohup "$binary" \
  --remote-debugging-port="$port" \
  --remote-allow-origins='*' \
  --user-data-dir="$user_data_dir" \
  >"$log_path" 2>&1 &
pid="$!"

echo "pid: $pid"
echo "port: $port"
echo "TMPDIR: $TMPDIR"
echo "user-data-dir: $user_data_dir"
echo "log: $log_path"
