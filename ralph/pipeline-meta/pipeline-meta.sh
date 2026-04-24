#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_ROOT="$(cd -- "${SCRIPT_DIR}/../.." && pwd)"
PIPELINE_META_ROOT="${PIPELINE_META_ROOT:-$DEFAULT_ROOT}"
PIPELINE_META_ROOT="$(git -C "$PIPELINE_META_ROOT" rev-parse --show-toplevel)"
PIPELINE_META_MAX_PARALLEL="${PIPELINE_META_MAX_PARALLEL:-1}"
PIPELINE_META_DRY_RUN="${PIPELINE_META_DRY_RUN:-}"

PIPELINE_META_DIR="${PIPELINE_META_ROOT}/ralph/pipeline-meta"
TEMPLATE_DIR="${PIPELINE_META_DIR}/templates"
LOG_DIR="${PIPELINE_META_DIR}/logs"
METRICS_DIR="${PIPELINE_META_DIR}/metrics"
LOCK_FILE="${PIPELINE_META_DIR}/.lock"
WORKTREE_BASE_DIR="${PIPELINE_META_ROOT}/worktrees"
RUN_TIMESTAMP="$(date -u +"%Y%m%dT%H%M%SZ")"
LOG_FILE="${LOG_DIR}/${RUN_TIMESTAMP}.log"

LOCK_FD=""
LOCK_MODE="pid"
JOB_PIDS=()
CHILD_PIDS=()

require_command() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'ERROR: missing required command: %s\n' "$1" >&2
    exit 1
  }
}

log() {
  printf '[%s] %s\n' "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" "$*"
}

trim() {
  local value="$1"
  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"
  printf '%s' "$value"
}

strip_matching_quotes() {
  local value
  value="$(trim "${1:-}")"
  if [[ "${#value}" -ge 2 ]]; then
    if [[ "${value:0:1}" == '"' && "${value: -1}" == '"' ]]; then
      value="${value:1:${#value}-2}"
    elif [[ "${value:0:1}" == "'" && "${value: -1}" == "'" ]]; then
      value="${value:1:${#value}-2}"
    fi
  fi
  printf '%s' "$value"
}

frontmatter_value() {
  local file="$1"
  local key="$2"
  local raw

  raw="$(
    awk -v lookup_key="$key" '
      NR == 1 && $0 == "---" { in_frontmatter = 1; next }
      in_frontmatter && $0 == "---" { exit }
      in_frontmatter && $0 ~ ("^[[:space:]]*" lookup_key ":[[:space:]]*") {
        sub("^[[:space:]]*" lookup_key ":[[:space:]]*", "", $0)
        print
        exit
      }
    ' "$file"
  )"

  strip_matching_quotes "$raw"
}

template_for_pipeline() {
  case "$1" in
    sdd) printf '%s\n' "${TEMPLATE_DIR}/sdd-v1.json" ;;
    tdd) printf '%s\n' "${TEMPLATE_DIR}/tdd-v1.json" ;;
    lightweight) printf '%s\n' "${TEMPLATE_DIR}/lightweight-v1.json" ;;
    *) return 1 ;;
  esac
}

render_template_definition() {
  local template_path="$1"
  local task_id="$2"
  local task_spec="$3"
  local worktree_path="$4"
  local crate="$5"
  local file_paths="$6"
  local rendered

  rendered="$(
    jq \
      --arg taskId "$task_id" \
      --arg taskSpec "$task_spec" \
      --arg worktreePath "$worktree_path" \
      --arg crate "$crate" \
      --arg filePaths "$file_paths" \
      '
      def replace_inputs:
        if type == "string" then
          gsub("\\{\\{inputs\\.taskId\\}\\}"; $taskId)
          | gsub("\\{\\{inputs\\.taskSpec\\}\\}"; $taskSpec)
          | gsub("\\{\\{inputs\\.worktreePath\\}\\}"; $worktreePath)
          | gsub("\\{\\{inputs\\.crate\\}\\}"; $crate)
          | gsub("\\{\\{inputs\\.filePaths\\}\\}"; $filePaths)
        else
          .
        end;
      walk(replace_inputs)
      ' \
      "$template_path"
  )"

  if printf '%s' "$rendered" | jq -e '.. | strings | select(test("\\{\\{inputs\\."))' >/dev/null 2>&1; then
    log "ERROR: unresolved template placeholders remain in ${template_path}"
    return 1
  fi

  printf '%s\n' "$rendered"
}

