#!/usr/bin/env bash
set -euo pipefail

command -v jq >/dev/null 2>&1 || { echo "ERROR: jq is required" >&2; exit 1; }

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
METRICS_DIR="${SCRIPT_DIR}/metrics"
BASELINE_FILE="${SCRIPT_DIR}/baseline.json"
RUN_STAMP="$(date -u +"%Y%m%dT%H%M%SZ")"
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
SUMMARY_FILE="${METRICS_DIR}/summary-${RUN_STAMP}.json"

mkdir -p "$METRICS_DIR"

files=()
while IFS= read -r file; do
  [[ -n "$file" ]] || continue
  files+=("$file")
done < <(find "$METRICS_DIR" -maxdepth 1 -type f -name '*.json' ! -name 'summary-*.json' | LC_ALL=C sort)

valid_files=()
for file in "${files[@]}"; do
  if jq empty "$file" >/dev/null 2>&1; then
    valid_files+=("$file")
  else
    echo "WARNING: skipping malformed metric file: $file" >&2
  fi
done

if [[ "${#files[@]}" -gt 0 && "${#valid_files[@]}" -eq 0 ]]; then
  echo "No valid metric files found in ${METRICS_DIR}" >&2
  exit 0
fi

if [[ "${#valid_files[@]}" -gt 0 ]]; then
  aggregate="$(
    jq -s '
      map(select(type == "object" and has("task_id"))) as $tasks
      | ($tasks | map(select(.status == "completed"))) as $completed
      | {
          total_tasks: ($tasks | length),
          completed_tasks: ($completed | length),
          first_pass_tasks: ($completed | map(select(.first_pass == true)) | length),
          avg_remediation_cycles: ($completed | map(.remediation_cycles // 0) | if length > 0 then add / length else 0 end),
          avg_wall_clock: ($completed | map(.wall_clock_seconds // 0) | if length > 0 then add / length else 0 end),
          avg_total_tokens: ($completed | map((.approx_tokens.input // 0) + (.approx_tokens.output // 0)) | if length > 0 then add / length else 0 end)
        }
    ' "${valid_files[@]}"
  )"
else
  aggregate='{"total_tasks":0,"completed_tasks":0,"first_pass_tasks":0,"avg_remediation_cycles":0,"avg_wall_clock":0,"avg_total_tokens":0}'
fi

IFS=$'\t' read -r total_tasks completed_tasks first_pass_tasks avg_remediation avg_wall_clock avg_total_tokens <<<"$(jq -r '[.total_tasks,.completed_tasks,.first_pass_tasks,.avg_remediation_cycles,.avg_wall_clock,.avg_total_tokens] | @tsv' <<<"$aggregate")"
IFS=$'\t' read -r delivery_score first_pass_score remediation_score time_score cost_score effectiveness <<<"$(awk -v total="$total_tasks" -v completed="$completed_tasks" -v first="$first_pass_tasks" -v remediation="$avg_remediation" -v wall="$avg_wall_clock" -v tokens="$avg_total_tokens" 'BEGIN {
  delivery = total > 0 ? completed / total : 0
  first_pass = completed > 0 ? first / completed : 0
  remediation_score = completed > 0 ? 1 - (remediation / 3) : 0
  if (remediation_score < 0) remediation_score = 0
  time_ratio = completed > 0 ? wall / 600 : 1
  cost_ratio = completed > 0 ? tokens / 100000 : 1
  time_score = 1 - (time_ratio < 1 ? time_ratio : 1)
  cost_score = 1 - (cost_ratio < 1 ? cost_ratio : 1)
  effectiveness = 100 * (0.35 * delivery + 0.25 * first_pass + 0.15 * remediation_score + 0.15 * time_score + 0.10 * cost_score)
  printf "%.6f\t%.6f\t%.6f\t%.6f\t%.6f\t%.6f\n", delivery, first_pass, remediation_score, time_score, cost_score, effectiveness
}')"

baseline_effectiveness='null'
delta='null'
if [[ -f "$BASELINE_FILE" ]]; then
  existing_baseline="$(jq -r '.effectiveness' "$BASELINE_FILE")" || {
    echo "ERROR: baseline.json is corrupted. Delete it to reset, or fix the file." >&2
    exit 1
  }
  baseline_effectiveness="$(awk -v value="$existing_baseline" 'BEGIN { printf "%.6f", value }')"
  delta="$(awk -v current="$effectiveness" -v baseline="$existing_baseline" 'BEGIN { printf "%.6f", current - baseline }')"
else
  jq -n --arg timestamp "$TIMESTAMP" --argjson effectiveness "$effectiveness" '{timestamp:$timestamp,effectiveness:$effectiveness}' >"$BASELINE_FILE"
fi

jq -n \
  --arg timestamp "$TIMESTAMP" \
  --argjson total_tasks "$total_tasks" \
  --argjson completed_tasks "$completed_tasks" \
  --argjson delivery "$delivery_score" \
  --argjson first_pass "$first_pass_score" \
  --argjson remediation "$remediation_score" \
  --argjson time "$time_score" \
  --argjson cost "$cost_score" \
  --argjson effectiveness "$effectiveness" \
  --argjson baseline "$baseline_effectiveness" \
  --argjson delta "$delta" \
  '{
    timestamp: $timestamp,
    total_tasks: $total_tasks,
    completed_tasks: $completed_tasks,
    scores: {delivery: $delivery, first_pass: $first_pass, remediation: $remediation, time: $time, cost: $cost},
    effectiveness: $effectiveness,
    baseline_effectiveness: $baseline,
    delta: $delta
  }' >"$SUMMARY_FILE"

printf 'Pipeline effectiveness: %.1f\n' "$effectiveness"
printf 'Tasks: %s total, %s completed\n' "$total_tasks" "$completed_tasks"
printf 'Scores: delivery=%.2f first_pass=%.2f remediation=%.2f time=%.2f cost=%.2f\n' \
  "$delivery_score" "$first_pass_score" "$remediation_score" "$time_score" "$cost_score"
if [[ "$baseline_effectiveness" == "null" ]]; then
  printf 'Baseline established at %.1f\n' "$effectiveness"
else
  printf 'Baseline: %.1f (delta %+.1f)\n' "$baseline_effectiveness" "$delta"
fi
printf 'Summary: %s\n' "$SUMMARY_FILE"
