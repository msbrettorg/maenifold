#!/usr/bin/env bash
set -euo pipefail

# Audit completed epic/container tasks for child-closure integrity.
#
# What this checks:
#   1. Done epics (YAML frontmatter: type: epic) must not have children still
#      in todo/. Children are discovered by task ID prefix (T-209.1 belongs to
#      T-209.0), frontmatter parent, or frontmatter depends_on references.
#   2. Done epics must not have close/cancel notes containing suspicious terms:
#      blocked, partial, deferred, skipped, in flight, backlog.
#
# Explicit exception rules for cancelled/superseded epics:
#   - Add YAML frontmatter:
#       epic_audit_exception: cancelled
#       epic_audit_exception: superseded
#     or add a close/cancel note line:
#       Epic-Audit-Exception: cancelled
#       Epic-Audit-Exception: superseded
#   - The marker must be paired with a close/cancel note containing the same
#     word ("cancelled"/"canceled" or "superseded"). A generic "Cancelled"
#     section alone is not an exception.
#   - Exceptions suppress both open-child and suspicious-close-note reports for
#     that epic. Use them only when the epic is intentionally cancelled or
#     superseded and the replacement path is documented in the note.

ROOT="${1:-.}"
ACTIVE_DIR="$ROOT/todo"
DONE_DIR="$ROOT/done"

if [[ ! -d "$ACTIVE_DIR" || ! -d "$DONE_DIR" ]]; then
  echo "audit-epics: expected todo/ and done/ under: $ROOT" >&2
  exit 2
fi

shopt -s nullglob

declare -a todo_files=("$ACTIVE_DIR"/*.md)
declare -a done_files=("$DONE_DIR"/*.md)
declare -a issues=()
declare -a exceptions=()

trim_value() {
  local value="$1"
  value="${value%%#*}"
  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"
  value="${value%\"}"
  value="${value#\"}"
  value="${value%\'}"
  value="${value#\'}"
  printf '%s\n' "$value"
}

frontmatter_value() {
  local file="$1"
  local key="$2"
  awk -v key="$key" '
    NR == 1 && $0 == "---" { in_fm = 1; next }
    in_fm && $0 == "---" { exit }
    in_fm {
      split($0, parts, ":")
      current = parts[1]
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", current)
      if (tolower(current) == tolower(key)) {
        sub(/^[^:]*:[[:space:]]*/, "", $0)
        print
        exit
      }
    }
  ' "$file" | while IFS= read -r value; do trim_value "$value"; done
}

task_id() {
  local file="$1"
  local id
  id="$(frontmatter_value "$file" "id")"
  if [[ -n "$id" ]]; then
    printf '%s\n' "$id"
    return
  fi
  basename "$file" | grep -Eo '^T-[0-9]+(\.[0-9A-Za-z]+)?' || true
}

task_title() {
  local file="$1"
  local title
  title="$(frontmatter_value "$file" "title")"
  if [[ -n "$title" ]]; then
    printf '%s\n' "$title"
  else
    basename "$file"
  fi
}

is_epic() {
  local file="$1"
  [[ "$(frontmatter_value "$file" "type" | tr '[:upper:]' '[:lower:]')" == "epic" ]]
}

task_prefix() {
  local id="$1"
  if [[ "$id" == *.0 ]]; then
    printf '%s\n' "${id%.*}"
  fi
}

references_task() {
  local file="$1"
  local key="$2"
  local target="$3"
  local value
  value="$(frontmatter_value "$file" "$key")"
  [[ "$value" =~ (^|[^A-Za-z0-9.-])"$target"([^A-Za-z0-9.-]|$) ]]
}

