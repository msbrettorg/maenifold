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
- `subWorkflows` (array of strings, optional) — workflow IDs referenced by this step (escape hatches, recursive expansions). Used by the pipeline DAG expander to recursively load and expand sub-workflow definitions into the execution graph. Each string must be a valid workflow ID matching a file in `assets/workflows/`.

Required per step:

- `metadata` (object)

Required metadata fields:

- `toolHints` (object keyed by tool names)
- `reasoning_effort` (`low` | `medium` | `high`)
- `stop_conditions` (array of strings)
- `next_actions` (array of strings)

Optional metadata fields:

- `escape_hatches` (object keyed by hatch names; each value is an object with `workflowId` and `condition` fields)
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
    },
    {
      "id": "analyze-results",
      "name": "Analyze Results",
      "description": "SEQUENTIAL THINKING REQUIRED\n\nAnalyze experimental data and determine next steps.",
      "requiresSequentialThinking": true,
      "subWorkflows": ["strong-inference", "paradigm-analysis"],
      "metadata": {
        "toolHints": {
          "SequentialThinking": {
            "reason": "Statistical and interpretive analysis",
            "priority": "mandatory",
            "estimated_thoughts": 5
          }
        },
        "reasoning_effort": "high",
        "stop_conditions": [
          "All hypotheses evaluated",
          "Posteriors computed"
        ],
        "next_actions": [
          "Draw conclusions or escalate to sub-workflow"
        ],
        "escape_hatches": {
          "multiple_survivors": {
            "workflowId": "strong-inference",
            "condition": "Multiple hypotheses survive with similar posteriors"
          },
          "anomalous_results": {
            "workflowId": "paradigm-analysis",
            "condition": "Results challenge existing frameworks"
          }
        }
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