submission_payload() {
  local rendered_definition="$1"
  jq -nc --argjson definition "$rendered_definition" '{definition: $definition}'
}

pipeline_view_payload() {
  local session_id="$1"
  jq -nc --arg sessionId "$session_id" '{sessionId: $sessionId, view: true}'
}

fallback_backlog_json() {
  local todo_dir="${PIPELINE_META_ROOT}/todo"
  local task_file task_id task_title task_pipeline
  local rows=()

  if [[ ! -d "$todo_dir" ]]; then
    printf '[]\n'
    return 0
  fi

  while IFS= read -r task_file; do
    [[ -n "$task_file" ]] || continue
    task_id="$(frontmatter_value "$task_file" "id")"
    if [[ -z "$task_id" ]]; then
      task_id="$(basename "$task_file" .md)"
    fi
    task_title="$(frontmatter_value "$task_file" "title")"
    task_pipeline="$(frontmatter_value "$task_file" "pipeline")"
    rows+=("$(
      jq -nc \
        --arg taskId "$task_id" \
        --arg title "$task_title" \
        --arg pipeline "$task_pipeline" \
        --arg path "$task_file" \
        '{
          taskId: $taskId,
          title: $title,
          pipeline: $pipeline,
          state: "pending",
          location: "todo",
          path: $path
        }'
    )")
  done < <(find "$todo_dir" -maxdepth 1 -type f -name 'T-*.md' | sort)

  if [[ "${#rows[@]}" -eq 0 ]]; then
    printf '[]\n'
    return 0
  fi

  printf '%s\n' "${rows[@]}" | jq -s '.'
}

record_metrics() {
  local task_id="$1"
  local pipeline="$2"
  local wall_clock_seconds="$3"
  local status="$4"
  local first_pass_clean="$5"
  local metrics_path="${METRICS_DIR}/${task_id}.json"
  local metrics_json

  metrics_json="$(
    jq -nc \
      --arg taskId "$task_id" \
      --arg pipeline "$pipeline" \
      --arg status "$status" \
      --argjson wallClockSeconds "$wall_clock_seconds" \
      --argjson firstPassClean "$first_pass_clean" \
      --arg timestamp "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
      '{
        taskId: $taskId,
        pipeline: $pipeline,
        wallClockSeconds: $wallClockSeconds,
        status: $status,
        firstPassClean: $firstPassClean,
        timestamp: $timestamp
      }'
  )"

  printf '%s\n' "$metrics_json" >"$metrics_path"
}

extract_pipeline_status() {
  printf '%s' "$1" | jq -r '(.data? // .) | .status // "unknown"'
}

