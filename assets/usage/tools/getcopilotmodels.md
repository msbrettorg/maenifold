# GetCopilotModels

Returns the locally available GitHub Copilot chat/completions models known to maenifold.

Parameters:
- refresh (boolean, optional): when true, refresh the cached model list before returning results
- learn (boolean, optional): when true, return this help text instead of querying Copilot

Usage examples:
- `{"learn":true}`
- `{"refresh":true}`
