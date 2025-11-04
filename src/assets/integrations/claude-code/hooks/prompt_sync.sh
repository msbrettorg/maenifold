#!/usr/bin/env bash
#
# User Prompt Submit Hook for Claude Code + Maenifold Integration
#
# Purpose: Sync Maenifold graph before processing user prompts
# to ensure latest knowledge is available.
#
# Hook Event: UserPromptSubmit
#
# Input (stdin): JSON with prompt details
# Output: Exit 0 to allow prompt, non-zero to block
#

set -euo pipefail

# Verify Maenifold CLI is available
if ! command -v maenifold &> /dev/null; then
  # Allow prompt if Maenifold not available
  exit 0
fi

# Read hook input
HOOK_INPUT=$(cat)
PROMPT=$(echo "$HOOK_INPUT" | jq -r '.prompt // ""')

# Check if prompt mentions searching for knowledge
if echo "$PROMPT" | grep -qi "search\|find\|where\|what.*know\|remember"; then
  # Sync the knowledge graph to ensure it's current
  maenifold --tool Sync 2>/dev/null || true

  # Optional: Output a message to the user
  echo "📊 Knowledge graph synchronized for search"
fi

# Check if prompt contains concepts to search
CONCEPTS=$(echo "$PROMPT" | grep -o '\[\[[^]]*\]\]' | head -3)
if [[ -n "$CONCEPTS" ]]; then
  # Could perform a quick search and inject results
  # For now, just note that concepts were found
  echo "🔍 Detected concepts in prompt: $CONCEPTS"
fi

# Always allow the prompt to continue
exit 0