#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

# Approved-build token. The maenifold-core/gate-core/jockey build scripts reject
# any compile that does not carry this token (see AGENTS.md > Build). Exporting it
# here is what makes the rebuild script the only approved build path.
export MAENIFOLD_REBUILD=1

UID_NUM=$(id -u)

launchctl_kickstart_with_timeout() {
  local svc=$1
  local timeout_sec=${2:-60}
  ( launchctl kickstart -k "gui/${UID_NUM}/${svc}" ) &
  local pid=$!
  local waited=0
  while kill -0 "$pid" 2>/dev/null; do
    if [ "$waited" -ge "$timeout_sec" ]; then
      kill -TERM "$pid" 2>/dev/null || true
      sleep 1
      kill -KILL "$pid" 2>/dev/null || true
      echo "  ⚠ kickstart timed out after ${timeout_sec}s — falling back to bootout/bootstrap"
      return 1
    fi
    sleep 1
    waited=$((waited + 1))
  done
  wait "$pid" 2>/dev/null || true
  return 0
}

verify_mcp_port() {
  local port=$1
  local attempts=${2:-15}
  local code="000"
  local attempt
  for attempt in $(seq 1 "$attempts"); do
    code=$(curl --connect-timeout 2 --max-time 10 -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:${port}/mcp" 2>/dev/null) || code="000"
    if [ "$code" = "405" ] || [ "$code" = "406" ]; then
      echo "  ✓ localhost:${port} responding (HTTP ${code})"
      return 0
    fi
    sleep 1
  done
  echo "  ✗ localhost:${port} not responding (HTTP ${code})"
  return 1
}

verify_gate_domain() {
  local attempts=${1:-15}
  local gate_check=0
  local attempt
  for attempt in $(seq 1 "$attempts"); do
    gate_check=$(curl --connect-timeout 2 --max-time 10 -s -X POST http://127.0.0.1:8080/mcp \
      -H 'Content-Type: application/json' \
      -H 'Accept: application/json, text/event-stream' \
      -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"GatewayIndex","arguments":{}}}' \
      2>/dev/null | grep -c 'maenifold.gate' || true)
    if [ "${gate_check:-0}" -gt 0 ]; then
      echo "  ✓ maenifold.gate domain registered"
      return 0
    fi
    sleep 1
  done
  echo "  ✗ maenifold.gate domain MISSING from GatewayIndex — daemon may be running a stale binary"
  return 1
}

echo "═══ Toolchain check ═══"
for tool in node npm; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "  ✗ $tool not found on PATH — required for UI build"
    exit 1
  fi
done
echo "  ✓ node $(node --version), npm $(npm --version)"

echo ""
echo "═══ Release build: all Rust crates ═══"
make manifest
cargo build --release -p maenifold-jockey -p maenifold-core -p maenifold-pipeline-driver -p maenifold-cli

echo ""
echo "═══ Verify binaries ═══"
for bin in maenifold maenifold-pipeline-driver; do
  src="$ROOT/target/release/$bin"
  dest="/usr/local/bin/$bin"
  if [ ! -f "$src" ]; then
    echo "  ✗ $bin missing at $src — cargo build did not produce it"
    exit 1
  fi
  if [ ! -L "$dest" ]; then
    echo "  ⚠ $dest is not a symlink — skipping verify (you manage this path)"
    continue
  fi
  actual=$(readlink "$dest")
  if [ "$actual" = "$src" ]; then
    echo "  ✓ $bin symlink correct ($dest → $src)"
  else
    echo "  ⚠ $bin symlink points elsewhere: $dest → $actual"
    echo "     Expected: $src"
    echo "     This script will NOT modify /usr/local/bin (no sudo). Fix manually if needed:"
    echo "       ln -sf $src $dest"
  fi
done

echo ""
echo "═══ UI: install deps (npm ci) ═══"
( cd "$ROOT/ui" && npm ci )

