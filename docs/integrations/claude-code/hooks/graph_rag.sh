#!/usr/bin/env bash
#
# Graph RAG Hook for Claude Code + Maenifold Integration
#
# Purpose: Unified hook for semantic context injection via knowledge graph.
# Consolidates SessionStart and Task augmentation into a single script.
#
# Usage:
#   graph_rag.sh session_start  - SessionStart hook (concepts from recent activity)
#   graph_rag.sh task_augment   - PreToolUse hook for Task (concepts from prompt)
#
# Input (stdin): JSON hook payload
# Output (stdout): JSON hook response
# Exit: 0 on success
#

set -euo pipefail

# ============================================================================
# Mode Detection
# ============================================================================

MODE="${1:-}"
if [[ -z "$MODE" ]]; then
  echo '{"error":"Usage: graph_rag.sh {session_start|task_augment}"}' >&2
  exit 1
fi

# ============================================================================
# CLI Detection
# ============================================================================

find_maenifold_cli() {
  # Try PATH first
  if command -v maenifold >/dev/null 2>&1; then
    echo "maenifold"
    return 0
  fi

  # Try MAENIFOLD_ROOT environment variable
  if [[ -n "${MAENIFOLD_ROOT:-}" && -x "$MAENIFOLD_ROOT/src/bin/Release/net9.0/maenifold" ]]; then
    echo "$MAENIFOLD_ROOT/src/bin/Release/net9.0/maenifold"
    return 0
  fi

  return 1
}

MAENIFOLD_CLI=$(find_maenifold_cli || true)
if [[ -z "$MAENIFOLD_CLI" ]]; then
  # Silent exit if Maenifold not available
  if [[ "$MODE" == "session_start" ]]; then
    echo '{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":""}}'
  fi
  # task_augment exits without output (pass-through)
  exit 0
fi

# ============================================================================
# Shared Helper Functions
# ============================================================================

# Extract [[concepts]] from text
extract_concepts() {
  local text="$1"
  echo "$text" | grep -oE '\[\[[^]]+\]\]' | sed 's/\[\[\(.*\)\]\]/\1/' | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | sort -u || true
}

# Enrich concepts with BuildContext + FindSimilarConcepts
enrich_concepts() {
  local concepts="$1"
  local max_concepts="${2:-5}"
  local char_limit_per_concept="${3:-1000}"
  local context=""
  local count=0

  while IFS= read -r concept; do
    [[ -z "$concept" ]] && continue
    count=$((count + 1))

    if [[ $count -gt $max_concepts ]]; then
      context="$context
*(additional concepts truncated for token budget)*
"
      break
    fi

    context="$context
### [[$concept]]
"

    # Get concept context (depth=2 for extended network per search-and-scripting.md §5.6)
    local context_md=$("$MAENIFOLD_CLI" --tool BuildContext --payload "$(jq -n --arg c "$concept" '{conceptName: $c, depth: 2, maxEntities: 5, includeContent: false}')" 2>/dev/null || echo "")

    if [[ -n "$context_md" ]]; then
      # Limit to char_limit_per_concept chars per concept
      context="$context
$(echo "$context_md" | head -c "$char_limit_per_concept")
"
    fi

    # Find similar concepts for semantic expansion
    local similar_md=$("$MAENIFOLD_CLI" --tool FindSimilarConcepts --payload "$(jq -n --arg c "$concept" '{conceptName: $c, maxResults: 3}')" 2>/dev/null || echo "")

    if [[ -n "$similar_md" ]]; then
      # Extract concept names from markdown (format: "• concept-name (similarity: ...)")
      local similar_concepts=$(echo "$similar_md" | grep -oE '•[[:space:]]+[^[:space:]]+' | sed 's/^•[[:space:]]*//' | head -3 | paste -sd ', ' -)
      if [[ -n "$similar_concepts" ]]; then
        context="$context
**Related:** $similar_concepts
"
      fi
    fi

    context="$context
"
  done <<< "$concepts"

  echo "$context"
}

extract_repo_name() {
  local path="$1"
  if [[ -z "$path" ]]; then
    echo ""
    return
  fi
  # Get basename, strip common suffixes
  basename "$path" | sed 's/\.git$//'
}

# ============================================================================
# Mode: session_start
# ============================================================================

if [[ "$MODE" == "session_start" ]]; then
  HOOK_INPUT=$(cat)
  SESSION_ID=$(echo "$HOOK_INPUT" | jq -r '.session_id // "unknown"')
  CWD=$(echo "$HOOK_INPUT" | jq -r '.cwd // ""')

  # Single RecentActivity call
  RECENT_ACTIVITY=$("$MAENIFOLD_CLI" --tool RecentActivity --payload '{"limit":10,"timespan":"24.00:00:00","includeContent":false}' 2>/dev/null || echo "")

  # Extract top concepts from recent activity (frequency-ranked)
  TOP_CONCEPTS=$(echo "$RECENT_ACTIVITY" | grep -oE '\[\[[^]]+\]\]' | sed 's/\[\[\(.*\)\]\]/\1/' | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | sort | uniq -c | sort -rn | head -5 | awk '{print $2}')

  # Detect active thinking sessions (look for "(sequential)" marker in recent activity)
  ACTIVE_SESSIONS=$(echo "$RECENT_ACTIVITY" | grep -c '(sequential)' || echo "0")

  # Extract repo name from CWD
  REPO_NAME=$(extract_repo_name "$CWD")

  # Build Context Sections
  CONTEXT="# Knowledge Graph Context

