"""
Workflow orchestration tool for systematic problem-solving through predefined methodologies.

Workflows provide structured, multi-step approaches with:
- Predefined step sequences loaded from JSON definitions
- Session persistence with frontmatter and content tracking
- Queue management for chaining multiple workflows
- [[Concept]] validation for knowledge graph integration
- Sequential thinking integration for embedded analysis
"""

import json
import os
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

from maenifold.config import Config
from maenifold.utils.markdown_io import extract_wiki_links, read_markdown, write_markdown


# ===== Session Management =====


def get_session_path(thinking_type: str, session_id: str) -> str:
    """
    Get the full file path for a session file.

    Session path format: {memory_path}/thinking/{thinking_type}/{year}/{month}/{day}/{session_id}.md

    Args:
        thinking_type: Type of session (e.g., "workflow", "sequential")
        session_id: Session identifier (format: {type}-{unix_ms_timestamp})

    Returns:
        Absolute path to session file

    Raises:
        ValueError: If session_id format is invalid
    """
    # Extract timestamp from session_id (format: workflow-1768421493459)
    try:
        timestamp_str = session_id.split('-')[-1]
        timestamp_ms = int(timestamp_str)
    except (ValueError, IndexError) as e:
        raise ValueError(f"Invalid session_id format: {session_id}. Expected format: {{type}}-{{unix_ms_timestamp}}") from e

    # Convert to datetime
    dt = datetime.fromtimestamp(timestamp_ms / 1000.0, tz=timezone.utc)

    # Build path
    year = dt.strftime("%Y")
    month = dt.strftime("%m")
    day = dt.strftime("%d")

    path = os.path.join(
        Config.memory_path,
        "thinking",
        thinking_type,
        year,
        month,
        day,
        f"{session_id}.md"
    )

    return path


def session_exists(thinking_type: str, session_id: str) -> bool:
    """
    Check if a session file exists.

    Args:
        thinking_type: Type of session
        session_id: Session identifier

    Returns:
        True if session file exists
    """
    try:
        path = get_session_path(thinking_type, session_id)
        return os.path.exists(path)
    except ValueError:
        return False


def create_session(
    thinking_type: str,
    session_id: str,
    frontmatter: dict,
    content: str
) -> None:
    """
    Create a new session file.

    Args:
        thinking_type: Type of session
        session_id: Session identifier
        frontmatter: Session metadata dictionary
        content: Initial session content
    """
    path = get_session_path(thinking_type, session_id)

    # Ensure parent directories exist
    parent_dir = os.path.dirname(path)
    os.makedirs(parent_dir, exist_ok=True)

    write_markdown(path, frontmatter, content)


def read_session(thinking_type: str, session_id: str) -> tuple[Optional[dict], str, str]:
    """
    Read a session file.

    Args:
        thinking_type: Type of session
        session_id: Session identifier

    Returns:
        Tuple of (frontmatter, content, checksum)

    Raises:
        FileNotFoundError: If session file doesn't exist
    """
    path = get_session_path(thinking_type, session_id)
    return read_markdown(path)


def update_session(
    thinking_type: str,
    session_id: str,
    frontmatter: dict,
    content: str
) -> None:
    """
    Update an existing session file.

    Args:
        thinking_type: Type of session
        session_id: Session identifier
        frontmatter: Updated metadata dictionary
        content: Updated content
    """
    path = get_session_path(thinking_type, session_id)
    write_markdown(path, frontmatter, content)


def append_to_session(
    thinking_type: str,
    session_id: str,
    heading: str,
    content: str
) -> None:
    """
    Append an H2 section to a session file.

    Args:
        thinking_type: Type of session
        session_id: Session identifier
        heading: Section heading (without ## prefix)
        content: Section content
    """
    path = get_session_path(thinking_type, session_id)

    # Read existing content
    existing_content = ""
    if os.path.exists(path):
        with open(path, 'r', encoding='utf-8') as f:
            existing_content = f.read()

    # Ensure trailing newline
    if existing_content and not existing_content.endswith('\n'):
        existing_content += '\n'

    # Append new section
    new_section = f"\n## {heading}\n\n{content}\n"

    with open(path, 'w', encoding='utf-8') as f:
        f.write(existing_content + new_section)


