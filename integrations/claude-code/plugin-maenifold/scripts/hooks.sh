#!/usr/bin/env bash
#
# Graph RAG Hook - Claude Code + Maenifold Integration
# Usage: hooks.sh {session_start|task_augment|subagent_stop}
#

set -euo pipefail

MODE="${1:-}"
[[ -z "$MODE" ]] && { echo '{"error":"Usage: hooks.sh {session_start|task_augment|subagent_stop}"}' >&2; exit 1; }

# --- Shared Functions ---

# Timeout wrapper for CLI calls (macOS-compatible, no GNU timeout required)
# Usage: run_with_timeout <timeout_seconds> <command> [args...]
# Returns: Command output on success, empty string on timeout/error
run_with_timeout() {
  local timeout="${1:-5}"; shift
  local output_file
  output_file=$(mktemp)
  local status_file
  status_file=$(mktemp)

  # Run command in background, capture output and status
  ( ("$@" > "$output_file" 2>/dev/null && echo "0" > "$status_file") || echo "1" > "$status_file" ) &
  local pid=$!

  # Wait for timeout or completion (count in deciseconds: timeout * 10)
  local elapsed=0
  local timeout_deciseconds=$((timeout * 10))
  while kill -0 "$pid" 2>/dev/null && (( elapsed < timeout_deciseconds )); do
    sleep 0.1
    elapsed=$((elapsed + 1))
  done

  # If still running after timeout, kill it
  if kill -0 "$pid" 2>/dev/null; then
    kill -9 "$pid" 2>/dev/null
    wait "$pid" 2>/dev/null
    rm -f "$output_file" "$status_file"
    echo ""
  else
    # Process completed, return output
    wait "$pid" 2>/dev/null
    cat "$output_file"
    rm -f "$output_file" "$status_file"
  fi
}

find_cli() {
  # 1. Check PATH first
  if command -v maenifold &>/dev/null; then
    command -v maenifold
    return 0
  fi

  # 2. Fall back to ~/maenifold/bin
  if [[ -x "$HOME/maenifold/bin/maenifold" ]]; then
    echo "$HOME/maenifold/bin/maenifold"
    return 0
  fi

  # 3. Error if neither found
  echo "ERROR: maenifold not found in PATH or ~/maenifold/bin" >&2
  echo "Install from: https://github.com/msbrettorg/maenifold/releases/latest" >&2
  return 1
}

MAENIFOLD_CLI=$(find_cli || true)
CLI_TIMEOUT="${CLI_TIMEOUT:-5}"
DB_PATH="$HOME/maenifold/memory.db"

