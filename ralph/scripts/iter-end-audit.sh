#!/usr/bin/env bash
# Re-snapshots process/disk state and fails if new hot toolchain processes appear or target/ grows >5 GiB.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd -P)"
SNAPSHOT_DIR="$REPO_ROOT/ralph/state/iter-snapshots"
CPU_LIMIT_TENTHS=500
TARGET_GROWTH_LIMIT_KB=$((5 * 1048576))

latest_start_ps="$(find "$SNAPSHOT_DIR" -name '*-start.ps.txt' -type f -print 2>/dev/null | sort | tail -n 1)"
if [[ -z "$latest_start_ps" ]]; then
	echo "❌ No iter start process snapshot found in $SNAPSHOT_DIR"
	exit 1
fi
latest_start_du="${latest_start_ps%.ps.txt}.du.txt"

timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
end_ps="$SNAPSHOT_DIR/${timestamp}-end.ps.txt"
end_du="$SNAPSHOT_DIR/${timestamp}-end.du.txt"

mkdir -p "$SNAPSHOT_DIR"
cd "$REPO_ROOT" || exit 1
ps -eo pid,etime,pcpu,command -ww >"$end_ps"
{
	du -sh target/ 2>/dev/null || true
	du -sh target/agents/ 2>/dev/null || true
} >"$end_du"

cpu_to_tenths() {
	local value="${1:-0}"
	local whole="${value%%.*}"
	local frac="0"
	if [[ "$value" == *.* ]]; then
		frac="${value#*.}"
		frac="${frac:0:1}"
	fi
	echo $((10#${whole:-0} * 10 + 10#${frac:-0}))
}

is_toolchain_process() {
	local command="$1"
	if [[ "$command" == *"$REPO_ROOT"* && "$command" == *"node"* ]]; then
		return 0
	fi
	case "$command" in
		*" cargo "*|cargo\ *|*/cargo\ *|*" rustc "*|rustc\ *|*/rustc\ *|\
		*" shell-mcp"*|shell-mcp\ *|*/shell-mcp\ *) return 0 ;;
	esac
	return 1
}

size_to_kb() {
	local raw="$1"
	awk -v size="$raw" 'BEGIN {
		if (size == "") { print 0; exit }
		unit = substr(size, length(size), 1)
		value = size + 0
		if (unit == "T") value *= 1073741824
		else if (unit == "G") value *= 1048576
		else if (unit == "M") value *= 1024
		else if (unit == "K") value *= 1
		else value /= 1024
		printf "%.0f\n", value
	}'
}

start_pids=" $(awk 'NR > 1 { print $1 }' "$latest_start_ps" | tr '\n' ' ') "
new_hot=()
while IFS= read -r line; do
	[[ "$line" =~ ^[[:space:]]*PID[[:space:]] ]] && continue
	read -r pid _etime pcpu command <<<"$line"
	[[ "${pid:-}" =~ ^[0-9]+$ ]] || continue
	[[ "$start_pids" == *" $pid "* ]] && continue
	[[ -n "${command:-}" ]] || continue
	is_toolchain_process "$command" || continue
	cpu_tenths="$(cpu_to_tenths "$pcpu")"
	if ((cpu_tenths > CPU_LIMIT_TENTHS)); then
		new_hot+=("$pid $pcpu $command")
	fi
done <"$end_ps"

start_target_size="$(awk '$2 == "target/" { print $1; exit }' "$latest_start_du" 2>/dev/null)"
end_target_size="$(awk '$2 == "target/" { print $1; exit }' "$end_du" 2>/dev/null)"
start_target_kb="$(size_to_kb "$start_target_size")"
end_target_kb="$(size_to_kb "$end_target_size")"
growth_kb=$((end_target_kb - start_target_kb))

failed=0
if ((${#new_hot[@]} > 0)); then
	failed=1
	echo "❌ New hot toolchain processes (${#new_hot[@]}):"
	printf '%s\n' "${new_hot[@]}"
fi
if ((growth_kb > TARGET_GROWTH_LIMIT_KB)); then
	failed=1
	growth_gb="$(awk -v kb="$growth_kb" 'BEGIN { printf "%.1f", kb / 1048576 }')"
	echo "❌ target/ grew ${growth_gb}G since iter start (limit 5G)."
fi

if ((failed)); then
	echo "--- process snapshot diff ---"
	diff -u "$latest_start_ps" "$end_ps" || true
	echo "--- disk snapshot diff ---"
	diff -u "$latest_start_du" "$end_du" || true
	exit 1
fi

echo "✅ iter end audit passed."
echo "$end_ps"
echo "$end_du"
