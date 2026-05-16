#!/usr/bin/env bash
#
# deploy-daemon.sh - Rebuild and redeploy the maenifold daemon + pipeline driver
#
# Purpose:
#   Wrap the manual daemon redeploy procedure so both required binaries are rebuilt
#   and stale pipeline-driver processes are cleaned up after the daemon restarts.
#
# Usage:
#   ralph/scripts/deploy-daemon.sh [--marker <string>] [--skip-build]
#
# What it does:
#   Builds both release binaries, verifies the new daemon binary, restarts the
#   LaunchAgent, kills stale driver PIDs one at a time, and verifies HTTP.
#
# Exit codes:
#   0 = deploy complete
#   2 = running daemon was not found
#   3 = cargo release build failed
#   4 = rebuilt daemon binary inode matches running daemon text segment
#   5 = launchd plist path could not be discovered
#   6 = launchctl bootstrap failed
#   7 = restarted daemon PID was missing or unchanged
#   8 = final HTTP verification failed
set -euo pipefail
LABEL="com.ma-collective.maenifold"
DAEMON_PATTERN="maenifold --http"
DRIVER_PATTERN="maenifold-pipeline-driver"
info() {
  printf '[deploy-daemon] %s\n' "$*"
}
warn() {
  printf '[deploy-daemon] WARN: %s\n' "$*" >&2
}
error() {
  printf '[deploy-daemon] ERROR: %s\n' "$*" >&2
}
resolve_repo_root() {
  local common_git_dir
  common_git_dir="$(git rev-parse --git-common-dir)"
  case "${common_git_dir}" in
    /*) ;;
    *) common_git_dir="$(cd "$(git rev-parse --show-toplevel)" && cd "${common_git_dir}" && pwd)" ;;
  esac
  cd "$(dirname "${common_git_dir}")" && pwd
}
first_daemon_pid() {
  pgrep -f "${DAEMON_PATTERN}" | head -1 || true
}
inode_for_path() {
  local path="$1"
  stat -f %i "${path}"
}
usage() {
  error "usage: $0 [--marker <string>] [--skip-build]"
}
MARKER_OVERRIDE=""
SKIP_BUILD=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --marker)
      [[ $# -ge 2 ]] || { usage; exit 1; }
      MARKER_OVERRIDE="$2"; shift 2 ;;
    --skip-build)
      SKIP_BUILD=1; shift ;;
    *)
      usage; exit 1 ;;
  esac
done
REPO_ROOT="$(resolve_repo_root)"
NEW_BINARY="${REPO_ROOT}/target/release/maenifold"
TOKEN_FILE="${HOME}/maenifold/auth/localhost.token"
UID_NUM="$(id -u)"
GUI_DOMAIN="gui/${UID_NUM}"
info "Repo root: ${REPO_ROOT}"
DAEMON_PID="$(first_daemon_pid)"
if [[ -z "${DAEMON_PID}" ]]; then
  error "no running daemon found with pattern: ${DAEMON_PATTERN}"
  error "start maenifold first, then rerun this script."
  exit 2
fi
info "Found running daemon PID=${DAEMON_PID}"
if [[ "${SKIP_BUILD}" -eq 1 ]]; then
  warn "--skip-build set; using existing release binaries"
else
  info "Building release binaries: maenifold-cli and maenifold-pipeline-driver"
  if ! build_output="$(
    cd "${REPO_ROOT}" &&
      cargo build --release -p maenifold-cli 2>&1 &&
      cargo build --release -p maenifold-pipeline-driver 2>&1
  )"; then
    error "cargo release build failed"
    printf '%s\n' "${build_output}" >&2
    exit 3
  fi
  info "Release build complete"
fi
if [[ ! -x "${NEW_BINARY}" ]]; then
  error "expected rebuilt daemon binary is missing or not executable: ${NEW_BINARY}"
  exit 3
fi
running_binary="$(lsof -p "${DAEMON_PID}" | awk '$4 == "txt" { print $9; exit }' || true)"
if [[ -z "${running_binary}" ]]; then
  error "could not resolve running daemon text-segment path via lsof for PID=${DAEMON_PID}"
  exit 4
fi
if [[ "${running_binary}" != *maenifold* ]]; then
  error "lsof returned unexpected binary path: ${running_binary} (expected 'maenifold' substring)"
  error "  This may indicate macOS lsof changed its txt-entry ordering."
  exit 4
fi
running_inode="$(lsof -p "${DAEMON_PID}" | awk '$4 == "txt" && $9 ~ /maenifold$/ { print $8; exit }')"
if [[ -z "${running_inode}" ]]; then
  error "could not extract running daemon inode from lsof NODE column for PID=${DAEMON_PID}"
  exit 4
fi
new_inode="$(inode_for_path "${NEW_BINARY}")"
info "Running daemon binary: ${running_binary} (lsof NODE inode=${running_inode})"
info "Rebuilt daemon binary: ${NEW_BINARY} (inode=${new_inode})"
if [[ "${running_inode}" == "${new_inode}" ]]; then
  error "rebuilt daemon binary disk inode matches running daemon lsof NODE inode"
  error "this suggests no new daemon binary was produced; refusing to bounce launchd."
  exit 4
fi
if [[ -n "${MARKER_OVERRIDE}" ]]; then
  marker="${MARKER_OVERRIDE}"
else
  marker="$(
    git -C "${REPO_ROOT}" log -1 --pretty='%s' |
      grep -Eo 'T-[0-9]+(\.[0-9]+)+' |
      head -1 || true
  )"
fi
if [[ -z "${marker}" ]]; then
  warn "no task-id marker found in last commit subject; skipping binary marker check"
else
  marker_count="$(LC_ALL=C grep -ac -- "${marker}" "${NEW_BINARY}" || true)"
  if [[ "${marker_count}" -lt 1 ]]; then
    warn "marker '${marker}' was not found in ${NEW_BINARY}; string checks are fragile"
  else
    info "Marker '${marker}' found in rebuilt daemon binary (${marker_count} match(es))"
  fi
fi
plist_path="$(
  launchctl print "${GUI_DOMAIN}/${LABEL}" 2>/dev/null |
    awk '/path =/ { print $3; exit }' |
    tr -d ';' || true
)"
if [[ -z "${plist_path}" ]]; then
  error "could not discover launchd plist path for ${GUI_DOMAIN}/${LABEL}"
  error "troubleshooting: run launchctl print \"${GUI_DOMAIN}/${LABEL}\" and check for a path field."
  exit 5
fi
info "LaunchAgent plist: ${plist_path}"
info "Booting out ${LABEL} (non-zero is allowed if it is already unloaded)"
launchctl bootout "${GUI_DOMAIN}" "${plist_path}" 2>&1 || true
sleep 1
info "Bootstrapping ${LABEL}"
if ! bootstrap_output="$(launchctl bootstrap "${GUI_DOMAIN}" "${plist_path}" 2>&1)"; then
  error "launchctl bootstrap failed for ${plist_path}"
  printf '%s\n' "${bootstrap_output}" >&2
  exit 6
fi
DEPLOY_DAEMON_PID_RETRY_SECONDS="${DEPLOY_DAEMON_PID_RETRY_SECONDS:-5}"
NEW_DAEMON_PID=""
for attempt in $(seq 1 "${DEPLOY_DAEMON_PID_RETRY_SECONDS}"); do
  sleep 1
  candidate="$(first_daemon_pid)"
  if [[ -n "${candidate}" && "${candidate}" != "${DAEMON_PID}" ]]; then
    NEW_DAEMON_PID="${candidate}"
    info "  daemon up after ${attempt}s, PID=${NEW_DAEMON_PID}"
    break
  fi
done
if [[ -z "${NEW_DAEMON_PID}" ]]; then
  error "daemon did not restart within ${DEPLOY_DAEMON_PID_RETRY_SECONDS}s after launchctl bootstrap"
  error "  old PID=${DAEMON_PID}, no new PID observed"
  exit 7
fi
info "New daemon PID=${NEW_DAEMON_PID}"
driver_lines="$(pgrep -af "${DRIVER_PATTERN}" || true)"
if [[ -z "${driver_lines}" ]]; then
  info "No stale ${DRIVER_PATTERN} processes found"
else
  while IFS= read -r driver_line; do
    [[ -z "${driver_line}" ]] && continue
    driver_pid="${driver_line%% *}"
    driver_ppid="$(ps -o ppid= -p "${driver_pid}" 2>/dev/null | tr -d ' ' || true)"
    if [[ "${driver_ppid}" == "${NEW_DAEMON_PID}" ]]; then
      info "Skipping new daemon child driver PID=${driver_pid}: ${driver_line}"
      continue
    fi
    info "Killing stale driver PID=${driver_pid}: ${driver_line}"
    kill "${driver_pid}" 2>/dev/null || warn "failed to kill stale driver PID=${driver_pid}"
  done <<<"${driver_lines}"
fi
if [[ ! -f "${TOKEN_FILE}" ]]; then
  error "token file not found for final HTTP verification: ${TOKEN_FILE}"
  exit 8
fi
TOKEN="$(tr -d '\n' < "${TOKEN_FILE}")"
http_code="$(
  curl -s -o /dev/null -w '%{http_code}' \
    http://localhost:8080/mcp \
    -H "Authorization: Bearer ${TOKEN}" 2>/dev/null || true
)"
if [[ -z "${http_code}" || "${http_code}" == "000" ]]; then
  error "daemon did not respond to http://localhost:8080/mcp (HTTP ${http_code:-missing})"
  exit 8
fi
info "HTTP verification responded with status ${http_code}"

printf '\n[deploy-daemon] Deploy complete. New daemon PID=%s, binary=%s\n' \
  "${NEW_DAEMON_PID}" "${NEW_BINARY}"
