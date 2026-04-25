#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
SCRIPT_PATH="${SCRIPT_DIR}/$(basename -- "${BASH_SOURCE[0]}")"

PIPELINE_META_ROOT="${PIPELINE_META_ROOT:-$DEFAULT_ROOT}"
PIPELINE_META_MAX_PARALLEL="${PIPELINE_META_MAX_PARALLEL:-1}"
PIPELINE_META_INNER_ITERATIONS="${PIPELINE_META_INNER_ITERATIONS:-1}"
PIPELINE_META_MAX_META_ITERATIONS="${PIPELINE_META_MAX_META_ITERATIONS:-5}"
PIPELINE_META_META_AGENT="${PIPELINE_META_META_AGENT:-Copilot}"
PIPELINE_META_META_THRESHOLD="${PIPELINE_META_META_THRESHOLD:-0.5}"
PIPELINE_META_META_TIMEOUT_SECONDS="${PIPELINE_META_META_TIMEOUT_SECONDS:-600}"
PIPELINE_META_DRY_RUN="${PIPELINE_META_DRY_RUN:-}"
PIPELINE_META_TASK_ID="${PIPELINE_META_TASK_ID:-}"
PIPELINE_META_LIMIT="${PIPELINE_META_LIMIT:-}"
PIPELINE_META_PIPELINE="${PIPELINE_META_PIPELINE:-}"

PIPELINE_META_DIR=""
TEMPLATE_DIR=""
LOG_DIR=""
METRICS_DIR=""
OTEL_DIR=""
META_DIR=""
META_SNAPSHOT_DIR=""
META_VERDICT_DIR=""
META_METRICS_ARCHIVE_DIR=""
META_BASELINE_FILE=""
LOCK_FILE=""
WORKTREE_BASE_DIR=""
RUN_TIMESTAMP="$(date -u +"%Y%m%dT%H%M%SZ")"
LOG_FILE=""
OTEL_FILE=""

LOCK_FD=""
LOCK_MODE="pid"
JOB_PIDS=()
CHILD_PIDS=()
FAILURES=0
PIPELINE_META_ENABLE_META=false

require_command() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'ERROR: missing required command: %s\n' "$1" >&2
    exit 1
  }
}

log() {
  printf '[%s] %s\n' "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" "$*"
}

usage() {
  cat <<EOF
Usage: $(basename "$0") [options]

Options:
  --meta                 Run the meta-optimization outer loop.
  --dry-run              Render and plan work without creating pipelines/worktrees.
  --task <T-id>          Run only one task, e.g. T-218.42.
  --limit <n>            Run at most n matching tasks.
  --pipeline <name>      Run only tasks using sdd, tdd, or lightweight.
  --root <path>          Repository root override.
  -h, --help             Show this help text.
EOF
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --meta)
        PIPELINE_META_ENABLE_META=true
        shift
        ;;
      --dry-run)
        PIPELINE_META_DRY_RUN=1
        shift
        ;;
      --task)
        PIPELINE_META_TASK_ID="${2:-}"
        shift 2
        ;;
      --limit)
        PIPELINE_META_LIMIT="${2:-}"
        shift 2
        ;;
      --pipeline)
        PIPELINE_META_PIPELINE="${2:-}"
        shift 2
        ;;
      --root)
        PIPELINE_META_ROOT="${2:-}"
        shift 2
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        log "ERROR: unknown argument: $1"
        usage >&2
        exit 1
        ;;
    esac
  done
}

