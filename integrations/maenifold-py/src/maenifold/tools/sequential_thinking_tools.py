"""
Sequential thinking tools for structured problem-solving with concept integration.

Implements the sequential_thinking function with session management, branching,
revision support, and workflow integration.
"""

import os
from datetime import datetime
from typing import Optional

from maenifold.config import Config
from maenifold.utils.markdown_io import (
    extract_wiki_links,
    read_markdown,
    write_markdown,
)

# Checkpoint frequency for memory hints
_CHECKPOINT_FREQUENCY = 3


def sequential_thinking(
    response: Optional[str] = None,
    next_thought_needed: bool = False,
    thought_number: int = 0,
    total_thoughts: int = 0,
    session_id: Optional[str] = None,
    cancel: bool = False,
    thoughts: Optional[str] = None,
    is_revision: bool = False,
    revises_thought: Optional[int] = None,
    branch_from_thought: Optional[int] = None,
    branch_id: Optional[str] = None,
    needs_more_thoughts: bool = False,
    analysis_type: Optional[str] = None,
    parent_workflow_id: Optional[str] = None,
    conclusion: Optional[str] = None,
    learn: bool = False,
) -> str:
    """
    Creates structured thinking sessions with [[concept]] integration and persistent markdown file storage.

    Args:
        response: Main response/thought - MUST include [[concepts]] to build knowledge
        next_thought_needed: Need another thought?
        thought_number: Current thought number
        total_thoughts: Total thoughts estimate
        session_id: Session ID (auto-generated if not provided)
        cancel: Cancel session (set to true to cancel)
        thoughts: Ambient/meta thoughts with [[concepts]] (use liberally)
        is_revision: Is this a revision?
        revises_thought: Which thought to revise
        branch_from_thought: Branch from thought
        branch_id: Branch ID
        needs_more_thoughts: Need more thoughts than estimated?
        analysis_type: Analysis type: bug, architecture, retrospective, or complex
        parent_workflow_id: Parent workflow session ID (creates bidirectional link)
        conclusion: Required conclusion with confession when next_thought_needed=False
        learn: Return help documentation instead of executing

    Returns:
        Status message with continuation guidance or completion confirmation
    """
    if learn:
        return _get_help_content()

    # Skip validation if cancel operation
    if not cancel:
        is_valid, validation_error = _validate_thinking_input(response, thoughts)
        if not is_valid:
            return validation_error

    session_id_provided = session_id is not None

    if session_id is None:
        session_id = f"session-{int(datetime.utcnow().timestamp() * 1000)}"

    if session_id_provided and not _is_valid_session_id_format(session_id):
        return "ERROR: Invalid sessionId format. Use maenifold-generated values (session-{unix-milliseconds}) or omit sessionId to start a new session."

    session_exists = _session_exists("sequential", session_id)

    if session_id_provided and thought_number == 0 and not session_exists and not is_revision:
        return f"ERROR: Session {session_id} not found. To start new session, don't provide sessionId."

    parent_workflow_error = _validate_parent_workflow(parent_workflow_id, thought_number)
    if parent_workflow_error:
        return parent_workflow_error

    # Validate branchId requirement for multi-agent safety
    if branch_from_thought is not None and not branch_id:
        return "ERROR: branchId required when branchFromThought is specified for multi-agent coordination"

    if thought_number == 0 and not branch_id and session_exists and not is_revision and not cancel:
        return f"ERROR: Session {session_id} exists. Use different sessionId or continue existing."
    if thought_number > 0 and not session_exists and not is_revision and not cancel:
        return f"ERROR: Session {session_id} missing. Start with thoughtNumber=0."
    if is_revision and not session_exists:
        return f"ERROR: Cannot revise - session {session_id} doesn't exist."

    if not session_exists:
        _create_session(session_id, analysis_type, parent_workflow_id)
        if parent_workflow_id:
            _link_parent_workflow(session_id, parent_workflow_id)

    # Only append thought section if not cancelling
    if not cancel and response is not None:
        heading, content = _build_thought_section(
            thought_number,
            total_thoughts,
            needs_more_thoughts,
            branch_id,
            is_revision,
            revises_thought,
            response,
            thoughts,
        )
        _append_to_session("sequential", session_id, heading, content)

    complete = not next_thought_needed
    if complete and not cancel:
        if not conclusion:
            return "ERROR: Conclusion required when completing session. Must synthesize findings with [[concepts]]."

        conclusion_concepts = extract_wiki_links(conclusion)
        if not conclusion_concepts:
            return "ERROR: Conclusion must include [[concepts]] for knowledge graph integration."

    _finalize_session(session_id, thought_number, cancel, complete, conclusion)

    response_message = _build_completion_message(
        thought_number, session_id, cancel, next_thought_needed, needs_more_thoughts, total_thoughts
    )
    return response_message


# === Private Helper Functions ===