**Session:** \`$SESSION_ID\`
**Working Directory:** \`$REPO_NAME\`
"

  # Active session warning
  if [[ "$ACTIVE_SESSIONS" -gt 0 ]]; then
    CONTEXT="$CONTEXT
⚠️  **Active Thinking Sessions:** $ACTIVE_SESSIONS incomplete session(s) detected in last 24h
"
  fi

  CONTEXT="$CONTEXT
---

## Hub Concepts (by recent activity)
"

  # Build lightweight context for top concepts
  if [[ -n "$TOP_CONCEPTS" ]]; then
    CONCEPT_CONTEXT=$(enrich_concepts "$TOP_CONCEPTS" 5 1000)
    CONTEXT="$CONTEXT$CONCEPT_CONTEXT"
  fi

  # Add repo-specific context if available
  if [[ -n "$REPO_NAME" ]]; then
    CONTEXT="$CONTEXT
---

## Repository Context: $REPO_NAME
"

    repo_results=$("$MAENIFOLD_CLI" --tool SearchMemories --payload "$(jq -n --arg q "$REPO_NAME" '{query: $q, mode: "Hybrid", pageSize: 3}')" 2>/dev/null || echo "")

    if [[ -n "$repo_results" ]]; then
      # Filter to semantic score > 0.5 (per search-and-scripting.md §5.4)
      repo_uris=$(echo "$repo_results" | awk '
        /^📄 / { uri = ""; next }
        /memory:\/\// { uri = $0; gsub(/[^a-zA-Z0-9:\/\._-].*/, "", uri) }
        /Semantic:/ {
          sem = $NF + 0
          if (sem > 0.5 && uri != "") print uri
          uri = ""
        }
      ' | head -3)
      if [[ -n "$repo_uris" ]]; then
        CONTEXT="$CONTEXT
Relevant memories:
$(echo "$repo_uris" | sed 's/^/- /')
"
      fi
    fi
  fi

  # Add recent activity summary
  CONTEXT="$CONTEXT
---

## Recent Activity (24h)

$(echo "$RECENT_ACTIVITY" | head -30)

---

*Lightweight context loaded. Use maenifold MCP tools for deep graph exploration.*
"

  # JSON Output
  jq -n \
    --arg context "$CONTEXT" \
    '{
      hookSpecificOutput: {
        hookEventName: "SessionStart",
        additionalContext: $context
      }
    }'

  exit 0
fi

# ============================================================================
# Mode: task_augment
# ============================================================================

if [[ "$MODE" == "task_augment" ]]; then
  HOOK_INPUT=$(cat)
  TOOL_NAME=$(echo "$HOOK_INPUT" | jq -r '.tool_name // ""')

  # Only process Task tool
  if [[ "$TOOL_NAME" != "Task" ]]; then
    exit 0
  fi

  # Extract the prompt from tool_input
  ORIGINAL_PROMPT=$(echo "$HOOK_INPUT" | jq -r '.tool_input.prompt // ""')

  if [[ -z "$ORIGINAL_PROMPT" ]]; then
    exit 0
  fi

  # Extract and frequency-rank concepts (repeated concepts weighted higher)
  CONCEPTS=$(echo "$ORIGINAL_PROMPT" | grep -oE '\[\[[^]]+\]\]' | sed 's/\[\[\(.*\)\]\]/\1/' | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | sort | uniq -c | sort -rn | awk '{print $2}')

  if [[ -z "$CONCEPTS" ]]; then
    # No concepts found - pass through without modification
    exit 0
  fi

  # Count concepts for logging
  CONCEPT_COUNT=$(echo "$CONCEPTS" | wc -l | tr -d ' ')

  # Build Context
  CONTEXT="
---

## Graph Context (auto-injected)

*Concepts detected in prompt: $CONCEPT_COUNT*
"

  # Build context for each concept (limit to 5 for token budget, 800 chars each)
  CONCEPT_CONTEXT=$(enrich_concepts "$CONCEPTS" 5 800)
  CONTEXT="$CONTEXT$CONCEPT_CONTEXT"

  CONTEXT="$CONTEXT
---

*Use maenifold MCP tools for deeper exploration.*
"

  # Build Augmented Prompt
  AUGMENTED_PROMPT="$ORIGINAL_PROMPT
$CONTEXT"

  # JSON Output
  jq -n \
    --arg prompt "$AUGMENTED_PROMPT" \
    '{
      hookSpecificOutput: {
        hookEventName: "PreToolUse",
        updatedInput: {
          prompt: $prompt
        }
      }
    }'

  exit 0
fi

# ============================================================================
# Unknown Mode
# ============================================================================

echo "{\"error\":\"Unknown mode: $MODE\"}" >&2
exit 1
