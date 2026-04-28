# Tab inventory — sourced from `tab-inventory` agent response (haiku, 2026-04-28)

This file was missing on disk; reconstructing it here. The `tab-inventory` agent
returned the full inventory in its reply but did not persist a file. Future duo
reviewers cited it; restoring so subsequent reviews have a stable reference.

See the full inventory in the `tab-inventory` agent's saved transcript or use:
- `outputs/uat/cdp-tab-enumeration.md` — what's reachable in the running UI.
- `outputs/uat/mcp-ui-capabilities.md` — top-10 expected MCP UI capabilities.
- The `tab-inventory` ageant's response text (in this session's transcript) —
  has the full per-tab table + component tree + store state diagram +
  composable graph + tested/untested matrix.

For the in-flight duo reviews referencing this path, the equivalent data is
available in the CDP enum + the source code (apps/desktop/src/).