def _validate_thinking_input(
    response: Optional[str], thoughts: Optional[str]
) -> tuple[bool, str]:
    """Validate that response or thoughts contain [[concepts]]."""
    response_concepts = extract_wiki_links(response) if response else []
    thoughts_concepts = extract_wiki_links(thoughts) if thoughts else []
    total_concepts = len(response_concepts) + len(thoughts_concepts)

    if total_concepts == 0:
        return (
            False,
            "ERROR: Must include [[concepts]]. Example: 'Analyzing [[Machine Learning]] algorithms'",
        )

    return (True, "")


def _is_valid_session_id_format(session_id: str) -> bool:
    """Validate session ID format: session-{unix-milliseconds}."""
    last_dash = session_id.rfind("-")
    if last_dash < 0 or last_dash == len(session_id) - 1:
        return False

    suffix = session_id[last_dash + 1 :]
    try:
        int(suffix)
        return True
    except ValueError:
        return False


def _validate_parent_workflow(
    parent_workflow_id: Optional[str], thought_number: int
) -> Optional[str]:
    """Validate parent workflow exists and is active."""
    if not parent_workflow_id:
        return None

    if thought_number != 0:
        return "ERROR: Parent workflow can only be set on first thought."

    if not _session_exists("workflow", parent_workflow_id):
        return f"ERROR: Parent workflow '{parent_workflow_id}' not found."

    parent_meta, _, _ = _read_session("workflow", parent_workflow_id)
    parent_status = parent_meta.get("status", "active") if parent_meta else "active"

    if parent_status in ("completed", "cancelled", "abandoned"):
        return f"ERROR: Parent workflow is {parent_status}."

    return None


def _build_thought_section(
    thought_number: int,
    total_thoughts: int,
    needs_more_thoughts: bool,
    branch_id: Optional[str],
    is_revision: bool,
    revises_thought: Optional[int],
    response: str,
    thoughts: Optional[str],
) -> tuple[str, str]:
    """Build thought section heading and content."""
    display_total = (
        thought_number + 1
        if needs_more_thoughts and thought_number >= total_thoughts
        else total_thoughts
    )
    agent_id = os.environ.get("AGENT_ID", "agent")

    suffix = ""
    if branch_id:
        suffix = f" (Branch: {branch_id})"
    elif is_revision and revises_thought is not None:
        suffix = f" (Revises: {revises_thought})"

    heading = f"Thought {thought_number}/{display_total} [{agent_id}]{suffix}"

    thoughts_section = f"\n\n*Thoughts: {thoughts}*" if thoughts else ""
    timestamp = datetime.utcnow().strftime("%Y-%m-%d %H:%M:%S")
    content = f"{response}{thoughts_section}\n\n*{timestamp}*\n"

    return (heading, content)


def _create_session(
    session_id: str, analysis_type: Optional[str], parent_workflow_id: Optional[str]
) -> None:
    """Create new sequential thinking session."""
    frontmatter = {
        "title": f"Sequential Thinking Session {session_id}",
        "permalink": session_id,
        "type": "sequential",
        "status": "active",
    }

    if analysis_type:
        frontmatter["analysisType"] = analysis_type

    if parent_workflow_id:
        frontmatter["parent"] = f"[[workflow/{parent_workflow_id}]]"

    initial_content = "# Sequential Thinking Session\n\n"

    path = _get_session_path("sequential", session_id)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    write_markdown(path, frontmatter, initial_content)


def _link_parent_workflow(session_id: str, parent_workflow_id: str) -> None:
    """Create bidirectional link to parent workflow."""
    parent_meta, parent_content, _ = _read_session("workflow", parent_workflow_id)
    if parent_meta is None:
        parent_meta = {}

    related = parent_meta.get("related", [])
    if not isinstance(related, list):
        related = []

    related.append(f"[[sequential/{session_id}]]")
    parent_meta["related"] = related

    path = _get_session_path("workflow", parent_workflow_id)
    write_markdown(path, parent_meta, parent_content)


def _finalize_session(
    session_id: str,
    thought_number: int,
    cancel: bool,
    complete: bool,
    conclusion: Optional[str],
) -> None:
    """Finalize session with cancellation or completion."""
    if cancel:
        frontmatter, existing_content, _ = _read_session("sequential", session_id)
        if frontmatter is None:
            frontmatter = {}
        frontmatter["status"] = "cancelled"
        frontmatter["cancelled"] = datetime.utcnow().isoformat()
        if thought_number >= 0:
            frontmatter["thoughtCount"] = thought_number + 1
        elif "thoughtCount" not in frontmatter:
            frontmatter["thoughtCount"] = 0

        path = _get_session_path("sequential", session_id)
        write_markdown(path, frontmatter, existing_content)

    elif complete and conclusion:
        _append_to_session("sequential", session_id, "Conclusion", conclusion)

        frontmatter, existing_content, _ = _read_session("sequential", session_id)
        if frontmatter is None:
            frontmatter = {}
        frontmatter["status"] = "completed"
        frontmatter["completed"] = datetime.utcnow().isoformat()
        frontmatter["thoughtCount"] = thought_number + 1

        path = _get_session_path("sequential", session_id)
        write_markdown(path, frontmatter, existing_content)


