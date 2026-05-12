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
pid_path="$pkg_tmp/mae-cdp-$port.pid"

if [ ! -x "$binary" ]; then
  echo "error: packaged binary not found or not executable: $binary" >&2
  echo "hint: run packaging once after parallel SWEs finish: cd apps/desktop && npm run package" >&2
  exit 1
fi

port_owner="$(lsof -nP -iTCP:"$port" -sTCP:LISTEN 2>/dev/null || true)"
if [ -n "$port_owner" ]; then
  echo "error: CDP port $port is already in use" >&2
  echo "$port_owner" >&2
  echo "kill the exact owning PID first, then re-run this script" >&2
  exit 1
fi

rm -rf "$pkg_tmp"
mkdir -p "$user_data_dir"

# T-249.192: always set MAENIFOLD_UAT for agent-driven CDP runs so the native
# folder-picker IPC short-circuits to a synthetic value instead of opening an
# OS-level modal that CDP cannot dismiss. MAENIFOLD_UAT_FOLDER_PICKER_PATH
# may be overridden by the caller; default points at a writable temp dir.
fixture_dir="${MAENIFOLD_UAT_FOLDER_PICKER_PATH:-$pkg_tmp/uat-fixture-folder}"
mkdir -p "$fixture_dir"
export MAENIFOLD_UAT=1
export MAENIFOLD_UAT_FOLDER_PICKER_PATH="$fixture_dir"

export TMPDIR="$pkg_tmp"
nohup "$binary" \
  --remote-debugging-port="$port" \
  --remote-allow-origins='*' \
  --user-data-dir="$user_data_dir" \
  >"$log_path" 2>&1 &
pid="$!"
disown
printf '%s\n' "$pid" >"$pid_path"

sleep 8
if ! ps -p "$pid" >/dev/null 2>&1; then
  echo "error: packaged app exited during CDP launch: pid $pid" >&2
  echo "log: $log_path" >&2
  tail -n 120 "$log_path" >&2 || true
  exit 1
fi

if ! grep -q "MAENIFOLD_UAT mode active" "$log_path"; then
  echo "error: MAENIFOLD_UAT guard did not activate in launched binary" >&2
  echo "the [uat] startup banner is missing from $log_path" >&2
  echo "this means the binary may pre-date T-249.192; rebuild with: cd apps/desktop && npm run package" >&2
  echo "log tail:" >&2
  tail -n 40 "$log_path" >&2 || true
  exit 1
fi

if ! curl -fsS "http://127.0.0.1:$port/json" >/tmp/mae-cdp-targets-"$port".json; then
  echo "error: CDP endpoint did not respond on port $port" >&2
  echo "pid: $pid" >&2
  echo "log: $log_path" >&2
  tail -n 120 "$log_path" >&2 || true
  exit 1
fi

echo "pid: $pid"
echo "port: $port"
echo "TMPDIR: $TMPDIR"
echo "MAENIFOLD_UAT: $MAENIFOLD_UAT"
echo "MAENIFOLD_UAT_FOLDER_PICKER_PATH: $MAENIFOLD_UAT_FOLDER_PICKER_PATH"
echo "user-data-dir: $user_data_dir"
echo "log: $log_path"
echo "pid-file: $pid_path"
echo "targets: /tmp/mae-cdp-targets-$port.json"
echo "connect: agent-browser connect $port"