configure_paths() {
  PIPELINE_META_ROOT="$(git -C "$PIPELINE_META_ROOT" rev-parse --show-toplevel)"
  PIPELINE_META_DIR="${PIPELINE_META_ROOT}/pipeline-meta"
  TEMPLATE_DIR="${PIPELINE_META_DIR}/templates"
  LOG_DIR="${PIPELINE_META_DIR}/logs"
  METRICS_DIR="${PIPELINE_META_DIR}/metrics"
  OTEL_DIR="${PIPELINE_META_DIR}/otel"
  META_DIR="${PIPELINE_META_DIR}/meta"
  META_SNAPSHOT_DIR="${META_DIR}/snapshots"
  META_VERDICT_DIR="${META_DIR}/verdicts"
  META_METRICS_ARCHIVE_DIR="${META_DIR}/metrics"
  META_BASELINE_FILE="${META_DIR}/baseline-summary.json"
  LOCK_FILE="${PIPELINE_META_DIR}/.lock"
  WORKTREE_BASE_DIR="${PIPELINE_META_ROOT}/worktrees"
  LOG_FILE="${LOG_DIR}/${RUN_TIMESTAMP}.log"
  OTEL_FILE="${OTEL_DIR}/${RUN_TIMESTAMP}.jsonl"
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

validate_task_id() {
  local id="$1"
  if [[ ! "$id" =~ ^T-[0-9]+(\.[0-9]+)?$ ]]; then
    log "ERROR: invalid task ID: $id"
    return 1
  fi
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
  local template_path
  case "$1" in
    sdd) template_path="${TEMPLATE_DIR}/sdd-v1.json" ;;
    slc) template_path="${TEMPLATE_DIR}/slc-v1.json" ;;
    tdd) template_path="${TEMPLATE_DIR}/tdd-v1.json" ;;
    lightweight) template_path="${TEMPLATE_DIR}/lightweight-v1.json" ;;
    *) return 1 ;;
  esac

  if [[ ! -f "$template_path" ]]; then
    log "ERROR: missing template for pipeline '$1': ${template_path}"
    return 1
  fi

  printf '%s\n' "$template_path"
}

render_template_definition() {
  local template_path="$1"
  local task_id="$2"
  local task_spec="$3"
  local worktree_path="$4"
  local crate="$5"
  local file_paths="$6"
  local rendered

  if ! rendered="$(
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
    )"; then
    log "ERROR: failed to render template ${template_path}"
    return 1
  fi

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
  local template_id="$3"
  local session_id="$4"
  local wall_clock_seconds="$5"
  local status="$6"
  local first_pass_clean="$7"
  local remediation_cycles="$8"
  local metrics_path="${METRICS_DIR}/${task_id}.json"
  local metrics_json

  metrics_json="$(
    jq -nc \
      --arg taskId "$task_id" \
      --arg sessionId "$session_id" \
      --arg template "$template_id" \
      --arg pipeline "$pipeline" \
      --arg status "$status" \
      --argjson wallClockSeconds "$wall_clock_seconds" \
      --argjson firstPassClean "$first_pass_clean" \
      --argjson remediationCycles "$remediation_cycles" \
      --arg timestamp "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
      '{
        task_id: $taskId,
        taskId: $taskId,
        session_id: $sessionId,
        sessionId: $sessionId,
        template: $template,
        pipeline: $pipeline,
        wall_clock_seconds: $wallClockSeconds,
        wallClockSeconds: $wallClockSeconds,
        status: $status,
        first_pass: $firstPassClean,
        firstPassClean: $firstPassClean,
        remediation_cycles: $remediationCycles,
        node_timings: {},
        approx_tokens: {
          input: 0,
          output: 0
        },
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