extract_first_pass_clean() {
  printf '%s' "$1" | jq -r '
    (.data? // .) as $root
    | ([ $root.nodes[]? | select(.status == "completed" and .nodeId == "remediate") ] | length == 0)
  '
}

extract_all_verdicts_pass() {
  printf '%s' "$1" | jq -r '
    (.data? // .) as $root
    | (
        ([ $root.nodes[]? | select(.status == "completed" and (.nodeId | startswith("gate_fail"))) ] | length == 0)
        and
        ([ $root.nodes[]? | .output? | objects | .verdict? | select(type == "string") | select(. != "PASS") ] | length == 0)
      )
  '
}

create_lock() {
  if command -v flock >/dev/null 2>&1; then
    LOCK_MODE="flock"
    exec {LOCK_FD}>"$LOCK_FILE"
    if ! flock -n "$LOCK_FD"; then
      printf 'ERROR: pipeline-meta.sh is already running (lockfile=%s)\n' "$LOCK_FILE" >&2
      exit 1
    fi
    printf '%s\n' "$$" 1>&"$LOCK_FD"
  else
    LOCK_MODE="pid"
    if [[ -f "$LOCK_FILE" ]]; then
      local existing_pid=""
      existing_pid="$(cat "$LOCK_FILE" 2>/dev/null || true)"
      if [[ -n "$existing_pid" ]] && kill -0 "$existing_pid" 2>/dev/null; then
        printf 'ERROR: pipeline-meta.sh is already running (pid=%s, lockfile=%s)\n' "$existing_pid" "$LOCK_FILE" >&2
        exit 1
      fi
    fi
    printf '%s\n' "$$" >"$LOCK_FILE"
  fi
}

cleanup() {
  local exit_code=$?
  local pid

  trap - EXIT INT TERM

  for pid in "${CHILD_PIDS[@]+"${CHILD_PIDS[@]}"}"; do
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done

  for pid in "${JOB_PIDS[@]+"${JOB_PIDS[@]}"}"; do
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done

  if [[ "$LOCK_MODE" == "flock" && -n "$LOCK_FD" ]]; then
    flock -u "$LOCK_FD" 2>/dev/null || true
  fi
  rm -f "$LOCK_FILE" 2>/dev/null || true

  exit "$exit_code"
}

merge_and_cleanup_worktree() {
  local task_id="$1"
  local branch_name="$2"
  local worktree_path="$3"

  git -C "$PIPELINE_META_ROOT" merge "$branch_name"
  git -C "$PIPELINE_META_ROOT" worktree remove "$worktree_path"
  git -C "$PIPELINE_META_ROOT" branch -d "$branch_name"
}

wait_for_all_jobs() {
  local pid
  local overall_status=0

  for pid in "${JOB_PIDS[@]+"${JOB_PIDS[@]}"}"; do
    if ! wait "$pid"; then
      overall_status=1
    fi
  done

  JOB_PIDS=()
  CHILD_PIDS=()
  return "$overall_status"
}

prune_finished_jobs() {
  local pid
  local remaining=()
  local still_running

  for pid in "${JOB_PIDS[@]+"${JOB_PIDS[@]}"}"; do
    still_running=0
    if kill -0 "$pid" 2>/dev/null; then
      still_running=1
    fi

    if [[ "$still_running" -eq 1 ]]; then
      remaining+=("$pid")
    else
      wait "$pid" || true
    fi
  done

  if [[ "${#remaining[@]}" -gt 0 ]]; then
    JOB_PIDS=("${remaining[@]}")
  else
    JOB_PIDS=()
  fi
}

wait_for_open_slot() {
  prune_finished_jobs
  while [[ "${#JOB_PIDS[@]}" -ge "$PIPELINE_META_MAX_PARALLEL" ]]; do
    sleep 1
    prune_finished_jobs
  done
}

process_task() {
  trap - EXIT INT TERM

  local task_row_json="$1"
  local task_id task_path gate_pipeline task_file frontmatter_task_id frontmatter_pipeline
  local crate file_paths task_spec template_path worktree_path branch_name rendered_definition
  local submit_json session_id driver_status view_json pipeline_status all_verdicts_pass
  local first_pass_clean status start_epoch end_epoch wall_clock_seconds ambient_prompt
  local worktree_created=false

  task_id="$(printf '%s' "$task_row_json" | jq -r '.taskId')"
  task_path="$(printf '%s' "$task_row_json" | jq -r '.path')"
  gate_pipeline="$(printf '%s' "$task_row_json" | jq -r '.pipeline // empty')"
  task_file="$task_path"
  start_epoch="$(date +%s)"
  status="failed"
  first_pass_clean=false

  log "Starting task ${task_id}"

  if [[ ! -f "$task_file" ]]; then
    log "ERROR: task file missing for ${task_id}: ${task_file}"
    return 0
  fi

  frontmatter_task_id="$(frontmatter_value "$task_file" "id")"
  if [[ -n "$frontmatter_task_id" ]]; then
    task_id="$frontmatter_task_id"
  fi

  frontmatter_pipeline="$(frontmatter_value "$task_file" "pipeline")"
  if [[ -n "$frontmatter_pipeline" ]]; then
    gate_pipeline="$frontmatter_pipeline"
  fi

  crate="$(frontmatter_value "$task_file" "crate")"
  file_paths="$(frontmatter_value "$task_file" "file_scope")"
  task_spec="$(<"$task_file")"

  if ! template_path="$(template_for_pipeline "$gate_pipeline")"; then
    log "WARN: unsupported pipeline '${gate_pipeline}' for ${task_id}; skipping"
    if [[ -z "$PIPELINE_META_DRY_RUN" ]]; then
      end_epoch="$(date +%s)"
      wall_clock_seconds=$((end_epoch - start_epoch))
      record_metrics "$task_id" "$gate_pipeline" "$wall_clock_seconds" "failed" false
    fi
    return 0
  fi

  worktree_path="${WORKTREE_BASE_DIR}/${task_id}"
  branch_name="task/${task_id}"
  ambient_prompt="Execute gate task ${task_id}"

  if ! rendered_definition="$(render_template_definition "$template_path" "$task_id" "$task_spec" "$worktree_path" "$crate" "$file_paths")"; then
    log "ERROR: template rendering failed for ${task_id}"
    if [[ -z "$PIPELINE_META_DRY_RUN" ]]; then
      end_epoch="$(date +%s)"
      wall_clock_seconds=$((end_epoch - start_epoch))
      record_metrics "$task_id" "$gate_pipeline" "$wall_clock_seconds" "failed" false
    fi
    return 0
  fi

  if [[ -n "$PIPELINE_META_DRY_RUN" ]]; then
    log "[dry-run] Would create worktree: git worktree add ${worktree_path} -b ${branch_name}"
    log "[dry-run] Would submit pipeline template: ${template_path}"
    log "[dry-run] Would execute: maenifold-pipeline-driver <session-id> \"${ambient_prompt}\""
    log "[dry-run] Would preserve or merge worktree based on final pipeline status"
    return 0
  fi

  mkdir -p "$WORKTREE_BASE_DIR"
  if [[ -e "$worktree_path" ]]; then
    log "ERROR: worktree already exists for ${task_id}: ${worktree_path}"
    end_epoch="$(date +%s)"
    wall_clock_seconds=$((end_epoch - start_epoch))
    record_metrics "$task_id" "$gate_pipeline" "$wall_clock_seconds" "failed" false
    return 0
  fi

  if ! git -C "$PIPELINE_META_ROOT" worktree add "$worktree_path" -b "$branch_name"; then
    log "ERROR: failed to create worktree for ${task_id}"
    end_epoch="$(date +%s)"
    wall_clock_seconds=$((end_epoch - start_epoch))
    record_metrics "$task_id" "$gate_pipeline" "$wall_clock_seconds" "failed" false
    return 0
  fi
  worktree_created=true

  submit_json="$(
    maenifold tool Pipeline "$(submission_payload "$rendered_definition")" --json 2>&1
  )"
  if ! session_id="$(
    printf '%s' "$submit_json" | jq -r '.data.sessionId // .data.pipelineId // .sessionId // .pipelineId // empty'
  )"; then
    session_id=""
  fi

  if [[ -z "$session_id" ]]; then
    log "ERROR: failed to create pipeline for ${task_id}"
    log "$submit_json"
    end_epoch="$(date +%s)"
    wall_clock_seconds=$((end_epoch - start_epoch))
    record_metrics "$task_id" "$gate_pipeline" "$wall_clock_seconds" "failed" false
    log "FAILED: worktree preserved at ${worktree_path}"
    return 0
  fi

  set +e
  maenifold-pipeline-driver "$session_id" "$ambient_prompt"
  driver_status=$?
  set -e
  if [[ "$driver_status" -ne 0 ]]; then
    log "WARN: maenifold-pipeline-driver exited non-zero for ${task_id} (status=${driver_status})"
  fi

  if ! view_json="$(
    maenifold tool Pipeline "$(pipeline_view_payload "$session_id")" --json 2>&1
  )"; then
    log "ERROR: failed to inspect final pipeline state for ${task_id}"
    log "$view_json"
    end_epoch="$(date +%s)"
    wall_clock_seconds=$((end_epoch - start_epoch))
    record_metrics "$task_id" "$gate_pipeline" "$wall_clock_seconds" "failed" false
    log "FAILED: worktree preserved at ${worktree_path}"
    return 0
  fi

  pipeline_status="$(extract_pipeline_status "$view_json")"
  all_verdicts_pass="$(extract_all_verdicts_pass "$view_json")"
  first_pass_clean="$(extract_first_pass_clean "$view_json")"

  if [[ "$pipeline_status" == "completed" && "$all_verdicts_pass" == "true" ]]; then
    if merge_and_cleanup_worktree "$task_id" "$branch_name" "$worktree_path"; then
      status="completed"
      worktree_created=false
      log "Completed task ${task_id}; merged ${branch_name} and cleaned worktree"
    else
      log "ERROR: merge/cleanup failed after successful pipeline for ${task_id}"
      log "FAILED: worktree preserved at ${worktree_path}"
      status="failed"
    fi
  else
    log "FAILED: task ${task_id} finished with pipeline status=${pipeline_status}, allPass=${all_verdicts_pass}"
    log "FAILED: worktree preserved at ${worktree_path}"
    status="failed"
  fi

  end_epoch="$(date +%s)"
  wall_clock_seconds=$((end_epoch - start_epoch))
  record_metrics "$task_id" "$gate_pipeline" "$wall_clock_seconds" "$status" "$first_pass_clean"

  if [[ "$status" == "completed" ]]; then
    return 0
  fi

  return 0
}

