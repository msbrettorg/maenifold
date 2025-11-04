#!/usr/bin/env bash
#
# Stop Hook for Claude Code + Maenifold Integration
#
# Purpose: After Claude completes a response, optionally save
# important discoveries and sync the knowledge graph.
#
# Hook Event: Stop
#
# Input (stdin): JSON with session details
# Output: Exit 0 (success)
#

set -euo pipefail

# Verify Maenifold CLI is available
if ! command -v maenifold &> /dev/null; then
  exit 0
fi

# Read hook input
HOOK_INPUT=$(cat)
SESSION_ID=$(echo "$HOOK_INPUT" | jq -r '.session_id // "unknown"')

# Check if any memory operations occurred during this session
# This is a simple check - you could make it more sophisticated
MEMORY_STATUS=$(maenifold --tool MemoryStatus 2>/dev/null || echo "{}")

# If new memories were created, sync the graph
if echo "$MEMORY_STATUS" | grep -q "FILES"; then
  maenifold --tool Sync 2>/dev/null || true
  echo "✅ Knowledge graph synchronized after session"
fi

exit 0