# ===== Workflow Operations =====


def _get_workflows_path() -> str:
    """Get the path to the workflows directory."""
    return os.path.join(Config.assets_path, "workflows")


def _load_workflow(workflow_id: str) -> dict:
    """
    Load workflow definition from JSON file.

    Args:
        workflow_id: Workflow identifier

    Returns:
        Workflow definition dictionary

    Raises:
        FileNotFoundError: If workflow file doesn't exist
        ValueError: If workflow JSON is invalid
    """
    workflow_path = os.path.join(_get_workflows_path(), f"{workflow_id}.json")

    if not os.path.exists(workflow_path):
        raise FileNotFoundError(f"Workflow '{workflow_id}' not found at {workflow_path}")

    try:
        with open(workflow_path, 'r', encoding='utf-8') as f:
            workflow = json.load(f)
    except json.JSONDecodeError as e:
        raise ValueError(f"Invalid JSON in workflow '{workflow_id}': {e}") from e

    # Validate workflow structure
    if "steps" not in workflow:
        raise ValueError(f"Workflow '{workflow_id}' is missing 'steps' property")

    if not workflow["steps"]:
        raise ValueError(f"Workflow '{workflow_id}' has no steps")

    return workflow


def _validate_concepts(response: str, thoughts: Optional[str]) -> Optional[str]:
    """
    Validate that response or thoughts contain at least one [[concept]].

    Args:
        response: Response text
        thoughts: Optional thoughts text

    Returns:
        Error message if validation fails, None if valid
    """
    response_concepts = extract_wiki_links(response)
    thoughts_concepts = extract_wiki_links(thoughts) if thoughts else []
    total_concepts = len(response_concepts) + len(thoughts_concepts)

    if total_concepts == 0:
        return (
            "ERROR: Response or thoughts must contain at least one [[concept]] in double brackets.\n"
            "Example: 'Implementing [[REST API]] using [[ASP.NET Core]]'\n"
            "This ensures your workflow contributions build the knowledge graph."
        )

    return None


def _start_workflow(workflow_id: str) -> str:
    """
    Start a new workflow session.

    Args:
        workflow_id: Workflow ID or JSON array of workflow IDs

    Returns:
        Success message with first step
    """
    # Parse workflow ID(s)
    ids = []
    if workflow_id.startswith('['):
        try:
            ids = json.loads(workflow_id)
        except json.JSONDecodeError as e:
            return f"ERROR: Invalid JSON array for workflowId: {e}"
    else:
        ids.append(workflow_id)

    # Generate session ID
    timestamp_ms = int(datetime.now(timezone.utc).timestamp() * 1000)
    session_id = f"workflow-{timestamp_ms}"

    # Load first workflow
    try:
        first_workflow = _load_workflow(ids[0])
    except (FileNotFoundError, ValueError) as e:
        return f"ERROR: {e}"

    steps = first_workflow["steps"]

    # Create initial content
    content_lines = [
        "# Workflow Session",
        "",
        f"Started: {datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M:%S')}",
        "",
        f"## Step 1/{len(steps)}",
        "",
        json.dumps(steps[0], indent=2)
    ]
    content = "\n".join(content_lines)

    # Create frontmatter
    frontmatter = {
        "title": f"Workflow Session {session_id}",
        "permalink": session_id,
        "type": "workflow",
        "status": "active",
        "queue": ids,
        "currentWorkflow": 0,
        "currentStep": 0
    }

    # Create session file
    create_session("workflow", session_id, frontmatter, content)

    return f"Created session: {session_id}\n\nStep 1/{len(steps)}\n\n{json.dumps(steps[0], indent=2)}"