def _build_completion_message(
    thought_number: int,
    session_id: str,
    cancel: bool,
    next_thought_needed: bool,
    needs_more_thoughts: bool,
    total_thoughts: int,
) -> str:
    """Build completion/continuation message."""
    base_message = (
        f"Created session: {session_id}"
        if thought_number == 0
        else f"Added thought {thought_number} to session: {session_id}"
    )

    if cancel:
        return f"{base_message}\n\n❌ Thinking cancelled"
    if not next_thought_needed:
        return f"{base_message}\n\n✅ Thinking complete"

    next_thought = thought_number + 1
    next_total = (
        "?"
        if needs_more_thoughts and thought_number >= total_thoughts - 1
        else str(total_thoughts)
    )
    extend_note = (
        " (extending beyond initial estimate)"
        if needs_more_thoughts and thought_number >= total_thoughts - 1
        else ""
    )
    checkpoint_note = (
        "\n\n💡 **CHECK YOUR MEMORY:** `search_memories` for what exists and `build_context` on [[concepts]] | `sync` new findings to add them to the graph"
        if thought_number == 0 or thought_number % _CHECKPOINT_FREQUENCY == 0
        else ""
    )

    return f"{base_message}\n\n💭 Continue with thought {next_thought}/{next_total}{extend_note}{checkpoint_note}"


# === Session Management Functions ===


def _get_session_path(session_type: str, session_id: str) -> str:
    """Get file path for session based on timestamp in session ID."""
    # Extract timestamp from session-{timestamp}
    last_dash = session_id.rfind("-")
    timestamp_str = session_id[last_dash + 1 :]
    timestamp_ms = int(timestamp_str)

    # Convert to datetime
    date = datetime.utcfromtimestamp(timestamp_ms / 1000.0)

    # Build path: thinking/{type}/YYYY/MM/DD/{sessionId}.md
    return os.path.join(
        Config.memory_path,
        "thinking",
        session_type,
        date.strftime("%Y"),
        date.strftime("%m"),
        date.strftime("%d"),
        f"{session_id}.md",
    )


def _session_exists(session_type: str, session_id: str) -> bool:
    """Check if session file exists."""
    try:
        path = _get_session_path(session_type, session_id)
        return os.path.exists(path)
    except (ValueError, IndexError):
        return False


def _read_session(session_type: str, session_id: str) -> tuple[dict, str, str]:
    """Read session file and return (frontmatter, content, checksum)."""
    path = _get_session_path(session_type, session_id)
    return read_markdown(path)


def _append_to_session(
    session_type: str, session_id: str, heading: str, content: str
) -> None:
    """Append H2 section to session file."""
    path = _get_session_path(session_type, session_id)

    # Read existing content
    existing_content = ""
    if os.path.exists(path):
        with open(path, "r", encoding="utf-8") as f:
            existing_content = f.read()

    # Ensure trailing newline
    if existing_content and not existing_content.endswith("\n"):
        existing_content += "\n"

    # Append new section
    new_content = existing_content + f"\n## {heading}\n\n{content}\n"

    with open(path, "w", encoding="utf-8") as f:
        f.write(new_content)


def _get_help_content() -> str:
    """Return help documentation."""
    return """# Sequential Thinking

A detailed tool for dynamic and reflective problem-solving through thoughts.

## Parameters

- response: Main response/thought - MUST include [[concepts]]
- next_thought_needed: True if more thinking needed
- thought_number: Current thought number (start at 0)
- total_thoughts: Estimated total thoughts needed
- session_id: Continue existing session (auto-generated if omitted)
- cancel: Set to true to cancel session
- thoughts: Optional meta/ambient notes with [[concepts]]
- is_revision: True if revising previous thought
- revises_thought: Which thought number to revise
- branch_from_thought: Branch point thought number
- branch_id: Branch identifier
- needs_more_thoughts: True if extending beyond estimate
- analysis_type: Optional type (bug, architecture, retrospective, complex)
- parent_workflow_id: Link to parent workflow (first thought only)
- conclusion: Required when next_thought_needed=False (must include [[concepts]])
- learn: Return this help text

## Returns

Status message with continuation guidance or completion confirmation.

## Session Format

Sessions are stored at: thinking/sequential/YYYY/MM/DD/session-{timestamp}.md

## Key Requirements

1. Response or thoughts MUST include [[concepts]]
2. Conclusion required when completing (next_thought_needed=False)
3. Conclusion must include [[concepts]]
4. Parent workflow only on first thought (thought_number=0)
5. Branch ID required when branching

## Example

```python
sequential_thinking(
    response="Analyzing [[Machine Learning]] algorithms for [[classification]]",
    next_thought_needed=True,
    thought_number=0,
    total_thoughts=5,
    thoughts="Need to consider [[accuracy]] vs [[speed]] tradeoffs"
)
```
"""
