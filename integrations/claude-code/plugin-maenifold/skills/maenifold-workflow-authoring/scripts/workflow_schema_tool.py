#!/usr/bin/env python3
"""Create, update, and validate maenifold workflow JSON files."""

from __future__ import annotations

import argparse
import copy
import json
import re
import sys
from pathlib import Path
from typing import Any


REASONING_EFFORT_VALUES = {"low", "medium", "high"}
ID_PATTERN = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


def _load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        data = json.load(handle)
    if not isinstance(data, dict):
        raise ValueError(f"{path}: root JSON value must be an object")
    return data


def _write_json(path: Path, data: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(data, handle, indent=2, ensure_ascii=False)
        handle.write("\n")


def _replace_thinking_text(value: str) -> str:
    value = value.replace("ENHANCED THINKING REQUIRED", "SEQUENTIAL THINKING REQUIRED")
    value = re.sub(r"\bEnhanced Thinking\b", "Sequential Thinking", value)
    value = re.sub(r"\benhanced thinking\b", "sequential thinking", value)
    return value


def _walk_and_replace_text(node: Any) -> Any:
    if isinstance(node, dict):
        return {key: _walk_and_replace_text(value) for key, value in node.items()}
    if isinstance(node, list):
        return [_walk_and_replace_text(item) for item in node]
    if isinstance(node, str):
        return _replace_thinking_text(node)
    return node


def canonicalize_workflow(workflow: dict[str, Any]) -> tuple[dict[str, Any], list[str]]:
    migrated = copy.deepcopy(workflow)
    notes: list[str] = []

    if "enhancedThinkingEnabled" in migrated:
        if "sequentialThinkingEnabled" not in migrated:
            migrated["sequentialThinkingEnabled"] = migrated["enhancedThinkingEnabled"]
            notes.append("renamed top-level enhancedThinkingEnabled -> sequentialThinkingEnabled")
        del migrated["enhancedThinkingEnabled"]

    steps = migrated.get("steps")
    if isinstance(steps, list):
        for index, step in enumerate(steps):
            if not isinstance(step, dict):
                continue
            if "requiresEnhancedThinking" in step:
                if "requiresSequentialThinking" not in step:
                    step["requiresSequentialThinking"] = step["requiresEnhancedThinking"]
                    notes.append(
                        f"renamed steps[{index}].requiresEnhancedThinking -> requiresSequentialThinking"
                    )
                del step["requiresEnhancedThinking"]

    replaced = _walk_and_replace_text(migrated)
    if replaced != migrated:
        notes.append("updated enhanced thinking text markers to sequential thinking")
    migrated = replaced

    return migrated, notes


def _check_kebab_id(value: Any, label: str, errors: list[str]) -> None:
    if not isinstance(value, str) or not value.strip():
        errors.append(f"{label}: must be a non-empty string")
        return
    if not ID_PATTERN.match(value):
        errors.append(f"{label}: must be kebab-case (lowercase letters, numbers, hyphens)")


def _require_str(value: Any, label: str, errors: list[str]) -> None:
    if not isinstance(value, str) or not value.strip():
        errors.append(f"{label}: must be a non-empty string")


def validate_workflow(
    workflow: dict[str, Any],
    mode: str = "canonical",
) -> list[str]:
    errors: list[str] = []

    legacy_top = "enhancedThinkingEnabled" in workflow
    canonical_top = "sequentialThinkingEnabled" in workflow

    if mode == "canonical":
        if legacy_top:
            errors.append(
                "top-level: legacy field enhancedThinkingEnabled present; use sequentialThinkingEnabled"
            )
    if not canonical_top and not legacy_top:
        errors.append("top-level: missing sequentialThinkingEnabled")

    top_flag = workflow.get("sequentialThinkingEnabled")
    if top_flag is None and mode == "compatible":
        top_flag = workflow.get("enhancedThinkingEnabled")
    if top_flag is not None and not isinstance(top_flag, bool):
        errors.append("top-level sequential thinking flag must be boolean")

    _check_kebab_id(workflow.get("id"), "id", errors)
    _require_str(workflow.get("name"), "name", errors)
    _require_str(workflow.get("emoji"), "emoji", errors)
    _require_str(workflow.get("description"), "description", errors)

    # Multi-agent flag (required, boolean)
    multi_agent = workflow.get("multiAgent")
    if multi_agent is None:
        errors.append("multiAgent: required (boolean)")
    elif not isinstance(multi_agent, bool):
        errors.append("multiAgent: must be boolean")

    triggers = workflow.get("triggers")
    if not isinstance(triggers, list) or not triggers:
        errors.append("triggers: must be a non-empty array of strings")
    else:
        for index, trigger in enumerate(triggers):
            if not isinstance(trigger, str) or not trigger.strip():
                errors.append(f"triggers[{index}]: must be a non-empty string")

    steps = workflow.get("steps")
    if not isinstance(steps, list) or not steps:
        errors.append("steps: must be a non-empty array")
        return errors

    seen_ids: set[str] = set()
    for index, step in enumerate(steps):
        label = f"steps[{index}]"
        if not isinstance(step, dict):
            errors.append(f"{label}: must be an object")
            continue

        _check_kebab_id(step.get("id"), f"{label}.id", errors)
        step_id = step.get("id")
        if isinstance(step_id, str):
            if step_id in seen_ids:
                errors.append(f"{label}.id: duplicate id '{step_id}'")
            seen_ids.add(step_id)

        _require_str(step.get("name"), f"{label}.name", errors)
        _require_str(step.get("description"), f"{label}.description", errors)

        legacy_step = "requiresEnhancedThinking" in step
        canonical_step = "requiresSequentialThinking" in step
        if mode == "canonical" and legacy_step:
            errors.append(
                f"{label}: legacy field requiresEnhancedThinking present; use requiresSequentialThinking"
            )
        if not canonical_step and not legacy_step:
            errors.append(f"{label}: missing requiresSequentialThinking")

        step_flag = step.get("requiresSequentialThinking")
        if step_flag is None and mode == "compatible":
            step_flag = step.get("requiresEnhancedThinking")
        if step_flag is not None and not isinstance(step_flag, bool):
            errors.append(f"{label}: sequential thinking flag must be boolean")

        # T-WF-SCHEMA-001.1: RTM FR-27.x — metadata and core fields are required
        metadata = step.get("metadata")
        if metadata is None:
            errors.append(f"{label}.metadata: required")
        elif not isinstance(metadata, dict):
            errors.append(f"{label}.metadata: must be an object")
        else:
            # toolHints — required, must be object
            tool_hints = metadata.get("toolHints")
            if tool_hints is None:
                errors.append(f"{label}.metadata.toolHints: required")
            elif not isinstance(tool_hints, dict):
                errors.append(f"{label}.metadata.toolHints: must be an object")

            # reasoning_effort — required, must be valid value
            effort = metadata.get("reasoning_effort")
            if effort is None:
                errors.append(f"{label}.metadata.reasoning_effort: required")
            elif effort not in REASONING_EFFORT_VALUES:
                errors.append(
                    f"{label}.metadata.reasoning_effort: must be one of {sorted(REASONING_EFFORT_VALUES)}"
                )

            # stop_conditions — required, must be array
            stop_conditions = metadata.get("stop_conditions")
            if stop_conditions is None:
                errors.append(f"{label}.metadata.stop_conditions: required")
            elif not isinstance(stop_conditions, list):
                errors.append(f"{label}.metadata.stop_conditions: must be an array")

            # next_actions — required, must be array
            next_actions = metadata.get("next_actions")
            if next_actions is None:
                errors.append(f"{label}.metadata.next_actions: required")
            elif not isinstance(next_actions, list):
                errors.append(f"{label}.metadata.next_actions: must be an array")

        description = step.get("description")
        if (
            mode == "canonical"
            and isinstance(description, str)
            and "enhanced thinking" in description.lower()
        ):
            errors.append(f"{label}.description: contains legacy 'enhanced thinking' text")

    return errors


def cmd_create(args: argparse.Namespace) -> int:
    triggers = args.triggers if args.triggers else [args.id.replace("-", " ")]
    if args.step_count < 1:
        print("ERROR: --step-count must be >= 1", file=sys.stderr)
        return 1

    steps: list[dict[str, Any]] = []
    for index in range(1, args.step_count + 1):
        steps.append(
            {
                "id": f"step-{index}",
                "name": f"Step {index}",
                "description": "Describe objective, outputs, and required tool operations.",
                "requiresSequentialThinking": False,
                "metadata": {
                    "toolHints": {},
                    "reasoning_effort": "medium",
                    "stop_conditions": [],
                    "next_actions": [],
                },
            }
        )

    workflow: dict[str, Any] = {
        "id": args.id,
        "name": args.name,
        "emoji": args.emoji,
        "description": args.description,
        "triggers": triggers,
        "sequentialThinkingEnabled": args.sequential_thinking_enabled,
        "multiAgent": False,
        "steps": steps,
    }

    errors = validate_workflow(workflow, mode="canonical")
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    _write_json(Path(args.output), workflow)
    print(f"Created workflow scaffold: {args.output}")
    return 0


def cmd_update(args: argparse.Namespace) -> int:
    path = Path(args.path)
    workflow = _load_json(path)
    updated, notes = canonicalize_workflow(workflow)

    if updated == workflow:
        print(f"No changes needed: {path}")
    else:
        for note in notes:
            print(f"- {note}")

    output_path = path if args.in_place else Path(args.output) if args.output else None
    if output_path is None:
        print(json.dumps(updated, indent=2, ensure_ascii=False))
    else:
        _write_json(output_path, updated)
        print(f"Wrote updated workflow: {output_path}")

    return 0


def cmd_validate(args: argparse.Namespace) -> int:
    any_errors = False
    for raw_path in args.paths:
        path = Path(raw_path)
        try:
            workflow = _load_json(path)
        except Exception as exc:  # pylint: disable=broad-except
            print(f"{path}: ERROR: {exc}", file=sys.stderr)
            any_errors = True
            continue

        errors = validate_workflow(workflow, mode=args.mode)
        if errors:
            any_errors = True
            print(f"{path}: INVALID")
            for error in errors:
                print(f"  - {error}")
        else:
            print(f"{path}: OK")

    return 1 if any_errors else 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Create, update, and validate maenifold workflow JSON files."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    create_parser = subparsers.add_parser("create", help="Create a new workflow scaffold")
    create_parser.add_argument("--id", required=True, help="Workflow id (kebab-case)")
    create_parser.add_argument("--name", required=True, help="Workflow display name")
    create_parser.add_argument("--emoji", default="🧠", help="Workflow emoji")
    create_parser.add_argument("--description", required=True, help="Workflow description")
    create_parser.add_argument(
        "--triggers",
        nargs="+",
        default=[],
        help="One or more trigger phrases",
    )
    create_parser.add_argument(
        "--sequential-thinking-enabled",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Enable sequential thinking support",
    )
    create_parser.add_argument(
        "--step-count",
        type=int,
        default=3,
        help="Number of scaffolded steps",
    )
    create_parser.add_argument("--output", required=True, help="Output JSON file path")
    create_parser.set_defaults(func=cmd_create)

    update_parser = subparsers.add_parser(
        "update", help="Migrate legacy enhanced-thinking fields to sequential-thinking fields"
    )
    update_parser.add_argument("path", help="Workflow JSON path")
    update_parser.add_argument(
        "--in-place",
        action="store_true",
        help="Write changes to the input file",
    )
    update_parser.add_argument(
        "--output",
        help="Write changes to this path (ignored when --in-place is set)",
    )
    update_parser.set_defaults(func=cmd_update)

    validate_parser = subparsers.add_parser("validate", help="Validate one or more workflow files")
    validate_parser.add_argument("paths", nargs="+", help="Workflow JSON files")
    validate_parser.add_argument(
        "--mode",
        choices=["canonical", "compatible"],
        default="canonical",
        help="canonical enforces sequential-thinking fields; compatible allows legacy fields",
    )
    validate_parser.set_defaults(func=cmd_validate)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
