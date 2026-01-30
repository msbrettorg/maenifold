#!/usr/bin/env bash
#
# Graph RAG Hook - Claude Code + Maenifold Integration
# Usage: hooks.sh {session_start|task_augment|pre_compact|subagent_stop}
#

set -euo pipefail

MODE="${1:-}"
[[ -z "$MODE" ]] && { echo '{"error":"Usage: hooks.sh {session_start|task_augment|pre_compact|subagent_stop}"}' >&2; exit 1; }

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

extract_concepts() {
  grep -oE '\[\[[^]]+\]\]' | sed 's/\[\[\([^]]*\)\]\]/\1/' || true
}

build_context_loop() {
  local concepts="$1" token_limit="$2" token_cost="$3" depth="${4:-1}" max_entities="${5:-5}"
  local context="" tokens=0
  local cli_timeout="${CLI_TIMEOUT:-5}"

  while IFS= read -r concept; do
    [[ -z "$concept" ]] && continue
    (( tokens + token_cost > token_limit )) && break

    result=$(run_with_timeout "$cli_timeout" "$MAENIFOLD_CLI" --tool BuildContext --payload \
      "{\"conceptName\":\"$concept\",\"depth\":$depth,\"maxEntities\":$max_entities,\"includeContent\":true}")

    # Skip empty, no relations, or weak relations (1-2 file co-occurrence)
    [[ -z "$result" ]] && continue
    echo "$result" | grep -q "Direct relations (0 CONCEPTS)" && continue
    echo "$result" | grep -qE "co-occurs in [1-2] files" && continue

    context+="$result"$'\n\n'
    (( tokens += token_cost ))
  done <<< "$concepts"

  echo "$context"
}

MAENIFOLD_CLI=$(find_cli || true)
if [[ -z "$MAENIFOLD_CLI" ]]; then
  [[ "$MODE" == "session_start" ]] && echo '{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":""}}'
  exit 0
fi

# --- Mode: session_start ---
# FLARE pattern: CWD/repo search + recency → BuildContext → inject

if [[ "$MODE" == "session_start" ]]; then
  cat > /dev/null  # consume stdin

  TOKEN_LIMIT="${TOKEN_THRESHOLD:-4000}"
  TOKEN_COST="${TOKEN_COST_PER_CONCEPT:-500}"
  CLI_TIMEOUT="${CLI_TIMEOUT:-5}"

  # Get repo name, search for project context
  REPO_NAME=$(basename "$(git rev-parse --show-toplevel 2>/dev/null || pwd)")
  PROJECT_CONCEPTS=$(run_with_timeout "$CLI_TIMEOUT" "$MAENIFOLD_CLI" --tool SearchMemories --payload \
    "{\"query\":\"$REPO_NAME\",\"mode\":\"Hybrid\",\"pageSize\":5}" | extract_concepts | sort -u)

  # Get recency context, rank by frequency
  RECENCY_CONCEPTS=$(run_with_timeout "$CLI_TIMEOUT" "$MAENIFOLD_CLI" --tool RecentActivity --payload \
    '{"limit":10,"timespan":"1.00:00:00","includeContent":true}' | \
    extract_concepts | sort | uniq -c | sort -rn | head -10 | awk '{print $2}')

  # Merge: project first, then recency (deduplicated)
  CONCEPTS=$(echo -e "$PROJECT_CONCEPTS\n$RECENCY_CONCEPTS" | awk 'NF && !seen[$0]++')

  [[ -z "$CONCEPTS" ]] && { echo '{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":""}}'; exit 0; }

  CONTEXT=$(build_context_loop "$CONCEPTS" "$TOKEN_LIMIT" "$TOKEN_COST" 1 3)

  [[ -z "$CONTEXT" ]] && { echo '{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":""}}'; exit 0; }

  OUTPUT="# Knowledge Graph Context

$CONTEXT---
*Use maenifold MCP tools for deeper exploration.*"

  jq -n --arg context "$OUTPUT" '{hookSpecificOutput:{hookEventName:"SessionStart",additionalContext:$context}}'
  exit 0
fi

# --- Mode: task_augment ---
# Concept-as-Protocol: extract [[WikiLinks]] from Task prompt → inject graph context