extract_remediation_cycles() {
  printf '%s' "$1" | jq -r '
    (.data? // .) as $root
    | ([ $root.nodes[]? | select(.status == "completed" and .nodeId == "remediate") ] | length)
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
    ( set -C; printf '%s\n' "$$" >"$LOCK_FILE" ) 2>/dev/null || {
      log "ERROR: another instance is running (lock file exists)"
      exit 1
    }
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
      FAILURES=$((FAILURES + 1))
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
      if ! wait "$pid"; then
        FAILURES=$((FAILURES + 1))
      fi
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

sha256_file() {
  shasum -a 256 "$1" | awk '{print $1}'
}

verify_script_hash() {
  local expected_hash="$1"
  local phase="$2"
  local current_hash
  current_hash="$(sha256_file "$SCRIPT_PATH")"
  if [[ "$current_hash" != "$expected_hash" ]]; then
    log "ERROR: immutable script hash changed during ${phase}"
    log "expected=${expected_hash}"
    log "actual=${current_hash}"
    exit 1
  fi
}

latest_summary_file() {
  local latest_file
  latest_file="$(
    find "$METRICS_DIR" -maxdepth 1 -type f -name 'summary-*.json' | LC_ALL=C sort | tail -1
  )"
  [[ -n "$latest_file" ]] || return 1
  printf '%s\n' "$latest_file"
}

clear_live_metrics() {
  find "$METRICS_DIR" -maxdepth 1 -type f -name '*.json' -exec rm -f {} +
}

snapshot_templates() {
  local destination="$1"
  mkdir -p "$destination"
  find "$TEMPLATE_DIR" -maxdepth 1 -type f -name '*.json' -exec cp {} "$destination/" \;
}

restore_template_snapshot() {
  local snapshot_dir="$1"
  find "$TEMPLATE_DIR" -maxdepth 1 -type f -name '*.json' -exec rm -f {} +
  find "$snapshot_dir" -maxdepth 1 -type f -name '*.json' -exec cp {} "$TEMPLATE_DIR/" \;
}

changed_templates_from_snapshot() {
  local snapshot_dir="$1"
  local name
  local current_file
  local snapshot_file

  while IFS= read -r name; do
    [[ -n "$name" ]] || continue
    current_file="${TEMPLATE_DIR}/${name}"
    snapshot_file="${snapshot_dir}/${name}"
    if [[ ! -f "$current_file" || ! -f "$snapshot_file" ]]; then
      printf '%s\n' "$current_file"
      continue
    fi
    if ! cmp -s "$snapshot_file" "$current_file"; then
      printf '%s\n' "$current_file"
    fi
  done < <(
    {
      find "$snapshot_dir" -maxdepth 1 -type f -name '*.json' -exec basename {} \;
      find "$TEMPLATE_DIR" -maxdepth 1 -type f -name '*.json' -exec basename {} \;
    } | LC_ALL=C sort -u
  )
}

archive_live_metrics() {
  local destination="$1"
  mkdir -p "$destination"
  find "$METRICS_DIR" -maxdepth 1 -type f -name '*.json' -exec cp {} "$destination/" \;
}

metrics_effectiveness() {
  jq -r '.effectiveness // 0' "$1"
}

metrics_failed_task_rate() {
  jq -r '
    if (.total_tasks // 0) > 0 then
      ((.total_tasks - .completed_tasks) / .total_tasks)
    else
      0
    end
  ' "$1"
}

write_meta_baseline() {
  local summary_file="$1"
  local effectiveness="$2"
  local failed_task_rate="$3"
  jq -nc \
    --arg timestamp "$(date -u +"%Y-%m-%dT%H:%M:%SZ")" \
    --arg summaryFile "$summary_file" \
    --argjson effectiveness "$effectiveness" \
    --argjson failedTaskRate "$failed_task_rate" \
    '{
      timestamp: $timestamp,
      summary_file: $summaryFile,
      effectiveness: $effectiveness,
      failed_task_rate: $failedTaskRate
    }' >"$META_BASELINE_FILE"
}

template_context_for_prompt() {
  local file
  local first=true
  for file in "$TEMPLATE_DIR"/*.json; do
    [[ -f "$file" ]] || continue
    if [[ "$first" == "false" ]]; then
      printf '\n'
    fi
    first=false
    printf '### %s\n' "$(basename "$file")"
    printf '```json\n'
    cat "$file"
    printf '\n```\n'
  done
}

dispatch_meta_agent() {
  local iteration="$1"
  local summary_file="$2"
  local verdict_file="$3"
  local previous_verdict_file="$4"
  local baseline_score="$5"
  local baseline_failed_rate="$6"
  local current_score="$7"
  local current_failed_rate="$8"
  local previous_verdict_json="null"
  local prompt
  local payload

  if [[ -n "$previous_verdict_file" && -f "$previous_verdict_file" ]]; then
    previous_verdict_json="$(cat "$previous_verdict_file")"
  fi

  prompt="$(cat <<EOF
You are the meta-optimization agent for pipeline templates.

Iteration: ${iteration}
Pipeline root: ${PIPELINE_META_DIR}
Template directory: ${TEMPLATE_DIR}
Verdict path: ${verdict_file}
Immutable shell file: ${SCRIPT_PATH}

Objective:
- Analyze the latest pipeline telemetry and current templates.
- Propose exactly ONE template mutation in exactly ONE file under ${TEMPLATE_DIR}.
- Do not edit ${SCRIPT_PATH} or any shell file.
- Do not edit more than one template file.
- Write a verdict JSON to ${verdict_file}.

Optimizable knobs:
1. Per-node model assignment
2. Per-node maxTurns
3. Per-node timeout
4. Retry policies (retryMaxAttempts, backoff)
5. Conditional edge thresholds
6. Node prompts
7. adopt roles/colors
8. allowedTools per node
9. maxConcurrency
10. Node decomposition (split swe into draft+refine)

Shell keep/discard policy:
- current_score >= baseline_score + threshold
- current_failed_task_rate <= baseline_failed_task_rate
- your verdict has kept=true

Baseline:
- baseline_score=${baseline_score}
- baseline_failed_task_rate=${baseline_failed_rate}
- score_threshold=${PIPELINE_META_META_THRESHOLD}

Current iteration telemetry summary (${summary_file}):
$(cat "$summary_file")

Current effectiveness=${current_score}
Current failed_task_rate=${current_failed_rate}

Previous verdict:
${previous_verdict_json}

Current templates:
$(template_context_for_prompt)

Write ${verdict_file} as valid JSON with this exact schema:
{
  "kept": true,
  "targetTemplate": "sdd-v1.json",
  "changeSummary": "one sentence",
  "hypothesis": "one sentence",
  "rationale": "short paragraph"
}

If you do not recommend keeping the change, set kept=false, but still write the verdict file.
EOF
)"

  if [[ -n "$PIPELINE_META_DRY_RUN" ]]; then
    log "[dry-run] Would dispatch meta-agent ${PIPELINE_META_META_AGENT} for iteration ${iteration}"
    return 0
  fi

  payload="$(
    jq -nc \
      --arg agent "$PIPELINE_META_META_AGENT" \
      --arg cwd "$PIPELINE_META_ROOT" \
      --arg prompt "$prompt" \
      --argjson timeoutSeconds "$PIPELINE_META_META_TIMEOUT_SECONDS" \
      '{
        agent: $agent,
        mode: "prompt",
        cwd: $cwd,
        prompt: $prompt,
        autoGrantPermissions: true,
        timeoutSeconds: $timeoutSeconds
      }'
  )"

  maenifold tool AcpExec "$payload" --json
}

process_task() {
  trap - EXIT INT TERM

  local task_row_json="$1"
  local task_id task_path gate_pipeline task_file frontmatter_task_id frontmatter_pipeline
  local crate file_paths task_spec template_path template_id worktree_path branch_name rendered_definition
  local submit_json session_id driver_status view_json pipeline_status all_verdicts_pass
  local first_pass_clean remediation_cycles status start_epoch end_epoch wall_clock_seconds ambient_prompt

  task_id="$(printf '%s' "$task_row_json" | jq -r '.taskId')"
  task_path="$(printf '%s' "$task_row_json" | jq -r '.path')"
  gate_pipeline="$(printf '%s' "$task_row_json" | jq -r '.pipeline // empty')"
  task_file="$task_path"
  start_epoch="$(date +%s)"
  status="failed"
  first_pass_clean=false
  remediation_cycles=0
  session_id=""

  log "Starting task ${task_id}"

  if [[ ! -f "$task_file" ]]; then
    log "ERROR: task file missing for ${task_id}: ${task_file}"
    return 1
  fi

  frontmatter_task_id="$(frontmatter_value "$task_file" "id")"
  if [[ -n "$frontmatter_task_id" ]]; then
    task_id="$frontmatter_task_id"
  fi

  frontmatter_pipeline="$(frontmatter_value "$task_file" "pipeline")"
  if [[ -n "$frontmatter_pipeline" ]]; then
    gate_pipeline="$frontmatter_pipeline"
  fi

  if ! validate_task_id "$task_id"; then
    return 1
  fi

  crate="$(frontmatter_value "$task_file" "crate")"
  file_paths="$(frontmatter_value "$task_file" "file_scope")"
  task_spec="$(<"$task_file")"

  if ! template_path="$(template_for_pipeline "$gate_pipeline")"; then
    log "WARN: unsupported pipeline '${gate_pipeline}' for ${task_id}; skipping"
    if [[ -z "$PIPELINE_META_DRY_RUN" ]]; then
      end_epoch="$(date +%s)"
      wall_clock_seconds=$((end_epoch - start_epoch))
      record_metrics "$task_id" "$gate_pipeline" "unknown" "" "$wall_clock_seconds" "failed" false 0
    fi
    return 1
  fi
  template_id="$(basename "$template_path" .json)"

  worktree_path="${WORKTREE_BASE_DIR}/${task_id}"
  branch_name="task/${task_id}"
  ambient_prompt="Execute gate task ${task_id}"

  if ! rendered_definition="$(render_template_definition "$template_path" "$task_id" "$task_spec" "$worktree_path" "$crate" "$file_paths")"; then
    log "ERROR: template rendering failed for ${task_id}"
    if [[ -z "$PIPELINE_META_DRY_RUN" ]]; then
      end_epoch="$(date +%s)"
      wall_clock_seconds=$((end_epoch - start_epoch))
      record_metrics "$task_id" "$gate_pipeline" "$template_id" "" "$wall_clock_seconds" "failed" false 0
    fi
    return 1
  fi

  if [[ -n "$PIPELINE_META_DRY_RUN" ]]; then
    log "[dry-run] Would create worktree: git worktree add ${worktree_path} -b ${branch_name}"
    log "[dry-run] Would sync gate cache in worktree: gate sync --root ${worktree_path}"
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
    record_metrics "$task_id" "$gate_pipeline" "$template_id" "" "$wall_clock_seconds" "failed" false 0
    return 1
  fi

  if ! git -C "$PIPELINE_META_ROOT" worktree add "$worktree_path" -b "$branch_name"; then
    log "ERROR: failed to create worktree for ${task_id}"
    end_epoch="$(date +%s)"
    wall_clock_seconds=$((end_epoch - start_epoch))
    record_metrics "$task_id" "$gate_pipeline" "$template_id" "" "$wall_clock_seconds" "failed" false 0
    return 1
  fi

  if ! gate sync --root "$worktree_path"; then
    log "ERROR: failed to sync gate cache in worktree for ${task_id}"
    end_epoch="$(date +%s)"
    wall_clock_seconds=$((end_epoch - start_epoch))
    record_metrics "$task_id" "$gate_pipeline" "$template_id" "" "$wall_clock_seconds" "failed" false 0
    log "FAILED: worktree preserved at ${worktree_path}"
    return 1
  fi

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
    record_metrics "$task_id" "$gate_pipeline" "$template_id" "" "$wall_clock_seconds" "failed" false 0
    log "FAILED: worktree preserved at ${worktree_path}"
    return 1
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
    record_metrics "$task_id" "$gate_pipeline" "$template_id" "$session_id" "$wall_clock_seconds" "failed" false 0
    log "FAILED: worktree preserved at ${worktree_path}"
    return 1
  fi

  pipeline_status="$(extract_pipeline_status "$view_json")"
  all_verdicts_pass="$(extract_all_verdicts_pass "$view_json")"
  first_pass_clean="$(extract_first_pass_clean "$view_json")"
  remediation_cycles="$(extract_remediation_cycles "$view_json")"

  if [[ "$pipeline_status" == "completed" && "$all_verdicts_pass" == "true" ]]; then
    if merge_and_cleanup_worktree "$task_id" "$branch_name" "$worktree_path"; then
      status="completed"
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
  record_metrics "$task_id" "$gate_pipeline" "$template_id" "$session_id" "$wall_clock_seconds" "$status" "$first_pass_clean" "$remediation_cycles"

  if [[ "$status" == "completed" ]]; then
    return 0
  fi

  return 1
}

run_inner_loop() {
  local backlog_json
  local task_row
  local limit_json="null"

  FAILURES=0
  JOB_PIDS=()
  CHILD_PIDS=()

  if [[ -n "$PIPELINE_META_LIMIT" ]]; then
    limit_json="$PIPELINE_META_LIMIT"
  fi

  if ! backlog_json="$(gate status --root "$PIPELINE_META_ROOT" --json 2>/dev/null)"; then
    if [[ -n "$PIPELINE_META_DRY_RUN" ]]; then
      log "WARN: gate status unavailable; using todo/ fallback backlog for dry-run"
      backlog_json="$(fallback_backlog_json)"
    else
      log "ERROR: gate status failed for root=${PIPELINE_META_ROOT}"
      return 1
    fi
  fi

  while IFS= read -r task_row; do
    [[ -n "$task_row" ]] || continue
    wait_for_open_slot
    process_task "$task_row" &
    JOB_PIDS+=("$!")
    CHILD_PIDS+=("$!")
  done < <(
    printf '%s' "$backlog_json" | jq -c \
      --arg taskId "$PIPELINE_META_TASK_ID" \
      --arg pipeline "$PIPELINE_META_PIPELINE" \
      --argjson limit "$limit_json" '
      def numeric_parts:
        (.taskId // "")
        | ltrimstr("T-")
        | split(".")
        | map(tonumber? // 0);
      map(select((.state // "") == "pending" and (.location // "") == "todo"))
      | if $taskId != "" then map(select((.taskId // "") == $taskId)) else . end
      | if $pipeline != "" then map(select((.pipeline // "") == $pipeline)) else . end
      | sort_by((numeric_parts[0] // 0), (numeric_parts[1] // 0), (.taskId // ""))
      | if $limit != null then .[:$limit] else . end
      | .[]
    '
  )

  wait_for_all_jobs || true
  if [[ "$FAILURES" -gt 0 ]]; then
    log "WARN: ${FAILURES} task(s) failed"
    return 1
  fi

  return 0
}

meta_outer_loop() {
  # Delayed evaluation pattern:
  # Each iteration scores the PREVIOUS iteration's mutation.
  # Iteration 1: run original templates → establish baseline → meta proposes A (warm-up accept)
  # Iteration 2: run with A → score evaluates A → meta proposes B → keep/discard B based on A's score
  # Iteration N: score reflects mutation from N-1; keep/discard decision applies to N's proposal
  # This is standard hill-climbing: accept next mutation if current mutation improved on baseline.
  local meta_iteration
  local snapshot_dir
  local verdict_iteration_dir
  local verdict_file
  local previous_verdict_file=""
  local summary_file
  local current_score
  local current_failed_rate
  local baseline_exists=false
  local baseline_score
  local baseline_failed_rate
  local expected_hash
  local changed_templates=()
  local changed_template
  local changed_count
  local verdict_kept
  local verdict_target
  local keep_change
  local archive_dir

  mkdir -p "$META_SNAPSHOT_DIR" "$META_VERDICT_DIR" "$META_METRICS_ARCHIVE_DIR"

  for ((meta_iteration = 1; meta_iteration <= PIPELINE_META_MAX_META_ITERATIONS; meta_iteration++)); do
    log "meta iteration ${meta_iteration}/${PIPELINE_META_MAX_META_ITERATIONS} starting"
    snapshot_dir="${META_SNAPSHOT_DIR}/meta-$(printf '%02d' "$meta_iteration")"
    verdict_iteration_dir="${META_VERDICT_DIR}/meta-$(printf '%02d' "$meta_iteration")"
    verdict_file="${verdict_iteration_dir}/meta-verdict.json"
    archive_dir="${META_METRICS_ARCHIVE_DIR}/meta-$(printf '%02d' "$meta_iteration")"

    rm -rf "$snapshot_dir" "$verdict_iteration_dir" "$archive_dir"
    mkdir -p "$verdict_iteration_dir" "$archive_dir"
    snapshot_templates "$snapshot_dir"
    clear_live_metrics

    log "meta iteration ${meta_iteration}: running ${PIPELINE_META_INNER_ITERATIONS} inner iteration(s)"
    for ((inner_iteration = 1; inner_iteration <= PIPELINE_META_INNER_ITERATIONS; inner_iteration++)); do
      log "meta iteration ${meta_iteration}: inner iteration ${inner_iteration}/${PIPELINE_META_INNER_ITERATIONS}"
      if ! run_inner_loop; then
        log "WARN: inner loop reported failures during meta iteration ${meta_iteration}"
      fi
    done

    "${PIPELINE_META_DIR}/score.sh" >/dev/null
    summary_file="$(latest_summary_file)"
    archive_live_metrics "$archive_dir"

    current_score="$(metrics_effectiveness "$summary_file")"
    current_failed_rate="$(metrics_failed_task_rate "$summary_file")"

    if [[ -f "$META_BASELINE_FILE" ]]; then
      baseline_exists=true
      baseline_score="$(jq -r '.effectiveness // 0' "$META_BASELINE_FILE")"
      baseline_failed_rate="$(jq -r '.failed_task_rate // 0' "$META_BASELINE_FILE")"
    else
      baseline_exists=false
      baseline_score="$current_score"
      baseline_failed_rate="$current_failed_rate"
      write_meta_baseline "$summary_file" "$current_score" "$current_failed_rate"
      log "meta iteration ${meta_iteration}: established initial baseline score=${current_score} failed_task_rate=${current_failed_rate}"
    fi

    expected_hash="$(sha256_file "$SCRIPT_PATH")"
    verify_script_hash "$expected_hash" "meta pre-dispatch"

    if ! dispatch_meta_agent \
      "$meta_iteration" \
      "$summary_file" \
      "$verdict_file" \
      "$previous_verdict_file" \
      "$baseline_score" \
      "$baseline_failed_rate" \
      "$current_score" \
      "$current_failed_rate"; then
      log "WARN: meta-agent dispatch failed for iteration ${meta_iteration}"
    fi

    verify_script_hash "$expected_hash" "meta post-dispatch"

    changed_templates=()
    while IFS= read -r changed_template; do
      [[ -n "$changed_template" ]] || continue
      changed_templates+=("$changed_template")
    done < <(changed_templates_from_snapshot "$snapshot_dir")
    changed_count="${#changed_templates[@]}"

    keep_change=false
    if [[ ! -f "$verdict_file" ]]; then
      log "meta iteration ${meta_iteration}: missing verdict file; restoring snapshot"
    elif ! jq -e '.kept | type == "boolean"' "$verdict_file" >/dev/null 2>&1; then
      log "meta iteration ${meta_iteration}: malformed verdict file; restoring snapshot"
    elif ! jq -e 'has("targetTemplate") and has("changeSummary") and has("rationale")' "$verdict_file" >/dev/null 2>&1; then
      log "meta iteration ${meta_iteration}: verdict missing required fields (targetTemplate, changeSummary, rationale); restoring snapshot"
    elif [[ "$changed_count" -ne 1 ]]; then
      log "meta iteration ${meta_iteration}: expected exactly one changed template, found ${changed_count}; restoring snapshot"
    else
      verdict_kept="$(jq -r '.kept' "$verdict_file")"
      verdict_target="$(jq -r '.targetTemplate // empty' "$verdict_file")"
      if [[ -n "$verdict_target" && "$(basename "${changed_templates[0]}")" != "$verdict_target" ]]; then
        log "meta iteration ${meta_iteration}: verdict target ${verdict_target} does not match changed template $(basename "${changed_templates[0]}"); restoring snapshot"
      elif [[ "$baseline_exists" == "false" ]]; then
        if [[ "$verdict_kept" == "true" ]]; then
          keep_change=true
          log "meta iteration ${meta_iteration}: warm-up mutation accepted for next iteration"
        fi
      elif [[ "$verdict_kept" == "true" ]] && \
        awk -v current="$current_score" -v baseline="$baseline_score" -v threshold="$PIPELINE_META_META_THRESHOLD" 'BEGIN { exit !(current >= baseline + threshold) }' && \
        awk -v current="$current_failed_rate" -v baseline="$baseline_failed_rate" 'BEGIN { exit !(current <= baseline) }'; then
        keep_change=true
      fi
    fi

    if [[ "$keep_change" == "true" ]]; then
      log "meta iteration ${meta_iteration}: accepted template mutation (score=${current_score}, failed_task_rate=${current_failed_rate})"
      write_meta_baseline "$summary_file" "$current_score" "$current_failed_rate"
    else
      restore_template_snapshot "$snapshot_dir"
      log "meta iteration ${meta_iteration}: rejected template mutation and restored snapshot"
    fi

    previous_verdict_file="$verdict_file"
  done
}

main() {
  parse_args "$@"
  configure_paths

  mkdir -p "$LOG_DIR" "$METRICS_DIR" "$OTEL_DIR"
  export COPILOT_OTEL_FILE_EXPORTER_PATH="$OTEL_FILE"
  create_lock

  exec > >(tee -a "$LOG_FILE") 2>&1
  trap cleanup EXIT INT TERM

  require_command git
  require_command jq
  require_command gate
  require_command maenifold
  require_command maenifold-pipeline-driver
  require_command shasum

  if [[ -n "$PIPELINE_META_TASK_ID" ]] && ! validate_task_id "$PIPELINE_META_TASK_ID"; then
    exit 1
  fi
  if [[ -n "$PIPELINE_META_LIMIT" ]] && ! [[ "$PIPELINE_META_LIMIT" =~ ^[1-9][0-9]*$ ]]; then
    log "ERROR: PIPELINE_META_LIMIT/--limit must be a positive integer"
    exit 1
  fi
  if [[ -n "$PIPELINE_META_PIPELINE" && ! "$PIPELINE_META_PIPELINE" =~ ^(sdd|slc|tdd|lightweight)$ ]]; then
    log "ERROR: PIPELINE_META_PIPELINE/--pipeline must be one of: sdd, slc, tdd, lightweight"
    exit 1
  fi
  if ! [[ "$PIPELINE_META_MAX_PARALLEL" =~ ^[1-9][0-9]*$ ]]; then
    log "ERROR: PIPELINE_META_MAX_PARALLEL must be a positive integer"
    exit 1
  fi
  if ! [[ "$PIPELINE_META_INNER_ITERATIONS" =~ ^[1-9][0-9]*$ ]]; then
    log "ERROR: PIPELINE_META_INNER_ITERATIONS must be a positive integer"
    exit 1
  fi
  if ! [[ "$PIPELINE_META_MAX_META_ITERATIONS" =~ ^[1-9][0-9]*$ ]]; then
    log "ERROR: PIPELINE_META_MAX_META_ITERATIONS must be a positive integer"
    exit 1
  fi

  log "pipeline-meta.sh starting"
  log "root=${PIPELINE_META_ROOT}"
  log "max_parallel=${PIPELINE_META_MAX_PARALLEL}"
  log "inner_iterations=${PIPELINE_META_INNER_ITERATIONS}"
  log "meta_enabled=${PIPELINE_META_ENABLE_META}"
  log "max_meta_iterations=${PIPELINE_META_MAX_META_ITERATIONS}"
  log "task_filter=${PIPELINE_META_TASK_ID:-all}"
  log "pipeline_filter=${PIPELINE_META_PIPELINE:-all}"
  log "limit=${PIPELINE_META_LIMIT:-all}"
  log "dry_run=${PIPELINE_META_DRY_RUN:+true}"
  log "log_file=${LOG_FILE}"
  log "otel=${OTEL_FILE}"

  if [[ "$PIPELINE_META_ENABLE_META" == "true" ]]; then
    meta_outer_loop
  else
    run_inner_loop || exit 1
  fi

  log "pipeline-meta.sh finished"
}

main "$@"