def _view_workflow(session_id: str) -> str:
    """
    View workflow queue status.

    Args:
        session_id: Session identifier

    Returns:
        Queue status message
    """
    if not session_exists("workflow", session_id):
        return f"ERROR: Session {session_id} doesn't exist"

    try:
        frontmatter, _, _ = read_session("workflow", session_id)
    except FileNotFoundError:
        return f"ERROR: Session {session_id} doesn't exist"

    if not frontmatter:
        return "ERROR: Invalid session file format"

    current_workflow_index = int(frontmatter["currentWorkflow"])
    current_step = int(frontmatter["currentStep"])
    queue = frontmatter["queue"]

    # Load current workflow
    try:
        current_workflow = _load_workflow(queue[current_workflow_index])
    except (FileNotFoundError, ValueError) as e:
        return f"ERROR: {e}"

    steps = current_workflow["steps"]

    # Build status message
    result = []
    result.append(f"Queue: [{', '.join(queue)}]")
    result.append(f"Position: {queue[current_workflow_index]} (workflow {current_workflow_index + 1}/{len(queue)}, step {current_step + 1}/{len(steps)})")

    if current_step + 1 < len(steps):
        result.append(f"Next: {steps[current_step + 1]['name']}")
    elif current_workflow_index + 1 < len(queue):
        result.append(f"Next workflow: {queue[current_workflow_index + 1]}")
    else:
        result.append("Next: Queue will complete")

    return "\n".join(result)


def _append_workflow(session_id: str, append: str) -> str:
    """
    Append workflow(s) to session queue.

    Args:
        session_id: Session identifier
        append: Workflow ID or JSON array of workflow IDs

    Returns:
        Success message with updated queue
    """
    if not session_exists("workflow", session_id):
        return f"ERROR: Session {session_id} doesn't exist"

    try:
        frontmatter, content, _ = read_session("workflow", session_id)
    except FileNotFoundError:
        return f"ERROR: Session {session_id} doesn't exist"

    if not frontmatter:
        return "ERROR: Invalid session file format"

    # Parse workflows to append
    to_append = []
    if append.startswith('['):
        try:
            to_append = json.loads(append)
        except json.JSONDecodeError as e:
            return f"ERROR: Invalid JSON array for append: {e}"
    else:
        to_append.append(append)

    # Validate workflows exist
    for wf_id in to_append:
        try:
            _load_workflow(wf_id)
        except (FileNotFoundError, ValueError) as e:
            return f"ERROR: {e}"

    # Update queue
    queue = list(frontmatter["queue"])
    queue.extend(to_append)
    frontmatter["queue"] = queue

    # Update session
    update_session("workflow", session_id, frontmatter, content)

    # Append queue update section
    timestamp = datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M:%S')
    append_content = f"Added workflows to queue: {', '.join(to_append)}\n*{timestamp}*"
    append_to_session("workflow", session_id, "Queue Updated", append_content)

    return f"Added {len(to_append)} workflow(s) to queue. New queue: [{', '.join(queue)}]"


