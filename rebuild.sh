#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

# Focused test mode: `./rebuild.sh --test [crate ...]` runs cargo test under the
# build token (so the build guard permits the compile) and leaves the daemon
# untouched. This is the only approved way to run tests; bare `cargo test` is
# rejected by the build guard. Defaults to the four primary crates.
TEST_MODE=0
TEST_CRATES=(maenifold-jockey maenifold-core maenifold-pipeline-driver maenifold-cli)
if [ "${1:-}" = "--test" ]; then
  TEST_MODE=1
  shift
  if [ "$#" -gt 0 ]; then
    TEST_CRATES=("$@")
  fi
fi

# Approved-build token. The maenifold-core/gate-core/jockey build scripts reject
# any compile that does not carry this token (see AGENTS.md > Build). Exporting it
# here is what makes the rebuild script the only approved build path.
export MAENIFOLD_REBUILD=1

if [ "$TEST_MODE" -eq 1 ]; then
  echo "═══ Focused test run (build token set; daemon untouched) ═══"
  pargs=()
  for c in "${TEST_CRATES[@]}"; do
    pargs+=(-p "$c")
  done
  echo "  cargo test ${pargs[*]}"
  if cargo test "${pargs[@]}"; then
    exit 0
  else
    exit 1
  fi
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
echo "═══ UI: build Tauri macOS DMG (npm run tauri build -- --bundles dmg --ci) ═══"
TAURI_BUNDLE_DIR="$ROOT/ui/src-tauri/target/release/bundle"
TAURI_DMG_DIR="$TAURI_BUNDLE_DIR/dmg"
rm -rf "$TAURI_DMG_DIR" "$TAURI_BUNDLE_DIR/macos"
( cd "$ROOT/ui" && npm run tauri build -- --bundles dmg --ci )
shopt -s nullglob
tauri_dmgs=("$TAURI_DMG_DIR"/*.dmg)
if [ "${#tauri_dmgs[@]}" -ne 1 ]; then
  echo "  ✗ expected exactly one DMG in $TAURI_DMG_DIR, found ${#tauri_dmgs[@]}"
  printf '    %s\n' "${tauri_dmgs[@]}"
  exit 1
fi
TAURI_DMG="${tauri_dmgs[0]}"
echo "  ✓ Tauri DMG built at $TAURI_DMG"

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
echo "═══ Install desktop app from DMG to /Applications ═══"
APP_INSTALL="/Applications/Maenifold.app"
DMG_MOUNT="$(mktemp -d "${TMPDIR:-/tmp}/maenifold-dmg.XXXXXX")"
DMG_MOUNTED=0
cleanup_dmg_mount() {
  if [ "$DMG_MOUNTED" -eq 1 ]; then
    hdiutil detach -quiet "$DMG_MOUNT" || true
  fi
  rmdir "$DMG_MOUNT" 2>/dev/null || true
}
trap cleanup_dmg_mount EXIT

if ! hdiutil attach -quiet -readonly -nobrowse -mountpoint "$DMG_MOUNT" "$TAURI_DMG"; then
  echo "  ✗ failed to mount $TAURI_DMG"
  exit 1
fi
DMG_MOUNTED=1

mounted_apps=("$DMG_MOUNT"/*.app)
if [ "${#mounted_apps[@]}" -ne 1 ]; then
  echo "  ✗ expected exactly one .app inside $TAURI_DMG, found ${#mounted_apps[@]}"
  printf '    %s\n' "${mounted_apps[@]}"
  exit 1
fi

echo "  → installing $(basename "${mounted_apps[0]}") from $TAURI_DMG"
rm -rf "$APP_INSTALL"
ditto "${mounted_apps[0]}" "$APP_INSTALL"
hdiutil detach -quiet "$DMG_MOUNT"
DMG_MOUNTED=0
rmdir "$DMG_MOUNT"
trap - EXIT

if [ ! -d "$APP_INSTALL" ]; then
  echo "  ✗ install did not produce $APP_INSTALL"
  exit 1
fi

LEGACY_APP="/Applications/maenifold.app"
if [ -L "$LEGACY_APP" ]; then
  echo "  → removing legacy build-tree symlink $LEGACY_APP"
  rm "$LEGACY_APP"
elif [ -d "$LEGACY_APP" ]; then
  legacy_bundle_id=$(plutil -extract CFBundleIdentifier raw "$LEGACY_APP/Contents/Info.plist" 2>/dev/null || true)
  if [ "$legacy_bundle_id" = "com.maenifold.ui" ]; then
    echo "  → removing legacy lowercase Maenifold bundle $LEGACY_APP"
    rm -rf "$LEGACY_APP"
  else
    echo "  ✗ $LEGACY_APP exists but is not a Maenifold bundle (CFBundleIdentifier=$legacy_bundle_id)"
    exit 1
  fi
elif [ -e "$LEGACY_APP" ]; then
  echo "  ✗ $LEGACY_APP exists but is not an app bundle or symlink"
  exit 1
fi

codesign --verify --deep --strict "$APP_INSTALL"
echo "  ✓ installed desktop app from DMG at $APP_INSTALL"

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
if [ -d "$APP_INSTALL" ]; then
  echo "  ✓ Available at $APP_INSTALL (click in Finder or run: open $APP_INSTALL)"
else
  echo "  ⚠ $APP_INSTALL unavailable"
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
