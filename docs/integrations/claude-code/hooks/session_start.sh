#!/usr/bin/env bash
#
# Session Start Hook for Claude Code + Maenifold Integration
#
# Purpose: On ANY session start, restore semantic context from knowledge graph
# by analyzing recent activity, extracting concepts, and building context.
#
# Hook Event: SessionStart
# Environment: $CLAUDE_PROJECT_DIR - project root path
#
# Input (stdin): JSON with session_id, cwd, permission_mode
# Output: Exit 0 (success), stdout added as context to Claude
#

set -euo pipefail

# Maenifold CLI path - adjust to your installation
MAENIFOLD_CLI="$HOME/maenifold/bin/osx-x64/Maenifold"

# Verify Maenifold CLI exists
if [[ ! -x "$MAENIFOLD_CLI" ]]; then
  # Silently exit if Maenifold not available
  exit 0
fi

# Read hook input
HOOK_INPUT=$(cat)
SESSION_ID=$(echo "$HOOK_INPUT" | jq -r '.session_id // "unknown"')
CWD=$(echo "$HOOK_INPUT" | jq -r '.cwd // ""')

# Token budget (approximately)
MAX_TOKENS=5000
TOKENS_USED=0

# Get recent activity (last 24 hours, 10 items)
RECENT_JSON=$("$MAENIFOLD_CLI" --tool RecentActivity --payload '{"limit":10,"timespan":"24.00:00:00","includeContent":false}' 2>/dev/null || echo "{}")

# Extract all [[concepts]] from recent activity
CONCEPTS=$(echo "$RECENT_JSON" | grep -o '\[\[[^]]*\]\]' | sed 's/\[\[\(.*\)\]\]/\1/' | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | grep -v '^concept' | sort | uniq -c | sort -rn)

# Get top N concepts (start with top 10)
TOP_CONCEPTS=$(echo "$CONCEPTS" | head -10 | awk '{print $2}')

# Count concepts
CONCEPT_COUNT=$(echo "$TOP_CONCEPTS" | wc -l | tr -d ' ')

if [[ "$CONCEPT_COUNT" -eq 0 ]]; then
  # No concepts found, just show recent activity
  echo "📝 **Session Context**"
  echo ""
  echo "Recent activity (last 24 hours):"
  echo "$RECENT_JSON" | head -50
  exit 0
fi

# Build output
echo "🧠 **Knowledge Graph Context Restoration**"
echo ""
echo "**Session:** $SESSION_ID"
echo "**Active Concepts:** $CONCEPT_COUNT"
echo ""
echo "---"
echo ""

# Build context for top concepts (with token management)
echo "## Concept Network"
echo ""

CONTEXT_COUNT=0
while IFS= read -r CONCEPT; do
  if [[ -z "$CONCEPT" ]]; then
    continue
  fi

  # Stop if we've used too many tokens (rough estimate: 500 tokens per concept with content)
  if [[ $TOKENS_USED -gt 4000 ]]; then
    echo "*(Token limit reached, ${CONTEXT_COUNT} concepts loaded)*"
    break
  fi

  echo "### [[$CONCEPT]]"

  # Get context with limited depth and entities
  PAYLOAD=$(jq -n --arg c "$CONCEPT" '{conceptName: $c, depth: 1, maxEntities: 3, includeContent: true}')
  RESULT=$("$MAENIFOLD_CLI" --tool BuildContext --payload "$PAYLOAD" 2>/dev/null || echo "")

  if [[ -n "$RESULT" ]]; then
    # Limit output per concept to ~500 tokens (roughly 2000 chars)
    echo "$RESULT" | head -c 2000
    echo ""
    TOKENS_USED=$((TOKENS_USED + 500))
    CONTEXT_COUNT=$((CONTEXT_COUNT + 1))
  fi
  echo ""
done <<< "$TOP_CONCEPTS"

echo "---"
echo ""
echo "## Recent Work"
echo ""

# Include recent activity summary (without full content since we got context above)
ACTIVITY=$("$MAENIFOLD_CLI" --tool RecentActivity --payload '{"limit":5,"timespan":"24.00:00:00","includeContent":false}' 2>/dev/null || echo "")
echo "$ACTIVITY" | head -50

echo ""
echo "---"
echo ""
echo "*Graph-based context restoration complete. Top ${CONTEXT_COUNT} concepts loaded.*"

exit 0