def _continue_workflow(
    session_id: str,
    response: str,
    thoughts: Optional[str],
    status: Optional[str],
    conclusion: Optional[str]
) -> str:
    """
    Continue workflow session with response to current step.

    Args:
        session_id: Session identifier
        response: Response to current step
        thoughts: Optional ambient thoughts
        status: Optional status ("completed" or "cancelled")
        conclusion: Required conclusion when status="completed"

    Returns:
        Next step guidance or completion message
    """
    # Validate concepts
    concept_error = _validate_concepts(response, thoughts)
    if concept_error:
        return concept_error

    if not session_exists("workflow", session_id):
        return f"ERROR: Session {session_id} doesn't exist"

    try:
        frontmatter, existing_content, _ = read_session("workflow", session_id)
    except FileNotFoundError:
        return f"ERROR: Session {session_id} doesn't exist"

    if not frontmatter:
        return "ERROR: Invalid session file format"

    current_workflow_index = int(frontmatter["currentWorkflow"])
    current_step = int(frontmatter["currentStep"])
    queue = frontmatter["queue"]

    # Load current workflow
    try:
        current_workflow = _load_workflow(queue[current_workflow_index])
    except (FileNotFoundError, ValueError) as e:
        return f"ERROR: {e}"

    steps = current_workflow["steps"]

    # Build and record response
    response_lines = [response]
    if thoughts:
        response_lines.append("")
        response_lines.append(f"*Thoughts: {thoughts}*")
    response_lines.append("")
    response_lines.append(f"*{datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M:%S')}*")
    response_content = "\n".join(response_lines)

    heading = f"Step {current_step + 1}/{len(steps)} Response"
    append_to_session("workflow", session_id, heading, response_content)

    # Advance step
    current_step += 1
    frontmatter["currentStep"] = current_step

    # Re-read content after append
    _, updated_content, _ = read_session("workflow", session_id)
    existing_content = updated_content

    # Handle explicit completion
    if status == "completed":
        return _handle_completed_status(session_id, frontmatter, current_step, conclusion)

    # Handle cancellation
    if status == "cancelled":
        return _handle_cancelled_status(session_id, frontmatter, existing_content, current_step)

    # Handle workflow/queue progression
    if current_step >= len(steps):
        current_workflow_index += 1
        if current_workflow_index >= len(queue):
            return _handle_queue_exhaustion(session_id, frontmatter, current_step, conclusion)

        return _transition_to_next_workflow(session_id, frontmatter, existing_content, current_workflow_index, queue)

    return _advance_to_next_step(session_id, frontmatter, existing_content, current_step, steps)


def _handle_completed_status(
    session_id: str,
    frontmatter: dict,
    current_step: int,
    conclusion: Optional[str]
) -> str:
    """Handle workflow completion status."""
    if not conclusion:
        return "ERROR: Conclusion required when completing session. Must synthesize findings with [[concepts]]."

    conclusion_concepts = extract_wiki_links(conclusion)
    if not conclusion_concepts:
        return "ERROR: Conclusion must include [[concepts]] for knowledge graph integration."

    append_to_session("workflow", session_id, "Conclusion", conclusion)

    frontmatter["status"] = "completed"
    frontmatter["completed"] = datetime.now(timezone.utc).isoformat()
    frontmatter["stepCount"] = current_step

    _, content_after_conclusion, _ = read_session("workflow", session_id)
    update_session("workflow", session_id, frontmatter, content_after_conclusion)

    return "✅ Workflow session completed"


def _handle_cancelled_status(
    session_id: str,
    frontmatter: dict,
    existing_content: str,
    current_step: int
) -> str:
    """Handle workflow cancellation status."""
    frontmatter["status"] = "cancelled"
    frontmatter["cancelled"] = datetime.now(timezone.utc).isoformat()
    frontmatter["stepCount"] = current_step
    update_session("workflow", session_id, frontmatter, existing_content)

    return "❌ Workflow session cancelled"


def _handle_queue_exhaustion(
    session_id: str,
    frontmatter: dict,
    current_step: int,
    conclusion: Optional[str]
) -> str:
    """Handle queue completion."""
    if not conclusion:
        return "ERROR: All workflows complete. Call workflow tool again with conclusion parameter to finalize."

    conclusion_concepts = extract_wiki_links(conclusion)
    if not conclusion_concepts:
        return "ERROR: Conclusion must include [[concepts]] for knowledge graph integration."

    append_to_session("workflow", session_id, "Conclusion", conclusion)

    frontmatter["status"] = "completed"
    frontmatter["completed"] = datetime.now(timezone.utc).isoformat()
    frontmatter["stepCount"] = current_step

    _, content_after_conclusion, _ = read_session("workflow", session_id)
    update_session("workflow", session_id, frontmatter, content_after_conclusion)

    return "✅ All workflows in queue completed"


def _transition_to_next_workflow(
    session_id: str,
    frontmatter: dict,
    existing_content: str,
    current_workflow_index: int,
    queue: list
) -> str:
    """Transition to next workflow in queue."""
    current_step = 0
    frontmatter["currentWorkflow"] = current_workflow_index
    frontmatter["currentStep"] = current_step
    update_session("workflow", session_id, frontmatter, existing_content)

    try:
        next_workflow = _load_workflow(queue[current_workflow_index])
    except (FileNotFoundError, ValueError) as e:
        return f"ERROR: {e}"

    next_steps = next_workflow["steps"]

    step_json = json.dumps(next_steps[0], indent=2)
    append_to_session("workflow", session_id, f"Step 1/{len(next_steps)}", step_json)

    return f"Step 1/{len(next_steps)}\n\n{step_json}"


