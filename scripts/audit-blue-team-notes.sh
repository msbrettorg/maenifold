#!/usr/bin/env bash
set -euo pipefail

# Audit completed gate tasks for blue-team notes that lack acceptance-criteria
# evidence. Exit code 0 = clean, 1 = violations found.

usage() {
  cat <<'USAGE'
Usage:
  scripts/audit-blue-team-notes.sh [--recent N] [TASK_ID ...]

Scans done/*.md for blue-team sections. Optional TASK_ID arguments limit the
audit to matching done task files, for example T-05.70 or T-214.12.

Options:
  --recent N   audit only the N most recently modified done task files
  -h, --help   show this help
USAGE
}

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DONE_DIR="$REPO_ROOT/done"
recent=""
task_ids=()

while (($# > 0)); do
  case "$1" in
    --recent)
      if [[ $# -lt 2 || ! "$2" =~ ^[0-9]+$ || "$2" == "0" ]]; then
        echo "error: --recent requires a positive integer" >&2
        exit 2
      fi
      recent="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      while (($# > 0)); do
        task_ids+=("$1")
        shift
      done
      ;;
    -*)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
    *)
      task_ids+=("$1")
      shift
      ;;
  esac
done

if [[ ! -d "$DONE_DIR" ]]; then
  echo "error: done/ directory not found at $DONE_DIR" >&2
  exit 2
fi

files=()

if ((${#task_ids[@]} > 0)); then
  for task_id in "${task_ids[@]}"; do
    matched=false
    while IFS= read -r file; do
      files+=("$file")
      matched=true
    done < <(find "$DONE_DIR" -maxdepth 1 -type f \( -name "$task_id.md" -o -name "$task_id-*.md" \) | sort)
    if [[ "$matched" == false ]]; then
      echo "warning: no done task file found for $task_id" >&2
    fi
  done
elif [[ -n "$recent" ]]; then
  while IFS= read -r file; do
    files+=("$file")
  done < <(python3 - "$DONE_DIR" "$recent" <<'PY'
import pathlib
import sys

done_dir = pathlib.Path(sys.argv[1])
limit = int(sys.argv[2])
files = sorted(
    done_dir.glob("*.md"),
    key=lambda path: path.stat().st_mtime,
    reverse=True,
)
for path in files[:limit]:
    print(path)
PY
)
else
  while IFS= read -r file; do
    files+=("$file")
  done < <(find "$DONE_DIR" -maxdepth 1 -type f -name '*.md' | sort)
fi

task_id_for_file() {
  local file="$1"
  local id
  id=$(awk -F': *' 'tolower($1) == "id" { print $2; exit }' "$file")
  if [[ -n "$id" ]]; then
    echo "$id"
  else
    basename "$file" .md
  fi
}

acceptance_criteria_section() {
  local file="$1"
  awk '
    BEGIN { in_section = 0; ac_level = 0 }
    /^#{2,6}[[:space:]]+/ {
      heading = $0
      sub(/[[:space:]].*$/, "", heading)
      level = length(heading)
      if ($0 ~ /^#{2,6}[[:space:]]+[Aa]cceptance [Cc]riteria/) {
        in_section = 1
        ac_level = level
        next
      }
      if (in_section && level <= ac_level) {
        exit
      }
    }
    in_section { print }
  ' "$file"
}

blue_team_sections() {
  local file="$1"
  awk '
    BEGIN { in_section = 0 }
    /^##[[:space:]]+/ {
      title = $0
      sub(/^##[[:space:]]+/, "", title)
      lower = tolower(title)
      if (lower ~ /^blue-team[[:space:]]*$/) {
        in_section = 1
        print
        next
      }
      if (in_section) {
        in_section = 0
      }
    }
    in_section { print }
  ' "$file"
}

count_acceptance_criteria() {
  awk '
    /^[[:space:]]*([0-9]+[.)]|[-*])[[:space:]]+[^[:space:]]/ { count++ }
    END { print count + 0 }
  '
}

count_evidence_rows() {
  awk '
    BEGIN { count = 0 }
    {
      lower = tolower($0)
      has_status = lower ~ /(^|[^[:alpha:]])(pass|fail|blocked)([^[:alpha:]]|$)/
      has_ac = lower ~ /(^|[^[:alpha:]])ac[[:space:]#.:_-]*[0-9]+/
      has_named = lower ~ /acceptance criterion[[:space:]#.:_-]*[0-9]+/
      has_number = lower ~ /^[[:space:]]*[-*|]?[[:space:]]*[0-9]+[.)|]/
      if (has_status && (has_ac || has_named || has_number)) {
        count++
      }
    }
    END { print count + 0 }
  '
}

violations=0
generic_re='build passes|verified clean|red-team confirmed|tests pass|all checks pass'

for file in "${files[@]}"; do
  task_id="$(task_id_for_file "$file")"
  ac_section="$(acceptance_criteria_section "$file")"
  blue_sections="$(blue_team_sections "$file")"

  if [[ -z "$blue_sections" ]]; then
    continue
  fi

  ac_count="$(printf '%s\n' "$ac_section" | count_acceptance_criteria)"
  evidence_rows="$(printf '%s\n' "$blue_sections" | count_evidence_rows)"
  rel="${file#"$REPO_ROOT/"}"

  if printf '%s\n' "$blue_sections" | grep -Eiq "$generic_re"; then
    if (( evidence_rows == 0 )); then
      echo "$task_id: generic-only blue-team note in $rel"
      echo "  add commands, exit codes, and an acceptance-criterion evidence matrix"
      ((violations+=1))
    fi
  fi

  if (( ac_count > 0 && evidence_rows < ac_count )); then
    echo "$task_id: missing per-criterion blue-team evidence in $rel"
    echo "  acceptance criteria: $ac_count; evidence rows with PASS/FAIL/BLOCKED: $evidence_rows"
    ((violations+=1))
  fi
done

if (( violations > 0 )); then
  echo "❌ blue-team note audit: $violations violation(s) found."
  exit 1
fi

echo "✅ blue-team note audit passed."
