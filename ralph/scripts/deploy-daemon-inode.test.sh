#!/usr/bin/env bash
# deploy-daemon-inode.test.sh — T-322.36 blue-team: inode comparison regression test
set -euo pipefail

MOCK_RUNNING_INODE="315778576"   # what the kernel has mapped
MOCK_DISK_INODE="316830419"      # what stat -f %i returns after rebuild
MOCK_BINARY_PATH="/Users/brett/src/ma-collective/maenifold-rs/target/release/maenifold"
MOCK_PID="47031"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_SCRIPT="${SCRIPT_DIR}/deploy-daemon.sh"

fail() { printf 'not ok - %s\n' "$*" >&2; exit 1; }

assert_eq() {
  local actual="$1"
  local expected="$2"
  local label="$3"
  [[ "${actual}" == "${expected}" ]] || fail "${label}: got ${actual}, expected ${expected}"
  printf 'ok - %s (%s)\n' "${label}" "${actual}"
}

lsof() {
  [[ "$1" == "-p" && "$2" == "${MOCK_PID}" ]] || fail "unexpected lsof args: $*"
  cat <<LSOF
COMMAND     PID  USER   FD   TYPE DEVICE SIZE/OFF      NODE NAME
maenifold ${MOCK_PID} brett  cwd    DIR   1,15     1792 136939890 /Users/brett/src/ma-collective/maenifold-rs
maenifold ${MOCK_PID} brett  txt    REG   1,15 38256080 ${MOCK_RUNNING_INODE} ${MOCK_BINARY_PATH}
LSOF
}

stat() {
  [[ "$1" == "-f" && "$2" == "%i" && "$3" == "${MOCK_BINARY_PATH}" ]] ||
    fail "unexpected stat args: $*"
  printf '%s\n' "${MOCK_DISK_INODE}"
}

inode_for_path() { stat -f %i "$1"; }

extract_running_path_CURRENT() {
  # Mirrors deploy-daemon.sh line 102: path only, not the lsof NODE column.
  lsof -p "${MOCK_PID}" | awk '$4 == "txt" { print $9; exit }'
}

extract_running_inode_CURRENT() {
  local path
  path="$(extract_running_path_CURRENT)"
  inode_for_path "${path}"
}

extract_running_inode_FIXED() {
  lsof -p "${MOCK_PID}" |
    awk '$4 == "txt" && $9 ~ /maenifold$/ { print $8; exit }'
}

deploy_script_uses_fixed_inode_extraction() {
  grep -Eq 'awk .*(maenifold.*print \$8|print \$8.*maenifold)' "${DEPLOY_SCRIPT}"
}

extract_running_inode_FROM_SCRIPT() {
  if deploy_script_uses_fixed_inode_extraction; then
    extract_running_inode_FIXED
  else
    extract_running_inode_CURRENT
  fi
}

stale_daemon_needs_restart() { [[ "$1" != "$2" ]]; }

running_path="$(extract_running_path_CURRENT)"
assert_eq "${running_path}" "${MOCK_BINARY_PATH}" "current deploy awk extracts txt path"

current_inode="$(extract_running_inode_CURRENT)"
assert_eq "${current_inode}" "${MOCK_DISK_INODE}" "current extraction reproduces disk-vs-disk bug"

fixed_inode="$(extract_running_inode_FIXED)"
assert_eq "${fixed_inode}" "${MOCK_RUNNING_INODE}" "fixed extraction reads lsof NODE column"

if ! stale_daemon_needs_restart "${fixed_inode}" "${MOCK_DISK_INODE}"; then
  fail "fixed extraction did not detect stale daemon"
fi
printf 'ok - fixed extraction detects stale daemon needs restart\n'

if stale_daemon_needs_restart "${MOCK_DISK_INODE}" "${MOCK_DISK_INODE}"; then
  fail "equal lsof NODE and stat inode should not need redeploy"
fi
printf 'ok - equal running and disk inode reports no redeploy needed\n'

script_inode="$(extract_running_inode_FROM_SCRIPT)"
if ! stale_daemon_needs_restart "${script_inode}" "${MOCK_DISK_INODE}"; then
  fail "deploy-daemon.sh still compares disk-vs-disk; stale daemon was missed"
fi
printf 'ok - deploy-daemon.sh detects stale daemon with lsof NODE inode\n'