echo ""
echo "═══ UI: build Tauri native app (npm run tauri build) ═══"
( cd "$ROOT/ui" && npm run tauri build )
TAURI_APP="$ROOT/ui/src-tauri/target/release/bundle/macos/Maenifold.app"
if [ ! -d "$TAURI_APP" ]; then
  echo "  ✗ Tauri build did not produce $TAURI_APP"
  exit 1
fi
echo "  ✓ Tauri app built at $TAURI_APP"

echo ""
echo "═══ UI: build web SPA (npm run build:web — for iframe embedding via spa-server) ═══"
( cd "$ROOT/ui" && npm run build:web )
SPA_DIST="$ROOT/ui/dist-web"
if [ ! -d "$SPA_DIST" ] || [ ! -f "$SPA_DIST/index.html" ]; then
  echo "  ✗ web SPA build did not produce $SPA_DIST/index.html"
  exit 1
fi
echo "  ✓ web SPA built at $SPA_DIST"

echo ""
echo "═══ Install desktop app to /Applications ═══"
APP_SYMLINK="/Applications/maenifold.app"
if [ -L "$APP_SYMLINK" ]; then
  current_target=$(readlink "$APP_SYMLINK")
  if [ "$current_target" = "$TAURI_APP" ]; then
    echo "  ✓ symlink already correct: $APP_SYMLINK → $TAURI_APP"
  else
    echo "  → replacing symlink: was → $current_target"
    rm "$APP_SYMLINK"
    ln -s "$TAURI_APP" "$APP_SYMLINK"
    echo "  ✓ symlink now: $APP_SYMLINK → $TAURI_APP"
  fi
elif [ -e "$APP_SYMLINK" ]; then
  echo "  ⚠ $APP_SYMLINK exists and is not a symlink — refusing to overwrite"
  echo "     remove or rename it manually, then rerun ./rebuild.sh"
  exit 1
else
  ln -s "$TAURI_APP" "$APP_SYMLINK"
  echo "  ✓ symlink created: $APP_SYMLINK → $TAURI_APP"
fi

echo ""
echo "═══ Kill stale maenifold processes (orphans not managed by launchd) ═══"
# Orphans escape launchd's service management (e.g. when a previous launchd
# bootstrap deatched the daemon via setsid, or the service definition was
# replaced). Once orphaned, 'launchctl kickstart' can't kill them, and the
# fresh respawn fights for port 8080 → bind() EADDRINUSE → tight crash loop.
# Per-PID cleanup BEFORE kickstart prevents this.
#
# Patterns covered:
#   'maenifold --http'           — the daemon
#   'maenifold-pipeline-driver'  — driver children (per AGENTS.md driver
#                                   process hygiene rule; old drivers
#                                   persist for in-flight pipeline runs)
killed_any=0
for pat in 'maenifold --http' 'maenifold-pipeline-driver'; do
  pids=$(pgrep -f "$pat" 2>/dev/null || true)
  if [ -n "$pids" ]; then
    for pid in $pids; do
      ppid=$(ps -o ppid= -p "$pid" 2>/dev/null | tr -d ' ' || echo '?')
      cmd=$(ps -o command= -p "$pid" 2>/dev/null | head -c 80 || echo '?')
      echo "  → SIGTERM PID $pid (ppid=$ppid): $cmd"
      kill "$pid" 2>/dev/null || true
      killed_any=1
    done
  fi
done

if [ "$killed_any" -eq 1 ]; then
  sleep 2
  # SIGKILL stragglers that ignored SIGTERM
  for pat in 'maenifold --http' 'maenifold-pipeline-driver'; do
    pids=$(pgrep -f "$pat" 2>/dev/null || true)
    if [ -n "$pids" ]; then
      for pid in $pids; do
        echo "  → SIGKILL PID $pid (didn't exit on SIGTERM)"
        kill -9 "$pid" 2>/dev/null || true
      done
    fi
  done
  sleep 1
else
  echo "  ✓ no stale maenifold processes"
fi

