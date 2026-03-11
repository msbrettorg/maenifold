---
name: maenifold-workflow-authoring
description: Create, update, and validate Maenifold workflow JSON assets under `src/assets/workflows`. Use when users ask to scaffold new workflows, migrate legacy workflow schema fields, enforce schema consistency, or verify workflow definitions. Applies the think-tank/game-theory schema shape with renamed sequential thinking fields (`sequentialThinkingEnabled`, `requiresSequentialThinking`) and includes deterministic validation and migration commands.
---

# Maenifold Workflow Authoring

## Run Critical Tool Help First

Before creating, updating, or validating any workflow, load current tool behavior with:

1. `GetHelp` for `Workflow`
2. `GetHelp` for `SequentialThinking`
3. `GetHelp` for `Adopt`

Treat this as mandatory preflight.

## Use Maenifold Reasoning Primitives

For non-trivial workflow changes:

1. Start a `SequentialThinking` session (`thoughtNumber=0`) to plan schema decisions.
2. Record schema assumptions in `AssumptionLedger` (`action=append`).
3. Update assumption status (`validated` or `refined`) after validation passes.

## Canonical Workflow Schema

Use the same structure as `think-tank` and `game-theory`, with renamed keys:

- Top level: `sequentialThinkingEnabled` (replaces `enhancedThinkingEnabled`)
- Step level: `requiresSequentialThinking` (replaces `requiresEnhancedThinking`)

Read [references/schema.md](./references/schema.md) before edits.

## Deterministic Script

Use [scripts/workflow_schema_tool.py](./scripts/workflow_schema_tool.py) for all operations:

```bash
python3 scripts/workflow_schema_tool.py create --id my-workflow --name "My Workflow" --description "..." --output src/assets/workflows/my-workflow.json
python3 scripts/workflow_schema_tool.py update src/assets/workflows/my-workflow.json --in-place
python3 scripts/workflow_schema_tool.py validate src/assets/workflows/my-workflow.json --mode canonical
```

## Create Workflow

1. Scaffold with `create`.
2. Fill steps with clear `id`, `name`, `description`, and `requiresSequentialThinking`.
3. Add `metadata.toolHints`, `metadata.reasoning_effort`, `metadata.stop_conditions`, and `metadata.next_actions` as needed.
4. Validate with `--mode canonical`.

## Update Existing Workflow

1. Run `update` to migrate legacy keys and text:
   - `enhancedThinkingEnabled` -> `sequentialThinkingEnabled`
   - `requiresEnhancedThinking` -> `requiresSequentialThinking`
   - `ENHANCED THINKING REQUIRED` -> `SEQUENTIAL THINKING REQUIRED`
2. Re-run `validate --mode canonical`.
3. If migration changed semantics, review steps manually.

## Validate Workflow

Use `validate` to enforce:

- Required top-level fields and step fields
- Unique workflow and step IDs
- Boolean typing for thinking flags
- Trigger list quality
- Canonical field names (in canonical mode)
- Reasoning effort values (`low`, `medium`, `high`) when present

Use `--mode compatible` only when auditing legacy files before migration.

## Research Ingest

For ingesting Wikipedia specification articles into `memory://` before authoring workflows, use the **wiki-research** skill. It converts Wikipedia hyperlinks to `[[WikiLinks]]` and produces a concept index for coverage tracking.

## Final Check

After edits:

1. Validate target workflow files.
2. Optionally run validation across all workflows:
   - `python3 scripts/workflow_schema_tool.py validate src/assets/workflows/*.json --mode canonical`
3. Keep JSON formatted with two-space indentation and stable key ordering from the script output.