# --- Shared: build_graph_context ---
# One code path for both session_start and task_augment.
# Returns: graph of thought priming + recent activity threads.
build_graph_context() {
  local output=""

  # SECTION 1: Graph of Thought (structural priming)
  # Community index from SQLite — concept names grouped by reasoning domain.
  local graph_output=""
  if [[ -f "$DB_PATH" ]]; then
    graph_output=$(sqlite3 "$DB_PATH" "
      WITH recent_thinking AS (
        SELECT file_path
        FROM file_content
        WHERE file_path LIKE '%thinking/sequential/%'
          AND last_indexed > datetime('now', '-3 days')
      ),
      recency_boost AS (
        SELECT DISTINCT cm.concept_name, 1 as is_recent
        FROM concept_mentions cm
        JOIN recent_thinking rt ON cm.source_file = rt.file_path
      ),
      community_sizes AS (
        SELECT community_id, COUNT(*) as size
        FROM concept_communities
        GROUP BY community_id
        HAVING size >= 3
      ),
      concept_degree AS (
        SELECT cc.concept_name, cc.community_id,
               SUM(cg.co_occurrence_count) as total_weight,
               COALESCE(rb.is_recent, 0) as is_recent
        FROM concept_communities cc
        JOIN community_sizes cs ON cc.community_id = cs.community_id
        LEFT JOIN concept_graph cg
          ON cc.concept_name = cg.concept_a OR cc.concept_name = cg.concept_b
        LEFT JOIN recency_boost rb ON cc.concept_name = rb.concept_name
        WHERE cc.concept_name NOT GLOB '*[.]*'
          AND length(cc.concept_name) >= 3
          AND cc.concept_name NOT GLOB '[0-9]*'
        GROUP BY cc.concept_name, cc.community_id
      ),
      ranked AS (
        SELECT *,
          ROW_NUMBER() OVER (
            PARTITION BY community_id
            ORDER BY is_recent DESC, total_weight DESC
          ) as rn
        FROM concept_degree
        WHERE total_weight > 0
      )
      SELECT community_id, group_concat(concept_name, ' ')
      FROM ranked
      WHERE rn <= 20
      GROUP BY community_id
      ORDER BY MAX(is_recent) DESC, MAX(total_weight) DESC
    " 2>/dev/null || true)
  fi

  local priming=""
  if [[ -n "$graph_output" ]]; then
    while IFS='|' read -r cid concepts; do
      [[ -z "$concepts" ]] && continue
      local first=true line=""
      for c in $concepts; do
        if $first; then
          line="**[[$c]]**"
          first=false
        else
          line+=" [[$c]]"
        fi
      done
      priming+="$line"$'\n'
    done <<< "$graph_output"
  fi

  # SECTION 2: Recent Activity (temporal)
  local thread_index="" thread_count=0
  if [[ -n "$MAENIFOLD_CLI" ]]; then
    local ra_output
    ra_output=$(run_with_timeout "$CLI_TIMEOUT" "$MAENIFOLD_CLI" --tool RecentActivity --payload \
      '{"filter":"thinking","limit":5,"timespan":"3.00:00:00","includeContent":false}')

    if [[ -n "$ra_output" ]]; then
      local current_sid="" current_status="" current_tcount=""

      while IFS= read -r line; do
        local header_sid
        header_sid=$(echo "$line" | grep -oE 'session-[0-9]+-[0-9]+' || true)
        [[ -z "$header_sid" ]] && header_sid=$(echo "$line" | grep -oE 'workflow-[0-9]+' || true)

        if [[ -n "$header_sid" ]] && echo "$line" | grep -qF '**' 2>/dev/null; then
          if [[ -n "$current_sid" && $thread_count -lt 5 ]]; then
            local entry="$current_sid (${current_status:-unknown}, ${current_tcount:-0}t)"
            thread_index="${thread_index:+$thread_index | }$entry"
            thread_count=$((thread_count + 1))
          fi
          current_sid="$header_sid"; current_status=""; current_tcount=""
          continue
        fi

        if echo "$line" | grep -qE '^ *Status:' 2>/dev/null; then
          current_status=$(echo "$line" | grep -oE '(active|completed|abandoned)' || true)
        elif echo "$line" | grep -qE '^ *(Thoughts|Steps):' 2>/dev/null; then
          current_tcount=$(echo "$line" | grep -oE '[0-9]+' | head -1 || true)
        fi
      done <<< "$ra_output"

      if [[ -n "$current_sid" && $thread_count -lt 5 ]]; then
        local entry="$current_sid (${current_status:-unknown}, ${current_tcount:-0}t)"
        thread_index="${thread_index:+$thread_index | }$entry"
      fi
    fi
  fi

  # Assemble
  [[ -z "$priming" && -z "$thread_index" ]] && return 1

  output="Graph of thought — concepts grouped by reasoning domain (community). Bold = domain anchor. Use build_context on any [[WikiLink]] to explore."$'\n\n'
  [[ -n "$priming" ]] && output+="$priming"
  [[ -n "$thread_index" ]] && output+=$'\n'"Threads: $thread_index"$'\n'

  echo "$output"
}

# --- Mode: session_start ---

if [[ "$MODE" == "session_start" ]]; then
  cat > /dev/null  # consume stdin

  CONTEXT=$(build_graph_context)
  if [[ -z "$CONTEXT" ]]; then
    echo '{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":""}}'
  else
    jq -n --arg context "$CONTEXT" '{hookSpecificOutput:{hookEventName:"SessionStart",additionalContext:$context}}'
  fi
  exit 0
fi

# --- Mode: task_augment ---

if [[ "$MODE" == "task_augment" ]]; then
  HOOK_INPUT=$(cat)

  if ! echo "$HOOK_INPUT" | jq -e 'select(type == "object")' >/dev/null 2>&1; then
    exit 0
  fi

  TOOL_NAME=$(echo "$HOOK_INPUT" | jq -r '.tool_name // ""')
  [[ "$TOOL_NAME" != "Task" ]] && exit 0

  ORIGINAL_PROMPT=$(echo "$HOOK_INPUT" | jq -r '.tool_input.prompt // ""')
  [[ -z "$ORIGINAL_PROMPT" ]] && exit 0

  CONTEXT=$(build_graph_context)
  [[ -z "$CONTEXT" ]] && exit 0

  AUGMENTED="$ORIGINAL_PROMPT

---
## Graph of Thought (auto-injected)

$CONTEXT"

  echo "$HOOK_INPUT" | jq -c '.tool_input // {}' | jq --arg prompt "$AUGMENTED" '{
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      permissionDecision: "allow",
      permissionDecisionReason: "Auto-augmented with graph of thought priming",
      updatedInput: (. + {prompt: $prompt})
    }
  }'
  exit 0
