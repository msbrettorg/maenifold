#!/usr/bin/env bash
# Enforces target/ disk budget. Default threshold is 50 GiB via MAENIFOLD_TARGET_BUDGET_GB.
# Reports over-budget directories only; never removes files.

set -u

BUDGET_GB="${MAENIFOLD_TARGET_BUDGET_GB:-50}"
AGENTS_WARN_GB=20

if [[ ! "$BUDGET_GB" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
	echo "❌ Invalid MAENIFOLD_TARGET_BUDGET_GB: $BUDGET_GB"
	exit 1
fi

if [[ ! -d target ]]; then
	echo "✅ audit-disk-budget passed (target/ missing, budget ${BUDGET_GB}G)."
	exit 0
fi

target_human="$(du -sh target/ | awk '{print $1}')"
agents_human="missing"
if [[ -d target/agents ]]; then
	agents_human="$(du -sh target/agents/ 2>/dev/null | awk '{print $1}')"
fi

target_kb="$(du -sk target/ | awk '{print $1}')"
budget_kb="$(awk -v gb="$BUDGET_GB" 'BEGIN { printf "%.0f\n", gb * 1048576 }')"

if ((target_kb <= budget_kb)); then
	echo "✅ audit-disk-budget passed (target/ ${target_human}, target/agents/ ${agents_human}, budget ${BUDGET_GB}G)."
	exit 0
fi

echo "❌ target/ disk budget exceeded: target/ ${target_human} > ${BUDGET_GB}G"
echo "Per-subdir usage:"
du -sh -d 1 target/ 2>/dev/null | sort -hr

if [[ -d target/agents ]]; then
	agents_kb="$(du -sk target/agents/ 2>/dev/null | awk '{print $1}')"
	agents_warn_kb=$((AGENTS_WARN_GB * 1048576))
	if ((agents_kb > agents_warn_kb)); then
		echo "target/agents/ exceeds ${AGENTS_WARN_GB}G; consider: bash scripts/clean-stale-agent-caches.sh --apply"
	fi
fi

exit 1
