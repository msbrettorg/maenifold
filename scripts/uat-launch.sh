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
pkg_tmp="$repo_root/apps/desktop/uat-logs/$safe_task_id/runtime"
user_data_dir="$pkg_tmp/user-data"
log_path="$pkg_tmp/mae-cdp-$port.log"
pid_path="$pkg_tmp/mae-cdp-$port.pid"
targets_path="$pkg_tmp/mae-cdp-targets-$port.json"

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
# T-244.150 (post-mortem): launch the packaged binary in its OWN
# session/process group via setsid(2). The previous `nohup ... & disown`
# pattern kept the Electron app in the launching shell's process group, so
# when that shell exited (every short-lived bash call from an agent) macOS
# delivered group signals to the Electron helpers and the renderer self-
# destructed ~5s later with "Renderer process gone: killed 15" + "GPU killed"
# + ERR_FAILED loading dist/index.html.
#
# `nohup` only ignores SIGHUP; it does NOT detach from the process group.
# `setsid(8)` ships with Linux util-linux but is NOT on macOS by default, so
# we fall back to a Python `os.setsid` double-fork — POSIX-portable and
# present on every macOS install. The result is identical: child has a
# different PGID from the parent shell and survives the shell's exit.
#
# Capture the spawned PID by reading lsof on the CDP port a few moments
# later (the script's own $! captures the wrapper, not the grandchild).
launch_detached() {
  if command -v setsid >/dev/null 2>&1; then
    setsid "$binary" \
      --remote-debugging-port="$port" \
      --remote-allow-origins='*' \
      --user-data-dir="$user_data_dir" \
      >"$log_path" 2>&1 &
    disown || true
  else
    python3 -c "
import os, sys
pid = os.fork()
if pid > 0:
    sys.exit(0)
os.setsid()
pid2 = os.fork()
if pid2 > 0:
    sys.exit(0)
fd = os.open('$log_path', os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o644)
os.dup2(fd, 1)
os.dup2(fd, 2)
os.close(0)
os.execv('$binary', ['$binary', '--remote-debugging-port=$port', '--remote-allow-origins=*', '--user-data-dir=$user_data_dir'])
" || {
      echo "error: python3 detached-launch helper failed; install util-linux setsid or python3" >&2
      exit 1
    }
  fi
}
launch_detached

# Wait for the port to come up so we know the grandchild is alive.
deadline=$(( $(date +%s) + 12 ))
pid=""
while [ "$(date +%s)" -lt "$deadline" ]; do
  # `set -o pipefail` plus `head -n 1` closing its stdin can SIGPIPE the
  # preceding processes and abort the script, so wrap the pipeline so its
  # exit status is always 0 and we keep polling until the port really binds.
  pid="$(lsof -nP -iTCP:"$port" -sTCP:LISTEN -F p 2>/dev/null | awk 'NR==1{sub(/^p/,""); print; exit}' || true)"
  if [ -n "$pid" ]; then break; fi
  sleep 0.5
done
if [ -z "$pid" ] || ! ps -p "$pid" >/dev/null 2>&1; then
  echo "error: packaged app did not bind CDP port $port within 12s" >&2
  echo "log: $log_path" >&2
  tail -n 120 "$log_path" >&2 || true
  exit 1
fi
printf '%s\n' "$pid" >"$pid_path"

# Confirm the spawned process is in its own session (different PGID from us).
# If the PGIDs match, the detach failed and the renderer will die when the
# caller's shell exits — fail loudly rather than silently shipping a broken
# UAT session.
parent_pgid="$(ps -o pgid= -p $$ | tr -d ' ')"
child_pgid="$(ps -o pgid= -p "$pid" | tr -d ' ')"
if [ "$parent_pgid" = "$child_pgid" ]; then
  echo "error: detached-launch did not give the binary its own process group" >&2
  echo "  parent PGID=$parent_pgid child PGID=$child_pgid" >&2
  echo "  the renderer will die when this shell exits — investigate setsid/python3 setup" >&2
  kill "$pid" 2>/dev/null || true
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

if ! curl -fsS "http://127.0.0.1:$port/json" >"$targets_path"; then
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
echo "targets: $targets_path"
echo "connect: agent-browser connect $port"
