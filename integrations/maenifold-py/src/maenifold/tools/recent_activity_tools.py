"""
Recent activity tracking for maenifold.

Provides:
- RecentActivity: Time-based activity monitoring for memory files and thinking sessions
"""

import os
import re
import sqlite3
from datetime import datetime, timedelta, timezone
from typing import List, Optional, Tuple

from maenifold.config import Config
from maenifold.utils.sqlite_extensions import open_read_only, query_single


def recent_activity(
    limit: int = 10,
    filter: Optional[str] = None,
    timespan: Optional[str] = None,
    include_content: bool = False,
    learn: bool = False,
) -> str:
    """
    Monitors maenifold system activity with time-based filtering for memory and
    thinking session tracking.

    Select when AI needs to understand recent work patterns, find current sessions,
    or analyze usage trends. Requires optional filters for activity type, time span,
    and result limits for focused monitoring.

    Integrates with ReadMemory for session access, SearchMemories for activity
    analysis, MemoryStatus for system health. Returns chronological activity list
    with timestamps, types, and identifiers for session continuation or analysis.

    Args:
        limit: Max results (default 10)
        filter: Filter type: "thinking", "memory", "chat", "workflow", or "all" (default "all")
        timespan: Time span string (e.g., "24:00:00" for 24 hours). Must be positive.
        include_content: Include full section content (true) or headers only (false, default)
        learn: Return help documentation instead of executing

    Returns:
        Formatted activity report with timestamps, types, and session status
    """
    if learn:
        return _get_learn_content("RecentActivity")

    try:
        # Parse and validate timespan
        timespan_delta = None
        if timespan:
            try:
                timespan_delta = _parse_timespan(timespan)
                if timespan_delta.total_seconds() < 0:
                    return "ERROR: timespan parameter must be positive. Example: '24:00:00' for 24 hours."
            except ValueError as e:
                return f"ERROR: Invalid timespan format. {e}"

        # Collect activity
        results = _collect_recent_activity(timespan_delta, filter, limit)

        # Format output
        return _format_activity_report(results, include_content)

    except Exception as e:
        if "No database found" in str(e) or "no such table" in str(e):
            return "# Recent Activity\n\nNo database found. Run the `Sync` command first to index memory files."
        return f"ERROR: {str(e)}"


def _parse_timespan(timespan_str: str) -> timedelta:
    """
    Parse timespan string to timedelta.

    Format: "[days.]hours:minutes:seconds"
    Examples:
        "01:30:00" -> 1.5 hours
        "24.00:00:00" -> 1 day
        "7.00:00:00" -> 7 days

    Args:
        timespan_str: Timespan string

    Returns:
        timedelta object

    Raises:
        ValueError: If format is invalid
    """
    # Handle [days.]hours:minutes:seconds format
    parts = timespan_str.split(".")
    days = 0
    time_part = timespan_str

    if len(parts) == 2:
        try:
            days = int(parts[0])
            time_part = parts[1]
        except ValueError:
            raise ValueError(
                "Invalid timespan format. Expected: '[days.]hours:minutes:seconds' (e.g., '24:00:00' or '7.00:00:00')"
            )

    # Parse hours:minutes:seconds
    time_components = time_part.split(":")
    if len(time_components) != 3:
        raise ValueError(
            "Invalid timespan format. Expected: '[days.]hours:minutes:seconds' (e.g., '24:00:00' or '7.00:00:00')"
        )

    try:
        hours = int(time_components[0])
        minutes = int(time_components[1])
        seconds = int(time_components[2])
    except ValueError:
        raise ValueError(
            "Invalid timespan format. Expected: '[days.]hours:minutes:seconds' (e.g., '24:00:00' or '7.00:00:00')"
        )

    return timedelta(days=days, hours=hours, minutes=minutes, seconds=seconds)


def _collect_recent_activity(
    timespan: Optional[timedelta], filter_type: Optional[str], limit: int
) -> List[Tuple[str, str, str, str, str]]:
    """
    Collect recent activity from database.

    Args:
        timespan: Optional time span filter
        filter_type: Optional filter: "thinking", "memory", "chat", "workflow", or "all"
        limit: Maximum results

    Returns:
        List of tuples: (file_path, title, last_indexed, content, status)

    Raises:
        Exception: If database doesn't exist or is invalid
    """
    if not os.path.exists(Config.database_path):
        raise Exception("No database found")

    with open_read_only(Config.database_path) as conn:
        # Check if table exists
        table_exists = (
            query_single(
                conn,
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='file_content'",
            )
            or 0
        ) > 0

        if not table_exists:
            raise Exception("No database found. Run the `Sync` command first to index memory files.")

        # Build query
        sql = """
            SELECT file_path, title, last_indexed, content, status
            FROM file_content
            WHERE 1=1
        """
        parameters = {}

        # Ensure limit is non-negative
        if limit < 0:
            limit = 0

        # Apply filter
        if filter_type and filter_type != "all":
            thinking_folder = os.path.basename(Config.thinking_path)

            if filter_type == "thinking":
                sql += " AND file_path LIKE :filter_pattern"
                parameters["filter_pattern"] = f"memory://{thinking_folder}/%"
            elif filter_type == "chat":
                sql += " AND file_path LIKE :filter_pattern"
                parameters["filter_pattern"] = "memory://chat/%"
            elif filter_type == "memory":
                sql += " AND file_path NOT LIKE :filter_pattern AND file_path NOT LIKE :chat_pattern"
                parameters["filter_pattern"] = f"memory://{thinking_folder}/%"
                parameters["chat_pattern"] = "memory://chat/%"

        # Apply timespan filter
        if timespan:
            cutoff_datetime = datetime.now(timezone.utc) - timespan
            cutoff_str = cutoff_datetime.strftime("%Y-%m-%d %H:%M:%S")
            sql += " AND datetime(last_indexed) >= datetime(:cutoff)"
            parameters["cutoff"] = cutoff_str

        # Order and limit
        sql += " ORDER BY last_indexed DESC LIMIT :limit"
        parameters["limit"] = limit

        # Execute query
        cursor = conn.cursor()
        cursor.execute(sql, parameters)
        results = []

        for row in cursor.fetchall():
            file_path = row[0]
            title = row[1] if row[1] else ""
            last_indexed = row[2]
            content = row[3]
            status = row[4] if row[4] else "active"
            results.append((file_path, title, last_indexed, content, status))

        return results


