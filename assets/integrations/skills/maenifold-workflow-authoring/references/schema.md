# Maenifold Workflow Schema (Canonical)

This schema mirrors the established shape in:

- `src/assets/workflows/think-tank.json`
- `src/assets/workflows/game-theory.json`

with terminology updates from enhanced thinking -> sequential thinking.

## Top-Level Fields

Required:

- `id` (string, kebab-case)
- `name` (string)
- `emoji` (string)
- `description` (string)
- `triggers` (array of non-empty strings)
- `sequentialThinkingEnabled` (boolean)
- `multiAgent` (boolean) — `true` if the workflow dispatches sub-agents via Task tool (requires main agent context); `false` if any agent can run it
- `steps` (non-empty array)

Optional:

- `specification` (string, URL) — Wikipedia or other canonical source
- Additional metadata fields used by specific workflows

## Step Fields

Required per step:

- `id` (string, kebab-case)
- `name` (string)
- `description` (string)
- `requiresSequentialThinking` (boolean)

Required per step:

- `metadata` (object)

Required metadata fields:

- `toolHints` (object keyed by tool names)
- `reasoning_effort` (`low` | `medium` | `high`)
- `stop_conditions` (array of strings)
- `next_actions` (array of strings)

Optional metadata fields:

- `escape_hatches` (object)
- `quality_gates` (object)
- `success_criteria` (object)

## Canonical Example

```json
{
  "id": "my-workflow",
  "name": "My Workflow",
  "emoji": "🧠",
  "description": "A structured workflow with sequential thinking hooks.",
  "triggers": [
    "my workflow",
    "example trigger"
  ],
  "sequentialThinkingEnabled": true,
  "multiAgent": false,
  "steps": [
    {
      "id": "define-problem",
      "name": "Define Problem",
      "description": "SEQUENTIAL THINKING REQUIRED\n\nDefine scope and success criteria.",
      "requiresSequentialThinking": true,
      "metadata": {
        "toolHints": {
          "SequentialThinking": {
            "reason": "Clarify problem framing",
            "priority": "mandatory",
            "estimated_thoughts": 3
          }
        },
        "reasoning_effort": "high",
        "stop_conditions": [
          "Scope agreed",
          "Success criteria documented"
        ],
        "next_actions": [
          "Proceed to analysis"
        ]
      }
    }
  ]
}
```

## Top-Level Classification Fields

### `multiAgent`
- `false` — any agent can run this workflow (single-agent execution)
- `true` — workflow dispatches sub-agents via the Task tool; requires main agent context (sub-agents don't have Task tool access)

## Legacy-to-Canonical Mapping

- `enhancedThinkingEnabled` -> `sequentialThinkingEnabled`
- `requiresEnhancedThinking` -> `requiresSequentialThinking`
- Text marker `ENHANCED THINKING REQUIRED` -> `SEQUENTIAL THINKING REQUIRED`

Use `scripts/workflow_schema_tool.py update` to migrate legacy files.
