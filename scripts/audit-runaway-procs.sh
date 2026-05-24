#!/usr/bin/env bash
# Fails when build/test toolchain processes exceed 50% CPU for more than 5 minutes.
# Reports specific PIDs only; never kills or cleans processes.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
CPU_LIMIT_TENTHS=500
ELAPSED_LIMIT_SECONDS=300

etime_to_seconds() {
	local value="$1"
	local days=0
	local time_part="$value"
	if [[ "$value" == *-* ]]; then
		days="${value%%-*}"
		time_part="${value#*-}"
	fi

	local first=0 second=0 third=0
	IFS=: read -r first second third <<<"$time_part"
	if [[ -n "${third:-}" ]]; then
		echo $((days * 86400 + 10#$first * 3600 + 10#$second * 60 + 10#$third))
	elif [[ -n "${second:-}" ]]; then
		echo $((days * 86400 + 10#$first * 60 + 10#$second))
	else
		echo $((days * 86400 + 10#$first))
	fi
}

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

is_allowlisted() {
	local command="$1"
	case "$command" in
		"/Applications/Zed.app/Contents/MacOS/zed"*) return 0 ;;
		"/Applications/Obsidian.app/Contents/MacOS/Obsidian"*) return 0 ;;
		"/opt/homebrew/bin/copilot"*) return 0 ;;
		"/usr/local/bin/copilot"*) return 0 ;;
		"$HOME/.local/bin/copilot"*) return 0 ;;
		"$HOME/.npm-global/bin/copilot"*) return 0 ;;
	esac
	return 1
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

ps_output="$(ps -eo pid,etime,pcpu,command -ww)"
inspected=0
offenders=()

while IFS= read -r line; do
	[[ "$line" =~ ^[[:space:]]*PID[[:space:]] ]] && continue
	read -r pid etime pcpu command <<<"$line"
	[[ "${pid:-}" =~ ^[0-9]+$ ]] || continue
	inspected=$((inspected + 1))
	[[ -n "${command:-}" ]] || continue
	is_allowlisted "$command" && continue
	is_toolchain_process "$command" || continue
	seconds="$(etime_to_seconds "$etime")"
	cpu_tenths="$(cpu_to_tenths "$pcpu")"
	if ((seconds > ELAPSED_LIMIT_SECONDS && cpu_tenths > CPU_LIMIT_TENTHS)); then
		offenders+=("$pid $etime $pcpu $command")
	fi
done <<<"$ps_output"

if ((${#offenders[@]} > 0)); then
	echo "❌ Runaway processes (${#offenders[@]}):"
	printf '%s\n' "${offenders[@]}"
	exit 1
fi

echo "✅ audit-runaway-procs passed ($inspected processes inspected, 0 offenders)."
