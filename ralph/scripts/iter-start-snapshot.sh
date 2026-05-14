#!/usr/bin/env bash
# Captures process and target/ disk state at iteration start for end-of-iter side-effect audit.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd -P)"
SNAPSHOT_DIR="$REPO_ROOT/ralph/state/iter-snapshots"
timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

mkdir -p "$SNAPSHOT_DIR"
ps_file="$SNAPSHOT_DIR/${timestamp}-start.ps.txt"
du_file="$SNAPSHOT_DIR/${timestamp}-start.du.txt"

cd "$REPO_ROOT" || exit 1
ps -eo pid,etime,pcpu,command -ww >"$ps_file"
{
	du -sh target/ 2>/dev/null || true
	du -sh target/agents/ 2>/dev/null || true
} >"$du_file"

echo "✅ iter start snapshot written:"
echo "$ps_file"
echo "$du_file"