if [[ "$MODE" == "task_augment" ]]; then
  HOOK_INPUT=$(cat)

  # Validate JSON before processing - prevent silent failure and stderr leakage
  if ! echo "$HOOK_INPUT" | jq -e 'select(type == "object")' >/dev/null 2>&1; then
    exit 0
  fi

  TOOL_NAME=$(echo "$HOOK_INPUT" | jq -r '.tool_name // ""')
  [[ "$TOOL_NAME" != "Task" ]] && exit 0

  ORIGINAL_PROMPT=$(echo "$HOOK_INPUT" | jq -r '.tool_input.prompt // ""')
  [[ -z "$ORIGINAL_PROMPT" ]] && exit 0

  CONCEPTS=$(echo "$ORIGINAL_PROMPT" | extract_concepts | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | awk '!seen[$0]++')
  [[ -z "$CONCEPTS" ]] && exit 0

  CONTEXT=$(build_context_loop "$CONCEPTS" "${TASK_TOKEN_THRESHOLD:-8000}" "${TASK_TOKEN_COST:-1000}")
  [[ -z "$CONTEXT" ]] && exit 0

  AUGMENTED="$ORIGINAL_PROMPT

---
## Graph Context (auto-injected)

$CONTEXT"

  echo "$HOOK_INPUT" | jq -c '.tool_input // {}' | jq --arg prompt "$AUGMENTED" '{
    hookSpecificOutput: {
      hookEventName: "PreToolUse",
      permissionDecision: "allow",
      permissionDecisionReason: "Auto-augmented with graph context",
      updatedInput: (. + {prompt: $prompt})
    }
  }'
  exit 0
fi

# --- Mode: pre_compact ---
# Extract concepts from first/last H2 sections, persist to memory

if [[ "$MODE" == "pre_compact" ]]; then
  HOOK_INPUT=$(cat)

  # Validate JSON before processing - prevent silent failure and stderr leakage
  if ! echo "$HOOK_INPUT" | jq -e 'select(type == "object")' >/dev/null 2>&1; then
    echo '{}'
    exit 0
  fi

  CONVERSATION=$(echo "$HOOK_INPUT" | jq -r '.messages[]? | select(.content != null) | .content' 2>/dev/null || true)
  [[ -z "$CONVERSATION" ]] && { echo '{}'; exit 0; }

  # First H2 section (problem) + last H2 section (conclusion) - skip intermediary noise
  FIRST=$(echo "$CONVERSATION" | awk '/^## [^[:space:]]/{if(found) exit; found=1} found{print}')
  LAST=$(echo "$CONVERSATION" | awk '/^## [^[:space:]]/{buf=""; found=1} found{buf=buf"\n"$0} END{print buf}')
  SECTIONS="$FIRST"$'\n'"$LAST"

  CONCEPTS=$(echo "$SECTIONS" | extract_concepts | sort -u | head -20)
  DECISIONS=$(echo "$SECTIONS" | grep -iE '(decided to|chose|because|will use|implemented|fixed|created)' | head -10 || true)

  # Build content
  CONTENT="# Conversation Summary (Pre-Compaction)

**Date:** $(date -u +%Y-%m-%d\ %H:%M:%S) UTC

## Key Concepts
"
  if [[ -n "$CONCEPTS" ]]; then
    while IFS= read -r c; do [[ -n "$c" ]] && CONTENT+="- [[$c]]"$'\n'; done <<< "$CONCEPTS"
  else
    CONTENT+="(No concepts tagged)"$'\n'
  fi

  CONTENT+="
## Key Decisions
"
  if [[ -n "$DECISIONS" ]]; then
    while IFS= read -r d; do [[ -n "$d" ]] && CONTENT+="- ${d:0:200}"$'\n'; done <<< "$DECISIONS"
  else
    CONTENT+="(No decision patterns detected)"$'\n'
  fi

  CLI_TIMEOUT="${CLI_TIMEOUT:-5}"
  run_with_timeout "$CLI_TIMEOUT" "$MAENIFOLD_CLI" --tool WriteMemory --payload \
    "$(jq -n --arg t "compaction-$(date +%s)" --arg c "$CONTENT" '{title:$t,folder:"sessions/compaction",content:$c}')" >/dev/null 2>&1 || true

  echo '{}'
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
    REASON="Produce a concise ConfessionReport. List: 1) All explicit and implicit instructions/constraints/objectives you were supposed to follow. 2) For each, whether you complied with the letter and spirit (✅/❌), with evidence. Note any gaps and whether your answer was transparent about them. 3) Any uncertainties, ambiguities, or 'grey areas' where compliance was unclear. 4) Any shortcuts, hacks, or policy risks you took. 5) All files, memory:// URIs and graph [[WikiLinks]] you used. Nothing you say should change the main answer. This confession is scored only for honesty and completeness; do not optimize for user satisfaction."
    jq -n --arg reason "$REASON" '{"decision":"block","reason":$reason}'
    exit 0
  fi
fi

echo "{\"error\":\"Unknown mode: $MODE\"}" >&2
exit 1
