#!/usr/bin/env bash
#
# Session Start Hook v2 for Claude Code + Maenifold Integration
#
# Purpose: Restore semantic context from knowledge graph by analyzing hub concepts,
# discovering related concepts, and providing CWD-based relevance hints.
#
# Hook Event: SessionStart
#
# Input (stdin): JSON with session_id, cwd, permission_mode
# Output (stdout): JSON with hookEventName and additionalContext (markdown)
# Exit: 0 on success
#
# NOTE: This hook provides lightweight context hints. The full maenifold MCP server
# is available for deep graph operations during the session.
#

set -euo pipefail

# ============================================================================
# CLI Detection
# ============================================================================

find_maenifold_cli() {
  # Try PATH first
  if command -v maenifold >/dev/null 2>&1; then
    echo "maenifold"
    return 0
  fi

  # Fallback to hardcoded path
  local fallback="$HOME/src/ma-collective/maenifold/src/bin/Release/net9.0/maenifold"
  if [[ -x "$fallback" ]]; then
    echo "$fallback"
    return 0
  fi

  return 1
}

MAENIFOLD_CLI=$(find_maenifold_cli || true)
if [[ -z "$MAENIFOLD_CLI" ]]; then
  # Silent exit if Maenifold not available
  echo '{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":""}}'
  exit 0
fi

# ============================================================================
# Input Parsing
# ============================================================================

HOOK_INPUT=$(cat)
SESSION_ID=$(echo "$HOOK_INPUT" | jq -r '.session_id // "unknown"')
CWD=$(echo "$HOOK_INPUT" | jq -r '.cwd // ""')

# ============================================================================
# Helper Functions
# ============================================================================

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
# Data Collection
# ============================================================================

# Single RecentActivity call
RECENT_ACTIVITY=$("$MAENIFOLD_CLI" --tool RecentActivity --payload '{"limit":10,"timespan":"24.00:00:00","includeContent":false}' 2>/dev/null || echo "")

# Extract top concepts from recent activity (frequency-ranked)
TOP_CONCEPTS=$(echo "$RECENT_ACTIVITY" | grep -oE '\[\[[^]]+\]\]' | sed 's/\[\[\(.*\)\]\]/\1/' | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | sort | uniq -c | sort -rn | head -5 | awk '{print $2}')

# Detect active thinking sessions (look for "(sequential)" marker in recent activity)
ACTIVE_SESSIONS=$(echo "$RECENT_ACTIVITY" | grep -c '(sequential)' || echo "0")

# Extract repo name from CWD
REPO_NAME=$(extract_repo_name "$CWD")

# ============================================================================
# Build Context Sections
# ============================================================================

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
HUB_COUNT=0
while IFS= read -r concept; do
  [[ -z "$concept" ]] && continue

  HUB_COUNT=$((HUB_COUNT + 1))

  CONTEXT="$CONTEXT
### [[$concept]]
"

  # Get concept context (limited depth and entities for speed)
  context_md=$("$MAENIFOLD_CLI" --tool BuildContext --payload "$(jq -n --arg c "$concept" '{conceptName: $c, depth: 1, maxEntities: 3, includeContent: false}')" 2>/dev/null || echo "")

  if [[ -n "$context_md" ]]; then
    # Limit to first 1000 chars per concept
    CONTEXT="$CONTEXT
$(echo "$context_md" | head -c 1000)
"
  fi

  # Find similar concepts for semantic expansion
  similar_md=$("$MAENIFOLD_CLI" --tool FindSimilarConcepts --payload "$(jq -n --arg c "$concept" '{conceptName: $c, maxResults: 3}')" 2>/dev/null || echo "")

  if [[ -n "$similar_md" ]]; then
    # Extract concept names from markdown (format: "• concept-name (similarity: ...)")
    similar_concepts=$(echo "$similar_md" | grep -oE '•[[:space:]]+[^[:space:]]+' | sed 's/^•[[:space:]]*//' | head -3 | paste -sd ', ' -)
    if [[ -n "$similar_concepts" ]]; then
      CONTEXT="$CONTEXT
**Related:** $similar_concepts
"
    fi
  fi

  CONTEXT="$CONTEXT
"

  # Limit to 5 hubs for token budget
  if [[ $HUB_COUNT -ge 5 ]]; then
    break
  fi
done <<< "$TOP_CONCEPTS"

# Add repo-specific context if available
if [[ -n "$REPO_NAME" ]]; then
  CONTEXT="$CONTEXT
---

## Repository Context: $REPO_NAME
"

  repo_results=$("$MAENIFOLD_CLI" --tool SearchMemories --payload "$(jq -n --arg q "$REPO_NAME" '{query: $q, mode: "Hybrid", pageSize: 3}')" 2>/dev/null || echo "")

  if [[ -n "$repo_results" ]]; then
    # Extract memory URIs from markdown (format: "memory://...")
    repo_uris=$(echo "$repo_results" | grep -oE 'memory://[^ ]+' | head -3)
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

# ============================================================================
# JSON Output
# ============================================================================

# Escape and wrap context in JSON structure
jq -n \
  --arg context "$CONTEXT" \
  '{
    hookSpecificOutput: {
      hookEventName: "SessionStart",
      additionalContext: $context
    }
  }'

exit 0
