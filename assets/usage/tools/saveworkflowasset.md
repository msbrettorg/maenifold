# SaveWorkflowAsset

Creates or replaces a workflow asset JSON file under the workflows asset catalog.

Parameters:
- id (string, required): kebab-case workflow asset identifier
- content (string, required): JSON object text whose internal `id` matches the payload id
- learn (boolean, optional): when true, return this help text instead of writing a file

Usage examples:
- `{"learn":true}`
- `{"id":"daily-triage","content":"{\"id\":\"daily-triage\",\"nodes\":[]}"}`
