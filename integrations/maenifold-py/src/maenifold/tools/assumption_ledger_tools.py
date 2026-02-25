"""
Assumption ledger tools for declarative assumption tracking.

Implements assumption_ledger function for creating, updating, and reading
assumptions with minimal structure and knowledge graph integration.
"""

import os
from datetime import datetime
from typing import Optional

from maenifold.config import Config
from maenifold.utils.markdown_io import (
    path_to_uri,
    read_markdown,
    uri_to_path,
    write_markdown,
)


# Valid statuses for assumptions
_VALID_STATUSES = ["active", "validated", "invalidated", "refined"]


def assumption_ledger(
    action: str,
    assumption: Optional[str] = None,
    context: Optional[str] = None,
    validation_plan: Optional[str] = None,
    confidence: Optional[str] = None,
    concepts: Optional[list[str]] = None,
    uri: Optional[str] = None,
    status: Optional[str] = None,
    notes: Optional[str] = None,
    learn: bool = False,
) -> str:
    """
    Declare, update, or query assumptions with minimal structure.

    Captures agent reasoning without auto-inference or scoring.
    Integrates with SequentialThinking and Workflow sessions via context references.
    Returns declarative next-step suggestions aligned with Ma Protocol restraint.

    Args:
        action: Action to perform: "append" (create new), "update" (modify existing), "read" (view existing)
        assumption: Assumption statement (required for append)
        context: Context reference: workflow://thinking/session-ID or sequential session ID
        confidence: Current confidence: free text (e.g., 'high', 'medium', 'low', 'needs-verification')
        validation_plan: Planned validation method
        status: Status update: validated, invalidated, or refined (for update action)
        notes: Additional notes for updates
        uri: Memory URI for update/read actions (e.g., memory://assumptions/2025/01/assumption-1234567890)
        concepts: Agent-supplied [[concept]] tags for knowledge graph integration
        learn: Return help documentation instead of executing

    Returns:
        Status message with assumption URI and next-step suggestions
    """
    if learn:
        return _get_help_content()

    try:
        action_lower = action.lower()
        if action_lower == "append":
            return _append_assumption(assumption, context, validation_plan, confidence, concepts)
        elif action_lower == "update":
            return _update_assumption(uri, status, notes, validation_plan, confidence)
        elif action_lower == "read":
            return _read_assumption(uri)
        else:
            return "ERROR: Invalid action. Use 'append', 'update', or 'read'."
    except Exception as ex:
        return f"ERROR: {ex}"


# === Private Helper Functions ===


def _append_assumption(
    assumption: Optional[str],
    context: Optional[str],
    validation_plan: Optional[str],
    confidence: Optional[str],
    concepts: Optional[list[str]],
) -> str:
    """Create new assumption."""
    # Validate inputs
    validation_error = _validate_append_input(assumption, concepts)
    if validation_error:
        return validation_error

    # Generate path and create directory
    full_path = _generate_assumption_path()
    os.makedirs(os.path.dirname(full_path), exist_ok=True)

    # Build frontmatter
    date = datetime.utcnow()
    frontmatter = _build_append_frontmatter(date, context, validation_plan, confidence)

    # Build content
    content = _build_append_content(assumption, validation_plan, concepts)

    # Write file
    write_markdown(full_path, frontmatter, content)

    # Generate response
    memory_uri = path_to_uri(full_path, Config.memory_path)
    response = _build_append_response(memory_uri, assumption, confidence, context, validation_plan)

    return response


def _update_assumption(
    uri: Optional[str],
    status: Optional[str],
    notes: Optional[str],
    validation_plan: Optional[str],
    confidence: Optional[str],
) -> str:
    """Update existing assumption."""
    if not uri or not uri.strip():
        return "ERROR: URI is required for 'update' action."

    # Validate status if provided
    if status:
        status_error = _validate_status(status)
        if status_error:
            return status_error

    try:
        path = uri_to_path(uri, Config.memory_path)

        if not os.path.exists(path):
            return f"ERROR: Assumption not found at {uri}"

        # Read existing content
        frontmatter, content, _ = read_markdown(path)

        if frontmatter is None:
            frontmatter = {}

        # Update frontmatter
        _build_frontmatter_updates(frontmatter, status, confidence, validation_plan)

        # Append notes if provided
        updated_content = _append_update_notes(content, notes)

        # Write updated file
        write_markdown(path, frontmatter, updated_content)

        # Generate response
        response = _build_update_response(uri, status, confidence)

        return response

    except ValueError as ex:
        return f"ERROR: {ex}"


