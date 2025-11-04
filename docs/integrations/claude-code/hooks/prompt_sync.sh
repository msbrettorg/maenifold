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

# Maenifold CLI path - adjust to your installation
MAENIFOLD_CLI="$HOME/maenifold/bin/osx-x64/Maenifold"

# Verify Maenifold CLI exists
if [[ ! -x "$MAENIFOLD_CLI" ]]; then
  # Allow prompt if Maenifold not available
  exit 0
fi

# Read hook input
HOOK_INPUT=$(cat)
PROMPT=$(echo "$HOOK_INPUT" | jq -r '.prompt // ""')

# Check if prompt mentions searching for knowledge
if echo "$PROMPT" | grep -qi "search\|find\|where\|what.*know\|remember"; then
  # Sync the knowledge graph to ensure it's current
  "$MAENIFOLD_CLI" --tool Sync 2>/dev/null || true

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