def _extract_snippet(content: str, max_length: int = -1) -> str:
    """
    Extract snippet from content, excluding frontmatter lines.

    Args:
        content: Full content
        max_length: Maximum snippet length (-1 for default from config)

    Returns:
        Snippet string
    """
    if max_length == -1:
        max_length = Config.recent_activity_snippet_length

    # Split into lines
    lines = content.split("\n")

    # Filter out frontmatter lines (lines starting with *)
    content_lines = [line for line in lines if line and not line.lstrip().startswith("*")]

    # If no content lines remain, use all non-empty lines
    if not content_lines:
        content_lines = [line for line in lines if line]

    # Join with spaces
    snippet = " ".join(content_lines).strip()

    # Truncate if needed
    if len(snippet) > max_length:
        snippet = snippet[: max_length - 3] + "..."

    return snippet


def _extract_session_id(file_path: str) -> str:
    """
    Extract session ID from file path.

    Args:
        file_path: memory:// URI

    Returns:
        Session ID or filename without extension
    """
    thinking_folder = os.path.basename(Config.thinking_path)
    path = file_path.replace("memory://", "").replace(f"{thinking_folder}/", "")
    segments = path.split("/")

    if len(segments) > 1:
        return segments[1]

    # Return filename without extension
    name = segments[0]
    if name.endswith(".md"):
        name = name[:-3]
    return name


def _determine_file_type(file_path: str) -> str:
    """
    Determine file type from path.

    Args:
        file_path: memory:// URI

    Returns:
        File type: "sequential", "workflow", "thinking", or "memory"
    """
    thinking_folder = os.path.basename(Config.thinking_path)
    sequential_folder = os.path.basename(Config.sequential_path)

    if f"memory://{thinking_folder}/" in file_path:
        if f"/{sequential_folder}/" in file_path:
            return "sequential"
        elif "/workflow/" in file_path:
            return "workflow"
        else:
            return "thinking"
    return "memory"


def _extract_h2_sections(markdown: str) -> List[Tuple[str, str]]:
    """
    Extract H2 sections from markdown content.

    Args:
        markdown: Markdown content

    Returns:
        List of tuples: (heading_text, section_content)
    """
    sections = []
    lines = markdown.split("\n")
    current_heading = None
    current_content = []

    for line in lines:
        # Check if line is H2 heading
        if line.startswith("## "):
            # Save previous section if exists
            if current_heading is not None:
                sections.append((current_heading, "\n".join(current_content).strip()))

            # Start new section
            current_heading = line[3:].strip()
            current_content = []
        elif current_heading is not None:
            # Accumulate content for current section
            current_content.append(line)

    # Save final section
    if current_heading is not None:
        sections.append((current_heading, "\n".join(current_content).strip()))

    return sections


def _format_activity_report(
    results: List[Tuple[str, str, str, str, str]], include_content: bool
) -> str:
    """
    Format activity report.

    Args:
        results: List of tuples: (file_path, title, last_indexed, content, status)
        include_content: Include full content or snippets only

    Returns:
        Formatted report string
    """
    if not results:
        return "No recent activity found"

    output = ["# Recent Activity", ""]

    for file_path, title, last_indexed, content, status in results:
        session_id = _extract_session_id(file_path)
        file_type = _determine_file_type(file_path)

        # Parse timestamp
        try:
            modified = datetime.fromisoformat(last_indexed.replace(" ", "T"))
        except ValueError:
            modified = datetime.now(timezone.utc)

        # Extract sections
        sections = _extract_h2_sections(content)

        # Format based on type
        if file_type in ("sequential", "workflow"):
            _format_thinking_file(
                output, session_id, file_type, modified, sections, content, status, include_content
            )
        else:
            _format_memory_file(output, session_id, file_type, modified, sections, title, include_content)

        output.append("")

    return "\n".join(output)