def _read_assumption(uri: Optional[str]) -> str:
    """Read existing assumption."""
    if not uri or not uri.strip():
        return "ERROR: URI is required for 'read' action."

    try:
        path = uri_to_path(uri, Config.memory_path)

        if not os.path.exists(path):
            return f"ERROR: Assumption not found at {uri}"

        # Read file
        frontmatter, content, _ = read_markdown(path)

        # Generate response
        response = _build_read_response(uri, frontmatter, content)

        return response

    except ValueError as ex:
        return f"ERROR: {ex}"


def _generate_assumption_path() -> str:
    """Generate date-based path for new assumption file."""
    timestamp = int(datetime.utcnow().timestamp() * 1000)
    date = datetime.utcnow()
    year = date.strftime("%Y")
    month = date.strftime("%m")
    file_name = f"assumption-{timestamp}.md"

    return os.path.join(Config.memory_path, "assumptions", year, month, file_name)


def _build_append_frontmatter(
    date: datetime,
    context: Optional[str],
    validation_plan: Optional[str],
    confidence: Optional[str],
) -> dict:
    """Build frontmatter for new assumption."""
    frontmatter = {
        "created": date.isoformat() + "Z",
        "status": "active",
        "confidence": confidence if confidence else "unspecified",
    }

    if context and context.strip():
        frontmatter["context"] = context

    if validation_plan and validation_plan.strip():
        frontmatter["validation_plan"] = validation_plan

    return frontmatter


def _build_append_content(
    assumption: str,
    validation_plan: Optional[str],
    concepts: list[str],
) -> str:
    """Build markdown content for new assumption."""
    lines = [
        f"# Assumption: {assumption}",
        "",
        "## Statement",
        "",
        assumption,
        "",
    ]

    if validation_plan and validation_plan.strip():
        lines.extend([
            "## Validation Plan",
            "",
            validation_plan,
            "",
        ])

    lines.extend([
        "## Related Concepts",
        "",
    ])

    for concept in concepts:
        lines.append(f"- [[{concept}]]")

    return "\n".join(lines)


def _build_append_response(
    memory_uri: str,
    assumption: str,
    confidence: Optional[str],
    context: Optional[str],
    validation_plan: Optional[str],
) -> str:
    """Build response message for append operation."""
    lines = [
        f"✅ Assumption recorded at {memory_uri}",
        "",
        f"**Statement**: {assumption}",
        f"**Confidence**: {confidence if confidence else 'unspecified'}",
        "**Status**: active",
    ]

    if context and context.strip():
        lines.append(f"**Context**: {context}")

    lines.extend([
        "",
        "💡 Next: Consider running Sync() to index this assumption into the knowledge graph for SearchMemories discovery.",
    ])

    if validation_plan and validation_plan.strip():
        lines.append(f"💡 Remember to update status after completing: {validation_plan}")

    return "\n".join(lines)


def _build_frontmatter_updates(
    frontmatter: dict,
    status: Optional[str],
    confidence: Optional[str],
    validation_plan: Optional[str],
) -> None:
    """Update frontmatter fields in place."""
    if status and status.strip():
        frontmatter["status"] = status

    if confidence and confidence.strip():
        frontmatter["confidence"] = confidence

    if validation_plan and validation_plan.strip():
        frontmatter["validation_plan"] = validation_plan

    frontmatter["updated"] = datetime.utcnow().isoformat() + "Z"


def _append_update_notes(content: str, notes: Optional[str]) -> str:
    """Append timestamped notes to content."""
    if notes and notes.strip():
        timestamp = datetime.utcnow().strftime("%Y-%m-%d %H:%M:%S")
        content += f"\n## Update: {timestamp}\n\n{notes}\n"

    return content


