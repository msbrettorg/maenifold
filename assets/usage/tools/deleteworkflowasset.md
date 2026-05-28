# DeleteWorkflowAsset

Deletes a saved workflow asset after confirming it is not referenced by a running pipeline.

Parameters:
- name (string, required): kebab-case workflow asset name to delete
- learn (boolean, optional): when true, return this help text instead of deleting anything

Usage examples:
- `{"learn":true}`
- `{"name":"daily-triage"}`
