#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

UID_NUM=$(id -u)

echo "═══ Release build: all crates ═══"
cargo build --release -p maenifold-jockey -p maenifold-core -p maenifold-pipeline-driver -p maenifold-cli -p maenifold-gate

echo ""
echo "═══ Install binaries ═══"
for bin in maenifold maenifold-pipeline-driver gate; do
  src="$ROOT/target/release/$bin"
  dest="/usr/local/bin/$bin"
  if [ -f "$src" ]; then
    if [ -L "$dest" ] && [ "$(readlink "$dest")" = "$src" ]; then
      echo "  ✓ $bin symlink already correct"
    else
      sudo ln -sf "$src" "$dest"
      echo "  ✓ $bin → $dest"
    fi
  fi
done

echo ""
echo "═══ Restart launchd services ═══"
for svc in com.ma-collective.maenifold com.ma-collective.gate; do
  if launchctl list "$svc" &>/dev/null; then
    launchctl kickstart -k "gui/${UID_NUM}/${svc}" 2>/dev/null || true
    sleep 1
    launchctl start "$svc" 2>/dev/null || true
    echo "  ✓ $svc restarted"
  else
    echo "  — $svc not loaded, skipping"
  fi
done

echo ""
echo "═══ Desktop app ═══"
cd apps/desktop
npm run package
echo "  ✓ packaged"

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
echo "Done."