main() {
  local backlog_json
  local task_row

  mkdir -p "$LOG_DIR" "$METRICS_DIR"
  create_lock

  exec > >(tee -a "$LOG_FILE") 2>&1
  trap cleanup EXIT INT TERM

  require_command git
  require_command jq
  require_command gate
  require_command maenifold
  require_command maenifold-pipeline-driver

  if ! [[ "$PIPELINE_META_MAX_PARALLEL" =~ ^[1-9][0-9]*$ ]]; then
    log "ERROR: PIPELINE_META_MAX_PARALLEL must be a positive integer"
    exit 1
  fi

  log "pipeline-meta.sh starting"
  log "root=${PIPELINE_META_ROOT}"
  log "max_parallel=${PIPELINE_META_MAX_PARALLEL}"
  log "dry_run=${PIPELINE_META_DRY_RUN:+true}"
  log "log_file=${LOG_FILE}"

  if ! backlog_json="$(gate status --root "$PIPELINE_META_ROOT" --json 2>/dev/null)"; then
    if [[ -n "$PIPELINE_META_DRY_RUN" ]]; then
      log "WARN: gate status unavailable; using todo/ fallback backlog for dry-run"
      backlog_json="$(fallback_backlog_json)"
    else
      log "ERROR: gate status failed for root=${PIPELINE_META_ROOT}"
      exit 1
    fi
  fi

  while IFS= read -r task_row; do
    [[ -n "$task_row" ]] || continue
    wait_for_open_slot
    process_task "$task_row" &
    JOB_PIDS+=("$!")
    CHILD_PIDS+=("$!")
  done < <(
    printf '%s' "$backlog_json" | jq -c '
      def numeric_parts:
        (.taskId // "")
        | ltrimstr("T-")
        | split(".")
        | map(tonumber? // 0);
      map(select((.state // "") == "pending" and (.location // "") == "todo"))
      | sort_by((numeric_parts[0] // 0), (numeric_parts[1] // 0), (.taskId // ""))
      | .[]
    '
  )

  wait_for_all_jobs
  log "pipeline-meta.sh finished"
}

main "$@"