def _format_thinking_file(
    output: List[str],
    session_id: str,
    file_type: str,
    modified: datetime,
    sections: List[Tuple[str, str]],
    full_content: str,
    status: str,
    include_content: bool,
) -> None:
    """
    Format thinking file (sequential or workflow).

    Args:
        output: Output list to append to
        session_id: Session identifier
        file_type: "sequential" or "workflow"
        modified: Last modified timestamp
        sections: List of H2 sections
        full_content: Full file content
        status: Session status
        include_content: Include full content or snippets
    """
    # Check if sequential (has "Thought" sections)
    is_sequential = file_type == "sequential" or any(h.startswith("Thought") for h, _ in sections)

    # Filter out completion/cancellation sections
    meaningful_sections = [
        (h, c) for h, c in sections if h not in ("Completion", "Cancellation")
    ]

    if is_sequential:
        # Count thought sections
        thought_count = sum(1 for h, _ in sections if h.startswith("Thought"))

        output.append(f"**{session_id}** (sequential)")
        output.append(f"  Modified: {modified.strftime('%Y-%m-%d %H:%M')}")
        output.append(f"  Thoughts: {thought_count}")
        output.append(f"  Status: {status}")

        # Show first section
        if meaningful_sections:
            if include_content:
                output.append(f"  First H2: {meaningful_sections[0][0]}")
                output.append(f"  First H2 Content: {meaningful_sections[0][1]}")
            else:
                first_snippet = _extract_snippet(meaningful_sections[0][1])
                output.append(f'  First: "{first_snippet}"')

        # Show last section if different
        if len(meaningful_sections) > 1:
            if include_content:
                output.append(f"  Last H2: {meaningful_sections[-1][0]}")
                output.append(f"  Last H2 Content: {meaningful_sections[-1][1]}")
            else:
                last_snippet = _extract_snippet(meaningful_sections[-1][1])
                first_snippet = _extract_snippet(meaningful_sections[0][1])
                if last_snippet != first_snippet:
                    output.append(f'  Last: "{last_snippet}"')

    else:
        # Workflow
        step_count = sum(1 for h, _ in sections if "Step" in h or "Response" in h)

        output.append(f"**{session_id}** (workflow)")
        output.append(f"  Modified: {modified.strftime('%Y-%m-%d %H:%M')}")
        output.append(f"  Steps: {step_count}")
        output.append(f"  Status: {status}")

        # Show first section
        if meaningful_sections:
            if include_content:
                output.append(f"  First H2: {meaningful_sections[0][0]}")
                output.append(f"  First H2 Content: {meaningful_sections[0][1]}")
            else:
                first_snippet = _extract_snippet(meaningful_sections[0][1])
                output.append(f'  First: "{first_snippet}"')

        # Show current section if different
        if len(meaningful_sections) > 1:
            if include_content:
                output.append(f"  Last H2: {meaningful_sections[-1][0]}")
                output.append(f"  Last H2 Content: {meaningful_sections[-1][1]}")
            else:
                current_snippet = _extract_snippet(meaningful_sections[-1][1])
                first_snippet = _extract_snippet(meaningful_sections[0][1])
                if current_snippet != first_snippet:
                    output.append(f'  Current: "{current_snippet}"')


def _format_memory_file(
    output: List[str],
    session_id: str,
    file_type: str,
    modified: datetime,
    sections: List[Tuple[str, str]],
    title: str,
    include_content: bool,
) -> None:
    """
    Format memory file.

    Args:
        output: Output list to append to
        session_id: Session identifier
        file_type: File type
        modified: Last modified timestamp
        sections: List of H2 sections
        title: File title
        include_content: Include full content or snippets
    """
    output.append(f"**{session_id}** (memory)")
    output.append(f"  Modified: {modified.strftime('%Y-%m-%d %H:%M')}")
    output.append(f"  Title: {title}")
    output.append(f"  Sections: {len(sections)}")

    # Show first section
    if sections:
        if include_content:
            output.append(f"  First H2: {sections[0][0]}")
            output.append(f"  First H2 Content: {sections[0][1]}")
        else:
            first_snippet = _extract_snippet(sections[0][1])
            output.append(f'  First: "{first_snippet}"')

    # Show last section if different
    if len(sections) > 1:
        if include_content:
            output.append(f"  Last H2: {sections[-1][0]}")
            output.append(f"  Last H2 Content: {sections[-1][1]}")
        else:
            last_snippet = _extract_snippet(sections[-1][1])
            first_snippet = _extract_snippet(sections[0][1])
            if last_snippet != first_snippet:
                output.append(f'  Last: "{last_snippet}"')


def _get_learn_content(tool_name: str) -> str:
    """
    Load learning/help content for a tool.

    Args:
        tool_name: Name of the tool

    Returns:
        Help content or placeholder message
    """
    help_path = os.path.join(
        Config.assets_path, "usage", "tools", f"{tool_name.lower()}.md"
    )

    if not os.path.exists(help_path):
        return f"No help file found for tool: {tool_name}"

    with open(help_path, "r", encoding="utf-8") as f:
        return f.read()
