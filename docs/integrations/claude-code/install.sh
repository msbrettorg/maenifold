#!/usr/bin/env bash
#
# Installation script for Claude Code + Maenifold Integration
#
# This script sets up the hooks and configuration needed to integrate
# Maenifold's knowledge graph with Claude Code sessions.
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOOKS_DIR="$HOME/.claude/hooks"
SETTINGS_FILE="$HOME/.claude/settings.json"

echo "🚀 Installing Claude Code + Maenifold Integration"
echo ""

# Check if Maenifold is installed
MAENIFOLD_DETECTED=""
if command -v maenifold >/dev/null 2>&1; then
  MAENIFOLD_DETECTED="$(command -v maenifold)"
  echo "✓ Maenifold CLI found in PATH: $MAENIFOLD_DETECTED"
elif [[ -n "${MAENIFOLD_ROOT:-}" ]]; then
  if [[ -x "$MAENIFOLD_ROOT/src/bin/Release/net9.0/maenifold" ]]; then
    MAENIFOLD_DETECTED="$MAENIFOLD_ROOT/src/bin/Release/net9.0/maenifold"
    echo "✓ Maenifold CLI found via \$MAENIFOLD_ROOT: $MAENIFOLD_DETECTED"
  fi
fi

if [[ -z "$MAENIFOLD_DETECTED" ]]; then
  echo "⚠️  Warning: Maenifold CLI not found"
  echo "   Please install Maenifold or set MAENIFOLD_ROOT environment variable:"
  echo "   export MAENIFOLD_ROOT=\"\$HOME/maenifold\""
  echo ""
fi

# Create hooks directory
echo "📁 Creating hooks directory..."
mkdir -p "$HOOKS_DIR"

# Install hooks
echo "📝 Installing hooks..."
for hook in graph_rag.sh prompt_sync.sh stop_sync.sh; do
  if [[ -f "$SCRIPT_DIR/hooks/$hook" ]]; then
    cp "$SCRIPT_DIR/hooks/$hook" "$HOOKS_DIR/$hook"
    chmod +x "$HOOKS_DIR/$hook"
    echo "   ✓ Installed $hook"
  fi
done

# Check if settings.json exists
if [[ ! -f "$SETTINGS_FILE" ]]; then
  echo ""
  echo "📋 Creating settings.json..."
  cp "$SCRIPT_DIR/settings.json.example" "$SETTINGS_FILE"
  echo "   ✓ Created settings.json with hook configuration"
else
  echo ""
  echo "⚠️  settings.json already exists"
  echo "   Please manually update it with the hook configuration from:"
  echo "   $SCRIPT_DIR/settings.json.example"
  echo ""
  echo "   Required configuration:"
  cat <<EOF
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "~/.claude/hooks/graph_rag.sh session_start"
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Task",
        "hooks": [
          {
            "type": "command",
            "command": "~/.claude/hooks/graph_rag.sh task_augment"
          }
        ]
      }
    ]
  }
}
EOF
fi

echo ""
echo "✅ Installation complete!"
echo ""
echo "Next steps:"
echo "1. Ensure Maenifold is accessible (test: maenifold --tool MemoryStatus --payload '{}')"
if [[ -z "$MAENIFOLD_DETECTED" ]]; then
  echo "2. Set MAENIFOLD_ROOT environment variable (export MAENIFOLD_ROOT=\"\$HOME/maenifold\")"
fi
echo "3. Start a new Claude Code session to test context restoration"
echo "4. Try Task augmentation: Use Task tool with [[concepts]] in prompts"
echo ""
echo "The knowledge graph will now:"
echo "  • Restore context on every session start (SessionStart hook)"
echo "  • Auto-enrich Task prompts containing [[concepts]] (PreToolUse hook)"
echo ""
echo "Example Task prompt: \"Fix [[authentication]] bug in [[login]] module\""