def _advance_to_next_step(
    session_id: str,
    frontmatter: dict,
    existing_content: str,
    current_step: int,
    steps: list
) -> str:
    """Advance to next step in current workflow."""
    update_session("workflow", session_id, frontmatter, existing_content)

    next_step = steps[current_step]
    step_json = json.dumps(next_step, indent=2)
    append_to_session("workflow", session_id, f"Step {current_step + 1}/{len(steps)}", step_json)

    return f"Step {current_step + 1}/{len(steps)}\n\n{step_json}"


# ===== Main Workflow Function =====


def workflow(
    session_id: Optional[str] = None,
    workflow_id: Optional[str] = None,
    response: Optional[str] = None,
    thoughts: Optional[str] = None,
    status: Optional[str] = None,
    conclusion: Optional[str] = None,
    view: bool = False,
    append: Optional[str] = None,
    learn: bool = False
) -> str:
    """
    Execute workflow orchestration for systematic problem-solving.

    Operations:
    - Start new workflow: workflow_id only
    - Continue session: session_id + response
    - View queue: session_id + view=True
    - Append to queue: session_id + append

    Args:
        session_id: Continue existing session (optional)
        workflow_id: Start new workflow with ID or JSON array of IDs (optional)
        response: Response to current step (required for continue)
        thoughts: Optional ambient thoughts
        status: Optional status ("completed" or "cancelled")
        conclusion: Required conclusion when status="completed"
        view: View queue status (default: False)
        append: Append workflow(s) to queue (optional)
        learn: Return help documentation (default: False)

    Returns:
        Result message with next step or status

    Examples:
        >>> workflow(workflow_id="deductive-reasoning")
        "Created session: workflow-1768421493459\\n\\nStep 1/5..."

        >>> workflow(session_id="workflow-1768421493459", response="Analyzed [[requirements]]")
        "Step 2/5\\n\\n{...}"

        >>> workflow(session_id="workflow-1768421493459", view=True)
        "Queue: [deductive-reasoning]\\nPosition: deductive-reasoning (workflow 1/1, step 2/5)..."
    """
    if learn:
        # Return help documentation
        help_path = os.path.join(Config.assets_path, "usage", "tools", "workflow.md")
        if os.path.exists(help_path):
            with open(help_path, 'r', encoding='utf-8') as f:
                return f.read()
        return "Help documentation not available."

    # Validate parameters
    if workflow_id and session_id:
        return (
            "ERROR: Cannot provide both workflowId and sessionId together.\n\n"
            f"CURRENT PARAMETERS:\n"
            f"  sessionId: {session_id}\n"
            f"  workflowId: {workflow_id}\n\n"
            "CORRECT USAGE:\n"
            "  To continue existing session: Use sessionId + response\n"
            "  To start new workflow: Use workflowId only\n"
            "  To view queue: Use sessionId + view\n"
            "  To append to queue: Use sessionId + append\n\n"
            "TIP: You don't need to specify the workflow ID when continuing a session - "
            "the session already knows which workflow it's running."
        )

    # Dispatch operations
    if workflow_id and not session_id:
        return _start_workflow(workflow_id)

    if session_id and view:
        return _view_workflow(session_id)

    if session_id and append:
        return _append_workflow(session_id, append)

    if session_id and response:
        return _continue_workflow(session_id, response, thoughts, status, conclusion)

    return (
        "ERROR: Invalid parameter combination.\n\n"
        "VALID OPERATIONS:\n"
        "  Start new workflow: workflowId only\n"
        "  Continue session: sessionId + response\n"
        "  View queue: sessionId + view=true\n"
        "  Append to queue: sessionId + append\n\n"
        "TIP: view=true and append both require a sessionId to identify which session to operate on."
    )
