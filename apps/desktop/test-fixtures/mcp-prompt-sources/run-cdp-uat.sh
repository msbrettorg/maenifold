#!/usr/bin/env bash
# CDP UAT verification for T-303.18.6: per-source prompts UX
# Prerequisites: packaged binary exists, prompt_server.py accessible, agent-browser installed
# Steps:
# 1. Export MAENIFOLD_ROOT to point at this fixture directory
# 2. Launch packaged app via uat-launch.sh with MAENIFOLD_ROOT override
# 3. agent-browser connect <port>
# 4. Navigate to Chat tab
# 5. agent-browser snapshot -i | grep "mcp.fixtureA.summarize" → MUST be present
# 6. agent-browser snapshot -i | grep "mcp.fixtureB.summarize" → MUST be present
# 7. Click fixtureA command, submit → chat output must contain "server-id:fixtureA"
# 8. Click fixtureB command, submit → chat output must contain "server-id:fixtureB"
# 9. Capture 3 screenshots: palette, fixtureA-resolved, fixtureB-resolved

set -euo pipefail

fixture_dir="/Users/brett/src/ma-collective/maenifold-rs/worktrees/T-303.18.6/apps/desktop/test-fixtures/mcp-prompt-sources"
repo_root="/Users/brett/src/ma-collective/maenifold-rs/worktrees/T-303.18.6"

cat <<EOF
T-303.18.6 CDP UAT fixture

1. Export fixture root:
   export MAENIFOLD_ROOT="$fixture_dir"

2. Launch the packaged app with the fixture config:
   MAENIFOLD_ROOT="$fixture_dir" "$repo_root/scripts/uat-launch.sh" T-303.18.6 <port>

3. Connect CDP:
   agent-browser connect <port>

4. In the app, open the Chat tab.

5. Verify prompt palette entry for fixtureA:
   agent-browser snapshot -i | grep "mcp.fixtureA.summarize"
   Expected: match present

6. Verify prompt palette entry for fixtureB:
   agent-browser snapshot -i | grep "mcp.fixtureB.summarize"
   Expected: match present

7. Resolve fixtureA prompt:
   - Open the prompt/command palette entry for mcp.fixtureA.summarize
   - Submit the prompt
   - Verify the chat output contains: server-id:fixtureA

8. Resolve fixtureB prompt:
   - Open the prompt/command palette entry for mcp.fixtureB.summarize
   - Submit the prompt
   - Verify the chat output contains: server-id:fixtureB

9. Capture evidence screenshots:
   - palette
   - fixtureA-resolved
   - fixtureB-resolved
EOF