child_matches_epic() {
  local file="$1"
  local epic_id="$2"
  local prefix="$3"
  local child_id
  child_id="$(task_id "$file")"
  [[ -n "$child_id" ]] || return 1
  [[ "$child_id" != "$epic_id" ]] || return 1

  if [[ -n "$prefix" && "$child_id" == "$prefix".* ]]; then
    return 0
  fi
  if references_task "$file" "parent" "$epic_id"; then
    return 0
  fi
  [[ -n "$prefix" ]] || return 1
  if references_task "$file" "depends_on" "$epic_id"; then
    return 0
  fi
  return 1
}

close_notes() {
  local file="$1"
  awk '
    /^##[[:space:]]+/ {
      heading = tolower($0)
      in_note = heading ~ /(cancelled|canceled|closed|close|complete|completed|done|outcome|result|passed|pass)/
      if (in_note) print
      next
    }
    /^##[[:space:]]+/ { in_note = 0 }
    in_note { print }
  ' "$file"
}

close_note_body() {
  local file="$1"
  close_notes "$file" | grep -Ev '^##[[:space:]]+' || true
}

close_notes_without_fenced_code() {
  local file="$1"
  close_notes "$file" | awk '
    /^```/ { in_fence = !in_fence; next }
    !in_fence { print }
  '
}

exception_kind() {
  local file="$1"
  local raw note
  raw="$(frontmatter_value "$file" "epic_audit_exception" | tr '[:upper:]' '[:lower:]')"

  if [[ -z "$raw" ]]; then
    raw="$(grep -Eio 'Epic-Audit-Exception:[[:space:]]*(cancelled|canceled|superseded)' "$file" \
      | head -1 \
      | sed -E 's/.*:[[:space:]]*//' \
      | tr '[:upper:]' '[:lower:]' || true)"
  fi

  case "$raw" in
    cancelled|canceled)
      note="$(close_note_body "$file" | tr '[:upper:]' '[:lower:]')"
      if [[ "$note" =~ cancelled|canceled ]]; then
        printf '%s\n' "cancelled"
      fi
      ;;
    superseded)
      note="$(close_note_body "$file" | tr '[:upper:]' '[:lower:]')"
      if [[ "$note" =~ superseded ]]; then
        printf '%s\n' "superseded"
      fi
      ;;
  esac
}

suspicious_close_lines() {
  local file="$1"
  close_notes_without_fenced_code "$file" | grep -Ein 'blocked|partial|deferred|skipped|in flight|backlog' || true
}

record_issue() {
  issues+=("$1")
}

audit_done_epic() {
  local file="$1"
  local epic_id prefix title exception child_file child_id child_title suspicious

  epic_id="$(task_id "$file")"
  [[ -n "$epic_id" ]] || return 0
  prefix="$(task_prefix "$epic_id")"
  title="$(task_title "$file")"
  exception="$(exception_kind "$file")"

  if [[ -n "$exception" ]]; then
    exceptions+=("$epic_id ($exception): $title")
    return 0
  fi

  for child_file in "${todo_files[@]}"; do
    if child_matches_epic "$child_file" "$epic_id" "$prefix"; then
      child_id="$(task_id "$child_file")"
      child_title="$(task_title "$child_file")"
      record_issue "OPEN_CHILD: $epic_id is in done/ but $child_id remains in todo/ ($child_title)"
    fi
  done

  suspicious="$(suspicious_close_lines "$file")"
  if [[ -n "$suspicious" ]]; then
    record_issue "SUSPICIOUS_CLOSE: $epic_id close/cancel notes contain suspicious terms:
$suspicious"
  fi
}

for epic_file in "${done_files[@]}"; do
  if is_epic "$epic_file"; then
    audit_done_epic "$epic_file"
  fi
done

if ((${#exceptions[@]} > 0)); then
  echo "Epic audit exceptions:"
  printf '  - %s\n' "${exceptions[@]}"
fi

if ((${#issues[@]} > 0)); then
  echo "Epic audit violations:"
  printf '  - %s\n' "${issues[@]}"
  exit 1
fi

echo "Epic audit passed: no done/ epics have open todo/ children or suspicious close notes."
