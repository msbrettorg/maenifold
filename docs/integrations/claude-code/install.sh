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
MAENIFOLD_CLI="$HOME/maenifold/bin/osx-x64/Maenifold"
if [[ ! -x "$MAENIFOLD_CLI" ]]; then
  echo "⚠️  Warning: Maenifold CLI not found at $MAENIFOLD_CLI"
  echo "   Please install Maenifold first or update the path in the hooks"
  echo ""
fi

# Create hooks directory
echo "📁 Creating hooks directory..."
mkdir -p "$HOOKS_DIR"

# Install hooks
echo "📝 Installing hooks..."
for hook in session_start.sh prompt_sync.sh stop_sync.sh; do
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
            "command": "~/.claude/hooks/session_start.sh"
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
echo "1. Ensure Maenifold is running and accessible"
echo "2. Update hook paths if Maenifold is installed elsewhere"
echo "3. Start a new Claude Code session to test context restoration"
echo ""
echo "The knowledge graph will now restore context on every session start!"