def _build_update_response(
    uri: str,
    status: Optional[str],
    confidence: Optional[str],
) -> str:
    """Build response message for update operation."""
    lines = [f"✅ Assumption updated at {uri}"]

    if status and status.strip():
        lines.append(f"**New Status**: {status}")

    if confidence and confidence.strip():
        lines.append(f"**New Confidence**: {confidence}")

    lines.extend([
        "",
        "💡 Next: Run Sync() to update the knowledge graph with these changes.",
    ])

    return "\n".join(lines)


def _build_read_response(
    uri: str,
    frontmatter: Optional[dict],
    content: str,
) -> str:
    """Build response message for read operation."""
    lines = [
        f"📋 **Assumption**: {uri}",
        "",
    ]

    if frontmatter:
        if "status" in frontmatter:
            lines.append(f"**Status**: {frontmatter['status']}")

        if "confidence" in frontmatter:
            lines.append(f"**Confidence**: {frontmatter['confidence']}")

        if "created" in frontmatter:
            lines.append(f"**Created**: {frontmatter['created']}")

        if "updated" in frontmatter:
            lines.append(f"**Updated**: {frontmatter['updated']}")

        if "context" in frontmatter:
            lines.append(f"**Context**: {frontmatter['context']}")

        if "validation_plan" in frontmatter:
            lines.append(f"**Validation Plan**: {frontmatter['validation_plan']}")

        lines.append("")

    lines.extend([
        "## Content",
        "",
        content,
    ])

    return "\n".join(lines)


# === Validation Functions ===


def _validate_append_input(
    assumption: Optional[str],
    concepts: Optional[list[str]],
) -> Optional[str]:
    """Validate inputs for append action."""
    if not assumption or not assumption.strip():
        return "ERROR: Assumption text is required for 'append' action."

    if not concepts or len(concepts) == 0:
        return 'ERROR: At least one [[concept]] tag must be provided. Example: concepts: ["dialogue", "workflow-dispatch"]'

    # Validate concept format
    concept_error = _validate_concept_format(concepts)
    if concept_error:
        return concept_error

    return None


def _validate_concept_format(concepts: list[str]) -> Optional[str]:
    """Validate that concepts don't include bracket syntax."""
    for concept in concepts:
        if "[[" in concept or "]]" in concept:
            return "ERROR: Concept tags should not include brackets. Use 'dialogue' not '[[dialogue]]'."

    return None


def _validate_status(status: str) -> Optional[str]:
    """Validate status value."""
    if status.lower() not in _VALID_STATUSES:
        valid_list = ", ".join(_VALID_STATUSES)
        return f"ERROR: Invalid status. Use one of: {valid_list}"

    return None


def _get_help_content() -> str:
    """Return help documentation."""
    return """# AssumptionLedger

Declare, update, or query assumptions with minimal structure.

## Parameters

- action: "append" (create new), "update" (modify existing), "read" (view existing)
- assumption: Assumption statement (required for append)
- context: Context reference (workflow://thinking/session-ID or sequential session ID)
- confidence: Free text confidence level (e.g., 'high', 'medium', 'low', 'needs-verification')
- validation_plan: Planned validation method
- status: Status update for existing assumptions (validated, invalidated, refined)
- notes: Additional notes for updates
- uri: Memory URI for update/read operations
- concepts: List of [[concept]] tags (required for append, must NOT include brackets)
- learn: Return this help text

## Actions

### append
Create new assumption with:
- Required: assumption, concepts
- Optional: context, validation_plan, confidence

### update
Modify existing assumption with:
- Required: uri
- Optional: status, notes, validation_plan, confidence

### read
View existing assumption with:
- Required: uri

## Returns

Status message with assumption URI and next-step suggestions.

## Example

```python
assumption_ledger(
    action="append",
    assumption="The dialogue tool will remain MCP-only",
    concepts=["dialogue", "workflow-dispatch"],
    confidence="medium",
    validation_plan="Validate once dialogue MCP hooks are reintroduced",
    context="workflow://thinking/session-1756610546730"
)
```

## Storage

Assumptions are stored at: memory://assumptions/YYYY/MM/assumption-{timestamp}.md

Each file includes:
- YAML frontmatter with status, confidence, context
- Markdown content with statement, validation plan, and related concepts
- Integration with knowledge graph via [[concepts]]

## Valid Statuses

- active: Initial state
- validated: Confirmed correct
- invalidated: Proven incorrect
- refined: Updated/clarified
"""
