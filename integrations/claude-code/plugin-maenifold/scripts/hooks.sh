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
  local concepts="$1" token_limit="$2" token_cost="$3" depth="${4:-1}" max_entities="${5:-5}" include_content="${6:-true}"
  local context="" tokens=0
  local cli_timeout="${CLI_TIMEOUT:-5}"

  while IFS= read -r concept; do
    [[ -z "$concept" ]] && continue
    (( tokens + token_cost > token_limit )) && break

    result=$(run_with_timeout "$cli_timeout" "$MAENIFOLD_CLI" --tool BuildContext --payload \
      "{\"conceptName\":\"$concept\",\"depth\":$depth,\"maxEntities\":$max_entities,\"includeContent\":$include_content}")

    # Skip empty or zero-relation results
    [[ -z "$result" ]] && continue
    echo "$result" | grep -q "Direct relations (0 CONCEPTS)" && continue

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
# T-HOOKS-001.3: Community-clustered pointer array loader
# RTM: FR-16.1–16.7, FR-16.11, NFR-16.1.1–16.1.10

if [[ "$MODE" == "session_start" ]]; then
  cat > /dev/null  # consume stdin

  CLI_TIMEOUT="${CLI_TIMEOUT:-5}"
  EMPTY_RESPONSE='{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":""}}'

  # --- WikiLink validation (FR-16.11, NFR-16.1.5) ---
  # Must be >= 3 chars, lowercase-with-hyphens only, no dots, no slashes
  validate_wikilink() {
    local link="$1"
    [[ ${#link} -ge 3 ]] && [[ "$link" =~ ^[a-z0-9]([a-z0-9-]*[a-z0-9])?$ ]] && return 0
    return 1
  }

  # --- Skip list (bash 3.2 compatible, newline-delimited string) ---
  SKIP_LIST=""
  skip_list_add() { SKIP_LIST="$SKIP_LIST"$'\n'"$1"; }
  skip_list_has() { echo "$SKIP_LIST" | grep -qxF "$1" 2>/dev/null; }

  # --- Community cluster tracking (parallel indexed arrays, bash 3.2) ---
  # CLUSTER_CID[i], CLUSTER_LABEL[i], CLUSTER_MEMBERS[i]
  CLUSTER_COUNT=0

  # --- Step 1: Discover recent thinking sessions (FR-16.1) ---
  # RecentActivity with filter=thinking, limit=10, 3-day window, includeContent=false
  RA_OUTPUT=$(run_with_timeout "$CLI_TIMEOUT" "$MAENIFOLD_CLI" --tool RecentActivity --payload \
    '{"filter":"thinking","limit":10,"timespan":"3.00:00:00","includeContent":false}')

  # --- Step 2: Extract thread index and seeds from RecentActivity (FR-16.2, FR-16.6) ---
  # RecentActivity output is multi-line blocks:
  #   **session-NNNNN-NNNNN** (sequential)
  #     Thoughts: N
  #     Status: completed
  #     First: "...[[WikiLinks]]..."
  THREAD_INDEX=""
  THREAD_COUNT=0
  SEED_COUNT=0

  if [[ -n "$RA_OUTPUT" ]]; then
    current_sid=""
    current_status=""
    current_tcount=""
    current_seed_found=0

    while IFS= read -r line; do
      # Detect session header line: **session-NNNNN-NNNNN** or **workflow-NNNNN**
      header_sid=$(echo "$line" | grep -oE 'session-[0-9]+-[0-9]+' || true)
      if [[ -z "$header_sid" ]]; then
        header_sid=$(echo "$line" | grep -oE 'session-[0-9]+' || true)
        if [[ -z "$header_sid" ]]; then
          header_sid=$(echo "$line" | grep -oE 'workflow-[0-9]+' || true)
        fi
      fi

      # Session header starts with ** (bold markdown)
      if [[ -n "$header_sid" ]] && echo "$line" | grep -qF '**' 2>/dev/null; then
        # Flush previous session to thread index
        if [[ -n "$current_sid" && $THREAD_COUNT -lt 5 ]]; then
          [[ -z "$current_status" ]] && current_status="unknown"
          [[ -z "$current_tcount" ]] && current_tcount="0"
          entry="$current_sid ($current_status, ${current_tcount}t)"
          if [[ -z "$THREAD_INDEX" ]]; then
            THREAD_INDEX="$entry"
          else
            THREAD_INDEX="$THREAD_INDEX | $entry"
          fi
          THREAD_COUNT=$((THREAD_COUNT + 1))
        fi

        # Start tracking new session
        current_sid="$header_sid"
        current_status=""
        current_tcount=""
        current_seed_found=0
        continue
      fi

      # Parse Status line
      if echo "$line" | grep -qE '^ *Status:' 2>/dev/null; then
        current_status=$(echo "$line" | grep -oE '(active|completed|abandoned)' || true)
        continue
      fi

      # Parse Thoughts/Steps count
      if echo "$line" | grep -qE '^ *(Thoughts|Steps):' 2>/dev/null; then
        current_tcount=$(echo "$line" | grep -oE '[0-9]+' | head -1 || true)
        continue
      fi

      # Parse First line for seed concept (FR-16.2)
      if [[ $current_seed_found -eq 0 ]] && echo "$line" | grep -qE '^ *First:' 2>/dev/null; then
        seed=$(echo "$line" | extract_concepts | head -1)
        if [[ -n "$seed" ]]; then
          seed=$(echo "$seed" | tr '[:upper:]' '[:lower:]' | tr ' _' '-' | sed 's/--*/-/g; s/^-//; s/-$//')
          if validate_wikilink "$seed"; then
            # Deduplicate seeds
            seed_dup=0
            _si=0
            while [[ $_si -lt $SEED_COUNT ]]; do
              [[ "${SESSION_SEEDS[$_si]}" == "$seed" ]] && { seed_dup=1; break; }
              _si=$((_si + 1))
            done
            if [[ $seed_dup -eq 0 ]]; then
              SESSION_SEEDS[$SEED_COUNT]="$seed"
              SEED_COUNT=$((SEED_COUNT + 1))
            fi
          fi
        fi
        current_seed_found=1
        continue
      fi
    done <<< "$RA_OUTPUT"

    # Flush the last session (NFR-16.1.8: cap at 5)
    if [[ -n "$current_sid" && $THREAD_COUNT -lt 5 ]]; then
      [[ -z "$current_status" ]] && current_status="unknown"
      [[ -z "$current_tcount" ]] && current_tcount="0"
      entry="$current_sid ($current_status, ${current_tcount}t)"
      if [[ -z "$THREAD_INDEX" ]]; then
        THREAD_INDEX="$entry"
      else
        THREAD_INDEX="$THREAD_INDEX | $entry"
      fi
      THREAD_COUNT=$((THREAD_COUNT + 1))
    fi
  fi

  # --- Step 3: Fallback if no thinking sessions (NFR-16.1.6) ---
  if [[ $SEED_COUNT -eq 0 ]]; then
    REPO_NAME=$(basename "$(git rev-parse --show-toplevel 2>/dev/null || pwd)")
    FALLBACK_PAYLOAD=$(jq -n --arg q "$REPO_NAME" '{"query":$q,"mode":"Hybrid","pageSize":5}')
    FALLBACK_OUTPUT=$(run_with_timeout "$CLI_TIMEOUT" "$MAENIFOLD_CLI" --tool SearchMemories --payload "$FALLBACK_PAYLOAD")

    if [[ -n "$FALLBACK_OUTPUT" ]]; then
      while IFS= read -r seed; do
        [[ -z "$seed" ]] && continue
        seed=$(echo "$seed" | tr '[:upper:]' '[:lower:]' | tr ' _' '-' | sed 's/--*/-/g; s/^-//; s/-$//')
        if validate_wikilink "$seed"; then
          SESSION_SEEDS[$SEED_COUNT]="$seed"
          SEED_COUNT=$((SEED_COUNT + 1))
        fi
        [[ $SEED_COUNT -ge 3 ]] && break
      done <<< "$(echo "$FALLBACK_OUTPUT" | extract_concepts)"
    fi
  fi

  # If still no seeds, emit empty response
  if [[ $SEED_COUNT -eq 0 ]]; then
    echo "$EMPTY_RESPONSE"
    exit 0
  fi

  # --- Step 4: BuildContext expansion with skip list (FR-16.3, FR-16.4) ---
  # BuildContext CLI output format (no [[WikiLinks]]):
  #   Context for CONCEPT: concept-name
  #   Direct relations (N CONCEPTS):
  #     • related-concept (co-occurs in M files) [community N]
  #   Community siblings (N concepts in community M):
  #     • sibling-concept (shared neighbors: N, overlap: 0.XX)
  # Extract concept names from bullet lines: "• concept-name (...)"

  # Helper: extract concept names from bullet-point lines
  extract_bullet_concepts() {
    grep -oE '• [a-zA-Z0-9][a-zA-Z0-9._-]*' | sed 's/^• //' || true
  }

  BC_CALLS=0
  MAX_BC_CALLS=6

  _si=0
  while [[ $_si -lt $SEED_COUNT ]]; do
    seed="${SESSION_SEEDS[$_si]}"
    _si=$((_si + 1))

    [[ $BC_CALLS -ge $MAX_BC_CALLS ]] && break

    # Skip if already seen (FR-16.4)
    skip_list_has "$seed" && continue

    # Call BuildContext (FR-16.3): depth=1, maxEntities=3, includeContent=false (NFR-16.1.7)
    BC_OUTPUT=$(run_with_timeout "$CLI_TIMEOUT" "$MAENIFOLD_CLI" --tool BuildContext --payload \
      "{\"conceptName\":\"$seed\",\"depth\":1,\"maxEntities\":3,\"includeContent\":false}")
    BC_CALLS=$((BC_CALLS + 1))

    # Skip failed/empty calls without retry (NFR-16.1.9)
    [[ -z "$BC_OUTPUT" ]] && continue

    # Parse seed community from "Community siblings (N concepts in community M):" header
    seed_community=$(echo "$BC_OUTPUT" | grep -oE 'Community siblings \([0-9]+ concepts in community [0-9]+\)' | grep -oE 'community [0-9]+' | grep -oE '[0-9]+' || true)
    # Fallback: get from first direct relation's [community N] tag
    if [[ -z "$seed_community" ]]; then
      seed_community=$(echo "$BC_OUTPUT" | grep -oE '\[community [0-9]+\]' | head -1 | grep -oE '[0-9]+' || true)
    fi
    [[ -z "$seed_community" ]] && seed_community="uncategorized"

    # Add seed to skip list
    skip_list_add "$seed"

    # Find or create cluster for this community (inline, no subshell)
    cidx=-1
    _ci=0
    while [[ $_ci -lt $CLUSTER_COUNT ]]; do
      if [[ "${CLUSTER_CID[$_ci]}" == "$seed_community" ]]; then
        cidx=$_ci
        break
      fi
      _ci=$((_ci + 1))
    done
    if [[ $cidx -eq -1 ]]; then
      cidx=$CLUSTER_COUNT
      CLUSTER_CID[$CLUSTER_COUNT]="$seed_community"
      CLUSTER_LABEL[$CLUSTER_COUNT]="$seed"
      CLUSTER_MEMBERS[$CLUSTER_COUNT]=""
      CLUSTER_COUNT=$((CLUSTER_COUNT + 1))
    fi

    # Extract direct relations section
    direct_section=$(echo "$BC_OUTPUT" | sed -n '/^Direct relations/,/^$/p; /^Direct relations/,/^Community/p' || true)
    direct_concepts=""
    if [[ -n "$direct_section" ]]; then
      direct_concepts=$(echo "$direct_section" | extract_bullet_concepts)
    fi

    # Extract community siblings section
    sibling_section=$(echo "$BC_OUTPUT" | sed -n '/^Community siblings/,/^$/p' || true)
    sibling_concepts=""
    if [[ -n "$sibling_section" ]]; then
      sibling_concepts=$(echo "$sibling_section" | extract_bullet_concepts)
    fi

    # Add members to cluster (excluding seed and cluster label, deduplicated, validated)
    cluster_label="${CLUSTER_LABEL[$cidx]}"
    for concept in $direct_concepts $sibling_concepts; do
      [[ -z "$concept" ]] && continue
      concept=$(echo "$concept" | tr '[:upper:]' '[:lower:]' | tr ' _' '-' | sed 's/--*/-/g; s/^-//; s/-$//')
      validate_wikilink "$concept" || continue
      [[ "$concept" == "$seed" ]] && continue
      [[ "$concept" == "$cluster_label" ]] && continue
      existing="${CLUSTER_MEMBERS[$cidx]}"
      if ! echo " $existing " | grep -qF " $concept " 2>/dev/null; then
        if [[ -z "$existing" ]]; then
          CLUSTER_MEMBERS[$cidx]="$concept"
        else
          CLUSTER_MEMBERS[$cidx]="$existing $concept"
        fi
      fi
    done

    # Add direct relations to skip list
    for concept in $direct_concepts; do
      concept=$(echo "$concept" | tr '[:upper:]' '[:lower:]' | tr ' _' '-' | sed 's/--*/-/g; s/^-//; s/-$//')
      validate_wikilink "$concept" && skip_list_add "$concept"
    done

    # Add same-community siblings to skip list (all siblings are same community)
    for concept in $sibling_concepts; do
      concept=$(echo "$concept" | tr '[:upper:]' '[:lower:]' | tr ' _' '-' | sed 's/--*/-/g; s/^-//; s/-$//')
      validate_wikilink "$concept" && skip_list_add "$concept"
    done

    # Add direct relations tagged with any community to skip list
    while IFS= read -r dr_line; do
      [[ -z "$dr_line" ]] && continue
      dr_concept=$(echo "$dr_line" | extract_bullet_concepts | head -1)
      [[ -z "$dr_concept" ]] && continue
      dr_concept=$(echo "$dr_concept" | tr '[:upper:]' '[:lower:]' | tr ' _' '-' | sed 's/--*/-/g; s/^-//; s/-$//')
      validate_wikilink "$dr_concept" || continue
      dr_community=$(echo "$dr_line" | grep -oE '\[community [0-9]+\]' | grep -oE '[0-9]+' || true)
      if [[ -n "$dr_community" ]]; then
        skip_list_add "$dr_concept"
      fi
    done <<< "$direct_section"
  done

  # --- Step 5: Format output as community-clustered pointer array (FR-16.5) ---
  if [[ $CLUSTER_COUNT -eq 0 ]]; then
    echo "$EMPTY_RESPONSE"
    exit 0
  fi

  OUTPUT="# Active Work"$'\n'

  ci=0
  while [[ $ci -lt $CLUSTER_COUNT ]]; do
    label="${CLUSTER_LABEL[$ci]}"
    members="${CLUSTER_MEMBERS[$ci]}"

    OUTPUT+=$'\n'"**[[$label]]**"$'\n'

    # Format members as WikiLinks, deduplicated, space-separated
    if [[ -n "$members" ]]; then
      member_line=""
      seen_members=""
      for m in $members; do
        if echo " $seen_members " | grep -qF " $m " 2>/dev/null; then
          continue
        fi
        seen_members="$seen_members $m"
        if [[ -z "$member_line" ]]; then
          member_line="[[$m]]"
        else
          member_line="$member_line [[$m]]"
        fi
      done
      OUTPUT+="$member_line"$'\n'
    fi
    ci=$((ci + 1))
  done

  # --- Step 6: Token budget enforcement (NFR-16.1.2) ---
  # Estimate tokens: ~1 token per 4 chars. Target 150-350.
  output_chars=${#OUTPUT}
  est_tokens=$(( output_chars / 4 ))

  # Add thread index if within budget; prune if needed
  if [[ -n "$THREAD_INDEX" && $est_tokens -lt 300 ]]; then
    OUTPUT+=$'\n'"Threads: $THREAD_INDEX"$'\n'
  elif [[ -n "$THREAD_INDEX" && $est_tokens -lt 330 ]]; then
    # Prune to 3 sessions
    PRUNED_INDEX=$(echo "$THREAD_INDEX" | tr '|' '\n' | head -3 | tr '\n' '|' | sed 's/|$//' | sed 's/|/ | /g')
    [[ -n "$PRUNED_INDEX" ]] && OUTPUT+=$'\n'"Threads: $PRUNED_INDEX"$'\n'
  fi
  # If over ~330 tokens, omit thread index entirely

  # Action footer (FR-16.7)
  OUTPUT+=$'\n'"Continue: sequential_thinking with session ID. Explore: build_context on any [[WikiLink]]."$'\n'

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

  CONTEXT=$(build_context_loop "$CONCEPTS" "${TASK_TOKEN_THRESHOLD:-8000}" "${TASK_TOKEN_COST:-1000}" 1 5 false)
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