# Sanity: confirm port 8080 is free before handing off to launchd kickstart.
# If something else (non-maenifold) is holding it, the kickstart will still
# fail and we want to know now.
if lsof -nP -iTCP:8080 -sTCP:LISTEN >/dev/null 2>&1; then
  echo "  ⚠ port 8080 STILL held after kill — listing holders:"
  lsof -nP -iTCP:8080 -sTCP:LISTEN
  echo "     launchd kickstart will likely fail; investigate manually."
else
  echo "  ✓ port 8080 free"
fi

echo ""
echo "═══ Restart launchd services ═══"
for svc in com.ma-collective.maenifold; do
  if launchctl list "$svc" &>/dev/null; then
    if ! launchctl_kickstart_with_timeout "$svc"; then
      plist="$HOME/Library/LaunchAgents/${svc}.plist"
      if [ -f "$plist" ]; then
        launchctl bootout "gui/${UID_NUM}/${svc}" 2>/dev/null || true
        sleep 1
        launchctl bootstrap "gui/${UID_NUM}" "$plist"
        echo "  ✓ $svc bootstrapped after kickstart timeout"
      else
        echo "  ✗ $svc plist missing at $plist — cannot recover"
        exit 1
      fi
    fi
    sleep 1
    launchctl start "$svc" 2>/dev/null || true
    echo "  ✓ $svc restarted"
  else
    echo "  — $svc not loaded, skipping"
  fi
done

echo ""
echo "═══ Repoint com.maenifold.spa-server LaunchAgent at ui/dist-web ═══"
SPA_PLIST="$HOME/Library/LaunchAgents/com.maenifold.spa-server.plist"
if [ ! -f "$SPA_PLIST" ]; then
  echo "  — $SPA_PLIST not installed, skipping spa-server repoint"
else
  current_spa_path=$(plutil -extract ProgramArguments.2 raw "$SPA_PLIST" 2>/dev/null || echo "")
  if [ "$current_spa_path" = "$SPA_DIST" ]; then
    echo "  ✓ plist ProgramArguments[2] already = $SPA_DIST"
  else
    echo "  → repointing ProgramArguments[2]: $current_spa_path → $SPA_DIST"
    plutil -replace ProgramArguments.2 -string "$SPA_DIST" "$SPA_PLIST"
    launchctl bootout "gui/${UID_NUM}/com.maenifold.spa-server" 2>/dev/null || true
    sleep 1
    launchctl bootstrap "gui/${UID_NUM}" "$SPA_PLIST"
    echo "  ✓ spa-server bootstrapped with new path"
  fi
fi

echo ""
echo "═══ Launch desktop app ═══"
if [ -L "$APP_SYMLINK" ] && [ -d "$(readlink "$APP_SYMLINK")" ]; then
  echo "  ✓ Available at $APP_SYMLINK (click in Finder or run: open $APP_SYMLINK)"
else
  echo "  ⚠ $APP_SYMLINK unavailable"
fi

echo ""
echo "═══ Verify ═══"
verify_failed=0
for port in 8080; do
  if ! verify_mcp_port "$port"; then
    verify_failed=1
  fi
done

echo ""
echo "═══ Verify maenifold.gate domain registration ═══"
if ! verify_gate_domain; then
  verify_failed=1
fi

echo ""
echo "═══ Verify SPA server ═══"
if [ -f "$SPA_PLIST" ]; then
  spa_code=$(curl --connect-timeout 2 --max-time 5 -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:8090/" 2>/dev/null) || spa_code="000"
  if [ "$spa_code" = "200" ]; then
    echo "  ✓ localhost:8090 serving SPA (HTTP $spa_code)"
  else
    echo "  ✗ localhost:8090 not responding (HTTP $spa_code) — spa-server may need manual bootstrap"
    verify_failed=1
  fi
else
  echo "  — spa-server LaunchAgent not installed, skipping"
fi

echo ""
echo "Done."
exit "$verify_failed"
