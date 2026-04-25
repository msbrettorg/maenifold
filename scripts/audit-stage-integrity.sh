#!/usr/bin/env bash
set -u

# Expected stage order:
#   sdd/tdd:     pending -> swe -> red-team -> blue-team -> done
#   lightweight: pending -> swe -> blue-team -> done
# Allowed exceptions:
#   - Lightweight pipelines have no red-team stage.
#   - Cancelled tasks may have incomplete stage sequences.
#   - GSD pipeline has a different stage order and is skipped for order checks.
# Usage:
#   scripts/audit-stage-integrity.sh
#   scripts/audit-stage-integrity.sh --recent 25

usage() {
  printf 'Usage: %s [--recent N]\n' "$0" >&2
}

die() {
  printf 'error: %s\n' "$1" >&2
  exit 2
}

recent_limit=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --recent)
      [ "$#" -ge 2 ] || die "--recent requires a positive integer"
      case "$2" in
        ''|*[!0-9]*) die "--recent requires a positive integer" ;;
        0) die "--recent requires a positive integer" ;;
      esac
      recent_limit="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      die "unknown argument: $1"
      ;;
  esac
done

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
done_dir="$repo_root/done"

[ -d "$done_dir" ] || die "done/ directory not found at $done_dir"

audit_file() {
  awk '
    function trim(s) {
      sub(/^[[:space:]]+/, "", s)
      sub(/[[:space:]]+$/, "", s)
      return s
    }
    function stage_name(s) {
      s = tolower(trim(s))
      gsub(/[ _]+/, "-", s)
      return s
    }
    function section_name(line, s) {
      s = line
      sub(/^##[[:space:]]+/, "", s)
      sub(/[[:space:]]*#*[[:space:]]*$/, "", s)
      return stage_name(s)
    }
    function is_stage(s) {
      return s == "pending" || s == "swe" || s == "red-team" ||
        s == "blue-team" || s == "result" || s == "done" ||
        s == "cancelled"
    }
    function push_stage() {
      if (current_stage == "") {
        return
      }
      count++
      stages[count] = current_stage
      agents[count] = current_agent
      notes[count] = current_notes
      lines[count] = current_line
      current_agent = ""
      current_notes = ""
    }
    function expect_idx(pipeline_name, stage) {
      if (stage == "result") {
        stage = "done"
      }
      if (pipeline_name == "lightweight") {
        if (stage == "pending") return 0
        if (stage == "swe") return 1
        if (stage == "blue-team") return 2
        if (stage == "done") return 3
        return -1
      }
      if (pipeline_name == "sdd" || pipeline_name == "tdd") {
        if (stage == "pending") return 0
        if (stage == "swe") return 1
        if (stage == "red-team") return 2
        if (stage == "blue-team") return 3
        if (stage == "done") return 4
        return -1
      }
      return -1
    }
    function expected_stage(pipeline_name, idx) {
      if (pipeline_name == "lightweight") {
        if (idx == 0) return "pending"
        if (idx == 1) return "swe"
        if (idx == 2) return "blue-team"
        if (idx == 3) return "done"
      }
      if (pipeline_name == "sdd" || pipeline_name == "tdd") {
        if (idx == 0) return "pending"
        if (idx == 1) return "swe"
        if (idx == 2) return "red-team"
        if (idx == 3) return "blue-team"
        if (idx == 4) return "done"
      }
      return "unknown"
    }
    function violation(stage, line_no, message) {
      if (task_id == "") {
        task_id = FILENAME
        sub(/^.*\//, "", task_id)
        sub(/\.md$/, "", task_id)
      }
      printf "%s:%d: %s: %s\n", FILENAME, line_no, task_id, message
      violations++
    }
    /^id:[[:space:]]*/ && task_id == "" {
      task_id = $0
      sub(/^id:[[:space:]]*/, "", task_id)
      gsub(/^["'\'']|["'\'']$/, "", task_id)
      task_id = trim(task_id)
      next
    }
    /^pipeline:[[:space:]]*/ && pipeline == "" {
      pipeline = tolower($0)
      sub(/^pipeline:[[:space:]]*/, "", pipeline)
      gsub(/^["'\'']|["'\'']$/, "", pipeline)
      pipeline = trim(pipeline)
      next
    }
    /^##[[:space:]]+/ {
      next_stage = section_name($0)
      if (is_stage(next_stage)) {
        push_stage()
        current_stage = next_stage
        current_line = FNR
      } else {
        push_stage()
        current_stage = ""
      }
      next
    }
    current_stage != "" && /^[[:space:]]*-[[:space:]]*Stage ID:[[:space:]]*/ {
      current_stage = $0
      sub(/^[[:space:]]*-[[:space:]]*Stage ID:[[:space:]]*/, "", current_stage)
      current_stage = stage_name(current_stage)
      next
    }
    current_stage != "" && /^[[:space:]]*-[[:space:]]*Agent:[[:space:]]*/ {
      current_agent = $0
      sub(/^[[:space:]]*-[[:space:]]*Agent:[[:space:]]*/, "", current_agent)
      current_agent = tolower(trim(current_agent))
      next
    }
    current_stage != "" && /^[[:space:]]*-[[:space:]]*Notes:[[:space:]]*/ {
      note = $0
      sub(/^[[:space:]]*-[[:space:]]*Notes:[[:space:]]*/, "", note)
      current_notes = current_notes " " trim(note)
      next
    }
    END {
      push_stage()
      for (i = 1; i <= count; i++) {
        note_lc = tolower(notes[i])
        if (note_lc ~ /advancing to|fast-forwarding/) {
          violation(stages[i], lines[i],
            "transition-note language in " stages[i] " stage: " trim(notes[i]))
        }

        if (stages[i] == "red-team" && agents[i] !~ /^(red-team|red_team)$/) {
          violation(stages[i], lines[i],
            "agent/role mismatch: red-team stage recorded by agent " agents[i])
        }

        if (stages[i] == "blue-team" && agents[i] !~ /^(blue-team|blue_team)$/) {
          violation(stages[i], lines[i],
            "agent/role mismatch: blue-team stage recorded by agent " agents[i])
        }
      }
      if (pipeline == "gsd") {
        exit violations ? 1 : 0
      }
      known_pipeline = pipeline == "sdd" || pipeline == "tdd" || pipeline == "lightweight"
      for (i = 0; i <= 4; i++) {
        seen[i] = 0
      }
      last_idx = -1
      cancelled_seen = 0
      for (i = 1; i <= count; i++) {
        stage = stages[i]
        if (stage == "cancelled") {
          cancelled_seen = 1
          continue
        }
        idx = expect_idx(pipeline, stage)
        if (idx < 0) {
          if (known_pipeline) {
            violation(stage, lines[i],
              "unexpected stage " stage " for pipeline " pipeline)
          }
          continue
        }
        if (idx == 0) {
          for (j = 0; j <= 4; j++) {
            seen[j] = 0
          }
          seen[0] = 1
          last_idx = idx
          continue
        }
        predecessor = idx - 1
        if (!seen[predecessor]) {
          violation(stage, lines[i],
            "stage out of expected order for " pipeline ": " stage " appears before " expected_stage(pipeline, predecessor))
        }
        seen[idx] = 1
        last_idx = idx
      }
      if (known_pipeline && !cancelled_seen) {
        terminal_idx = pipeline == "lightweight" ? 3 : 4
        if (last_idx != terminal_idx) {
          violation("done", FNR,
            "missing terminal stage for " pipeline ": expected " expected_stage(pipeline, terminal_idx))
        }
      }
      exit violations ? 1 : 0
    }
  ' "$1"
}

files_checked=0
violations_found=0

while IFS= read -r task_file; do
  [ -n "$task_file" ] || continue
  files_checked=$((files_checked + 1))
  findings=$(audit_file "$task_file")
  status=$?
  if [ "$status" -ne 0 ] && [ -n "$findings" ]; then
    printf '%s\n' "$findings"
    violations_found=1
  fi
done < <(
  if [ -n "$recent_limit" ]; then
    find "$done_dir" -maxdepth 1 -type f -name '*.md' -print0 |
      xargs -0 ls -t 2>/dev/null |
      head -n "$recent_limit"
  else
    find "$done_dir" -maxdepth 1 -type f -name '*.md' | sort
  fi
)

if [ "$files_checked" -eq 0 ]; then
  printf 'No done/ task files found.\n'
  exit 0
fi

if [ "$violations_found" -eq 0 ]; then
  printf 'Stage integrity audit clean (%d files checked).\n' "$files_checked"
  exit 0
fi

printf 'Stage integrity audit found violations (%d files checked).\n' "$files_checked" >&2
exit 1
