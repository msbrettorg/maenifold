#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

DESKTOP_BIN="$ROOT/apps/desktop/node_modules/.bin"
# Interrupted npm installs can leave .bin present but empty, breaking package binaries.
if [ ! -d "$DESKTOP_BIN" ] || [ -z "$(find "$DESKTOP_BIN" -mindepth 1 -maxdepth 1 -print -quit)" ]; then
  echo "═══ Desktop dependencies ═══"
  echo "  .bin missing or empty after an interrupted install — running npm install"
  (cd "$ROOT/apps/desktop" && npm install)
  echo ""
fi

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

echo "═══ Release build: all crates ═══"
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
echo "═══ Desktop app ═══"
cd apps/desktop
npm run package
echo "  ✓ packaged"
cd "$ROOT"

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
echo "═══ Launch desktop app ═══"
cd apps/desktop
if pgrep -f 'maenifold.app' &>/dev/null; then
  osascript -e 'tell application "maenifold" to quit' 2>/dev/null || true
  sleep 2
fi
open out/maenifold-darwin-arm64/maenifold.app
echo "  ✓ launched"

echo ""
echo "═══ Verify ═══"
sleep 2
for port in 8080 8081; do
  code=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:${port}/mcp" 2>/dev/null || echo "000")
  if [ "$code" = "405" ] || [ "$code" = "406" ]; then
    echo "  ✓ localhost:${port} responding (HTTP ${code})"
  else
    echo "  ✗ localhost:${port} not responding (HTTP ${code})"
  fi
done

echo ""
echo "═══ Verify maenifold.gate domain registration ═══"
gate_check=$(curl -s -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"GatewayIndex","arguments":{}}}' \
  2>/dev/null | grep -c 'maenifold.gate' || true)
if [ "${gate_check:-0}" -gt 0 ]; then
  echo "  ✓ maenifold.gate domain registered"
else
  echo "  ✗ maenifold.gate domain MISSING from GatewayIndex — daemon may be running a stale binary"
fi

echo ""
echo "Done."
