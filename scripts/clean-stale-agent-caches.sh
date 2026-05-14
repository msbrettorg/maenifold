#!/usr/bin/env bash
set -euo pipefail

# Evict stale per-agent Cargo build caches under target/agents/<task-id>.
# The TTL defaults to 7 days and can be overridden with
# MAENIFOLD_AGENT_CACHE_TTL_DAYS. The script is a dry run by default:
# it lists cache directories and the bytes that would be reclaimed. Pass
# --apply to actually delete stale caches.

apply=0
if [[ "${1:-}" == "--apply" ]]; then
  apply=1
  shift
fi

if [[ $# -ne 0 ]]; then
  echo "usage: $0 [--apply]" >&2
  exit 2
fi

ttl_days="${MAENIFOLD_AGENT_CACHE_TTL_DAYS:-7}"
if ! [[ "$ttl_days" =~ ^[0-9]+$ ]]; then
  echo "MAENIFOLD_AGENT_CACHE_TTL_DAYS must be a non-negative integer" >&2
  exit 2
fi

agents_dir="target/agents"
if [[ ! -d "$agents_dir" ]]; then
  echo "target/agents/ does not exist (no caches to evict)"
  exit 0
fi

case "$(uname -s)" in
  Darwin) stat_mtime() { stat -f %m "$1"; } ;;
  *) stat_mtime() { stat -c %Y "$1"; } ;;
esac

cache_size() {
  if size_line="$(du -sb "$1" 2>/dev/null)"; then
    printf '%s\n' "${size_line%%[[:space:]]*}"
  else
    size_line="$(du -sk "$1")"
    printf '%s\n' "$(( ${size_line%%[[:space:]]*} * 1024 ))"
  fi
}

now="$(date +%s)"
count=0
stale_count=0
reclaimed=0

while IFS= read -r path; do
  mtime="$(stat_mtime "$path")"
  age_seconds=$(( now - mtime ))
  if (( age_seconds < 0 )); then
    age_seconds=0
  fi
  age_days=$(( age_seconds / 86400 ))
  size="$(cache_size "$path")"
  printf '%dd  %s  %s\n' "$age_days" "$size" "$path"
  count=$(( count + 1 ))

  if (( age_days < ttl_days )); then
    continue
  fi

  stale_count=$(( stale_count + 1 ))
  reclaimed=$(( reclaimed + size ))

  if (( apply == 1 )); then
    rm -rf "$path"
  fi
done < <(find target/agents -mindepth 1 -maxdepth 1 -type d | sort)

if (( apply == 1 )); then
  echo "Reclaimed: $reclaimed bytes from $stale_count caches"
else
  echo "Total: $count caches, $reclaimed would-be reclaimed bytes"
fi