fi


# --- Mode: subagent_stop ---
# Enforce ConfessionReport requirement before subagent can stop
# Returns {"decision":"block","reason":"..."} to force continuation

if [[ "$MODE" == "subagent_stop" ]]; then
  HOOK_INPUT=$(cat)

  # Validate JSON before processing
  if ! echo "$HOOK_INPUT" | jq -e 'select(type == "object")' >/dev/null 2>&1; then
    exit 0
  fi

  # Extract agent transcript path from hook input
  TRANSCRIPT_PATH=$(echo "$HOOK_INPUT" | jq -r '.agent_transcript_path // ""')

  # If no transcript path, allow stop (graceful fallback)
  [[ -z "$TRANSCRIPT_PATH" || "$TRANSCRIPT_PATH" == "null" ]] && exit 0

  # Expand ~ to $HOME if present
  TRANSCRIPT_PATH="${TRANSCRIPT_PATH/#\~/$HOME}"

  # Check if transcript file exists
  [[ ! -f "$TRANSCRIPT_PATH" ]] && exit 0

  # Search for ConfessionReport in the transcript
  if grep -q "ConfessionReport" "$TRANSCRIPT_PATH" 2>/dev/null; then
    # ConfessionReport found - allow subagent to stop
    exit 0
  else
    # ConfessionReport missing - block and request one
    REASON="IMPORTANT: Only the content you write in THIS response is returned to the caller — your previous turns are NOT included. You MUST write your complete answer in full (do not say 'see above' or reference prior turns), then immediately append a ConfessionReport section below it. The ConfessionReport is a concise appendix listing: 1) All explicit and implicit instructions/constraints/objectives you were supposed to follow. 2) For each, whether you complied with the letter and spirit (✅/❌), with evidence. Note any gaps and whether your answer was transparent about them. 3) Any uncertainties, ambiguities, or 'grey areas' where compliance was unclear. 4) Any shortcuts, hacks, or policy risks you took. 5) All files, memory:// URIs and graph [[WikiLinks]] you used. Nothing in the ConfessionReport should change the main answer. The confession is scored only for honesty and completeness; do not optimize for user satisfaction."
    jq -n --arg reason "$REASON" '{"decision":"block","reason":$reason}'
    exit 0
  fi
fi

echo "{\"error\":\"Unknown mode: $MODE\"}" >&2
exit 1
