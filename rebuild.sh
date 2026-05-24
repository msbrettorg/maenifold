#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

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
echo "═══ Desktop app ═══"
echo "  — front-end was rip'd; vanilla MCP server only. Skipping packaging."

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
echo "  — front-end was rip'd; nothing to launch."

echo ""
echo "═══ Verify ═══"
verify_failed=0
sleep 2
for port in 8080; do
  code=$(curl --connect-timeout 2 --max-time 10 -s -o /dev/null -w "%{http_code}" "http://localhost:${port}/mcp" 2>/dev/null || echo "000")
  if [ "$code" = "405" ] || [ "$code" = "406" ]; then
    echo "  ✓ localhost:${port} responding (HTTP ${code})"
  else
    echo "  ✗ localhost:${port} not responding (HTTP ${code})"
    verify_failed=1
  fi
done

echo ""
echo "═══ Verify maenifold.gate domain registration ═══"
gate_check=$(curl --connect-timeout 2 --max-time 10 -s -X POST http://127.0.0.1:8080/mcp \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"GatewayIndex","arguments":{}}}' \
  2>/dev/null | grep -c 'maenifold.gate' || true)
if [ "${gate_check:-0}" -gt 0 ]; then
  echo "  ✓ maenifold.gate domain registered"
else
  echo "  ✗ maenifold.gate domain MISSING from GatewayIndex — daemon may be running a stale binary"
  verify_failed=1
fi

echo ""
echo "Done."
exit "$verify_failed"
