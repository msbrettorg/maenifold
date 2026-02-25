"""
Memory tools for reading, writing, editing, and managing knowledge files.

This module implements the core memory operations for maenifold's knowledge graph:
- WriteMemory: Create new files with [[WikiLink]] concepts
- ReadMemory: Retrieve files by URI or title
- EditMemory: Modify existing files with checksum safety
- DeleteMemory: Remove files with confirmation
- MoveMemory: Relocate/rename files
- ExtractConceptsFromFile: Analyze [[WikiLink]] concepts in files

All operations enforce security validation and concept requirements.
"""

import os
import re
import urllib.parse
from pathlib import Path
from typing import Optional

from maenifold.config import Config
from maenifold.utils.markdown_io import (
    extract_wiki_links,
    generate_checksum,
    path_to_uri,
    read_markdown,
    slugify,
    uri_to_path,
    update_markdown,
    write_markdown,
)
from maenifold.utils.time_utils import get_utc_now_iso


def _sanitize_user_input(input_str: str) -> str:
    """
    Sanitize user input for safe use in filenames and paths.

    Security rules:
    - SEC-002: URL decode first to prevent encoded path traversal bypass
    - Strip dangerous characters and shell metacharacters
    - Remove path traversal components (.., /, \\)
    - Remove control characters
    - Limit length to 255 characters
    - Normalize whitespace

    Args:
        input_str: Raw user input

    Returns:
        Sanitized string safe for filesystem operations
    """
    if not input_str or not input_str.strip():
        return "Untitled"

    # Limit length early
    if len(input_str) > 255:
        input_str = input_str[:255]

    # SEC-002: URL-decode input first to prevent encoded path traversal bypass
    # This handles %2e%2e%2f (../) and other encoded variants
    try:
        input_str = urllib.parse.unquote(input_str)
    except Exception:
        # If URL decoding fails, continue with original input
        pass

    # Remove HTML tags
    input_str = re.sub(r"<[^>]*>", "", input_str, flags=re.IGNORECASE)

    # Remove dangerous characters for filenames
    input_str = re.sub(r"[:|>\-\[\]{}#&*!@`']", "", input_str)

    # Remove shell metacharacters
    dangerous_chars = ['$', '(', ')', ';', '&', '|', '`', '\\', '<', '>', '"']
    for ch in dangerous_chars:
        input_str = input_str.replace(ch, "")

    # Remove path traversal components
    input_str = input_str.replace("..", "").replace("/", "").replace("\\", "")

    # Remove control characters (0x00-0x1F, 0x7F-0x9F)
    input_str = re.sub(r"[\x00-\x1F\x7F-\x9F]", "", input_str)

    # Normalize whitespace
    input_str = re.sub(r"\s+", " ", input_str).strip()

    # Final check
    if not input_str or not input_str.strip():
        return "Untitled"

    return input_str


def _validate_path_security(folder_path: str) -> Optional[str]:
    """
    Validate folder path security constraints.

    Security rules:
    - No absolute paths allowed
    - No path traversal components ('.' or '..')
    - Must be relative paths only

    Args:
        folder_path: Folder path to validate

    Returns:
        Error message if validation fails, None if valid
    """
    # RT-SEC-004-BYPASS: URL-decode input first to prevent encoded path traversal bypass
    # This handles %2e%2e%2f (../) and other encoded variants like %252e (double-encoded)
    # Decode recursively until the string no longer changes to handle multiple encoding layers
    previous = None
    current = folder_path
    max_iterations = 5  # Prevent infinite loops from malformed input
    iteration = 0

    while previous != current and iteration < max_iterations:
        previous = current
        try:
            current = urllib.parse.unquote(current)
        except Exception:
            # If URL decoding fails, stop and use current value
            break
        iteration += 1

    folder_path = current

    # RT-SEC-004-NULLBYTE: Block null byte injection attacks
    if '\x00' in folder_path or '%00' in folder_path:
        return "Null bytes are not allowed in paths."

    # RT-SEC-004-UNICODE: Block overlong UTF-8 encoding and malformed percent sequences
    # After full URL decoding:
    # - Any remaining % indicates incomplete/malformed encoding
    # - Unicode replacement char (U+FFFD) indicates overlong or invalid UTF-8 encoding
    if '%' in folder_path or '\ufffd' in folder_path:
        return "Invalid or overlong URL encoding detected in path."

    # Reject absolute paths (check both Unix and Windows styles)
    if os.path.isabs(folder_path):
        return "Absolute paths are not allowed. Use relative paths only."

    # RT-SEC-004-WINPATH: Check for Windows-style absolute paths (works cross-platform)
    # Matches: C:\, C:/, \\server\share, //server/share
    if (len(folder_path) >= 2 and folder_path[1] == ':') or \
       folder_path.startswith('\\\\') or folder_path.startswith('//'):
        return "Absolute paths are not allowed. Use relative paths only."

    # Normalize separators
    normalized_path = folder_path.replace("\\", "/")

    # Check for path traversal components
    segments = [s for s in normalized_path.split("/") if s]  # Filter empty strings
    for segment in segments:
        # RT-SEC-009 FIX: Block all dot-only sequences (., .., ..., ....)
        if segment and all(c == '.' for c in segment):
            return "Path traversal components (dot sequences) are not allowed."

    return None


def _find_file_by_title(title: str) -> Optional[str]:
    """
    Find memory file by title using multiple lookup strategies.

    Lookup strategies:
    1. Direct path lookup (with and without .md extension)
    2. Slugified title search across all memory files

    Args:
        title: File title or relative path to search for

    Returns:
        Absolute file path if found, None otherwise
    """
    if not title or not title.strip():
        return None

    base_path = Config.memory_path

    # Normalize separators
    normalized = title.replace("\\", "/").strip("/")
    if not normalized:
        return None

    # Strategy 1: Direct path lookup
    relative_path = normalized.replace("/", os.sep)
    direct_path = os.path.join(base_path, relative_path)

    if os.path.isfile(direct_path):
        return direct_path

    # Try with .md extension
    if not os.path.splitext(relative_path)[1]:  # No extension
        md_path = direct_path + ".md"
        if os.path.isfile(md_path):
            return md_path
    elif not direct_path.endswith(".md"):
        # Has extension but not .md
        with_md_extension = os.path.join(
            base_path, os.path.splitext(relative_path)[0] + ".md"
        )
        if os.path.isfile(with_md_extension):
            return with_md_extension

    # Strategy 2: Slugified title search
    slug = slugify(title)
    for root, _, files in os.walk(base_path):
        for file in files:
            if file.endswith(".md"):
                file_slug = os.path.splitext(file)[0]
                if file_slug.lower() == slug.lower():
                    return os.path.join(root, file)

    return None


def write_memory(
    title: str,
    content: str,
    folder: Optional[str] = None,
    tags: Optional[list[str]] = None,
    learn: bool = False,
) -> str:
    """
    Create new knowledge file with [[WikiLink]] concepts.

    Args:
        title: File title
        content: Markdown content with at least one [[concept]]
        folder: Optional folder path for organization
        tags: Optional list of tags for categorization
        learn: Return help documentation instead of executing

    Returns:
        Success message with memory:// URI and checksum
        Error message if validation fails

    Raises:
        OSError: If file system operations fail
    """
    if learn:
        return "Help documentation not yet implemented"

    # Ensure directories exist
    Config.ensure_directories()

    # Sanitize title
    title = _sanitize_user_input(title)

    # Validate content has at least one [[concept]]
    concepts = extract_wiki_links(content)
    if not concepts:
        return (
            "ERROR: Content must contain at least one [[concept]] in double brackets to connect to the knowledge graph.\n"
            "Example: 'Learning about [[Machine Learning]] and [[Data Science]]'\n"
            "This ensures your note is connected to the knowledge graph and not orphaned."
        )

    # Validate folder path security
    if folder:
        validation_error = _validate_path_security(folder)
        if validation_error:
            return f"ERROR: Invalid folder path - {validation_error}"

    # Build file path
    file_name = slugify(title) + ".md"
    folder_path = Config.memory_path if not folder else os.path.join(Config.memory_path, folder)
    file_path = os.path.join(folder_path, file_name)

    # Create YAML frontmatter
    permalink = slugify(title)
    now = get_utc_now_iso()
    frontmatter = {
        "title": title,
        "permalink": permalink,
        "type": "memory",
        "status": "saved",
        "created": now,
        "modified": now,
    }

    if tags:
        frontmatter["tags"] = tags

    # BUG-DUP-H1-001: Prevent duplicate H1 headings
    trimmed_content = content.lstrip()
    has_h1 = trimmed_content.startswith("# ")
    full_content = content if has_h1 else f"# {title}\n\n{content}"

    # Create directory and write file
    os.makedirs(folder_path, exist_ok=True)
    write_markdown(file_path, frontmatter, full_content)

    # Generate URI and checksum
    with open(file_path, "r", encoding="utf-8") as f:
        full_text = f.read()
    checksum = generate_checksum(full_text)
    uri = path_to_uri(file_path, Config.memory_path)

    return f"Created memory FILE: {uri}\nChecksum: {checksum}\n\nHint: Use ReadMemory with this URI to access content"


def read_memory(
    identifier: str,
    include_checksum: bool = True,
    learn: bool = False,
) -> str:
    """
    Retrieve knowledge file by URI or title.

    Args:
        identifier: memory:// URI or file title
        include_checksum: Include checksum in output for safe editing
        learn: Return help documentation instead of executing

    Returns:
        Formatted file content with metadata
        Error message if file not found
    """
    if learn:
        return "Help documentation not yet implemented"

    # Resolve identifier to file path
    if identifier.startswith("memory://"):
        try:
            path = uri_to_path(identifier, Config.memory_path)
        except ValueError as e:
            return f"ERROR: {e}"
    else:
        path = _find_file_by_title(identifier)

    if not path or not os.path.isfile(path):
        return f"ERROR: Memory file not found: {identifier}"

    # Read file
    try:
        frontmatter, content, checksum = read_markdown(path)
    except Exception as e:
        return f"ERROR: Failed to read file: {e}"

    # Extract title
    title = (
        frontmatter.get("title")
        if frontmatter and frontmatter.get("title")
        else os.path.splitext(os.path.basename(path))[0]
    )

    # Build output
    uri = path_to_uri(path, Config.memory_path)
    location = os.path.relpath(path, Config.memory_path)

    lines = [
        f"# {title}",
        f"URI: {uri}",
        f"Location: {location}",
    ]

    # Add timestamps if available
    if frontmatter:
        if "created" in frontmatter:
            lines.append(f"Created: {frontmatter['created']}")
        if "modified" in frontmatter:
            lines.append(f"Modified: {frontmatter['modified']}")

    if include_checksum:
        lines.append(f"Checksum: {checksum}")

    lines.extend(["", "---", "", content])

    return "\n".join(lines)


def edit_memory(
    identifier: str,
    operation: str,
    content: str,
    checksum: Optional[str] = None,
    find_text: Optional[str] = None,
    section_name: Optional[str] = None,
    expected_count: Optional[int] = None,
    learn: bool = False,
) -> str:
    """
    Modify existing knowledge file with checksum safety.

    Operations:
    - append: Add content to end of file
    - prepend: Add content to start of file
    - find_replace: Replace all occurrences of find_text with content
    - replace_section: Replace markdown section content

    Args:
        identifier: memory:// URI or file title
        operation: Operation type (append, prepend, find_replace, replace_section)
        content: New content to add/replace (must contain [[concept]])
        checksum: Optional checksum for validation
        find_text: Text to find (required for find_replace)
        section_name: Section heading name (required for replace_section)
        expected_count: Expected match count (optional validation for find_replace)
        learn: Return help documentation instead of executing

    Returns:
        Success message with new URI and checksum
        Error message if validation fails
    """
    if learn:
        return "Help documentation not yet implemented"

    # Resolve identifier to file path
    if identifier.startswith("memory://"):
        try:
            path = uri_to_path(identifier, Config.memory_path)
        except ValueError as e:
            return f"ERROR: {e}"
    else:
        path = _find_file_by_title(identifier)

    if not path or not os.path.isfile(path):
        return f"ERROR: Memory file not found: {identifier}"

    # Read existing file
    try:
        frontmatter, existing_content, actual_checksum = read_markdown(path)
    except Exception as e:
        return f"ERROR: Failed to read file: {e}"

    # Validate checksum if provided
    if checksum and checksum != actual_checksum:
        return f"ERROR: Checksum mismatch. Expected: {checksum}, Actual: {actual_checksum}"

    # Validate new content has [[concept]]
    content_concepts = extract_wiki_links(content)
    if not content_concepts:
        # Extract existing concepts for suggestions
        existing_concepts = extract_wiki_links(existing_content)

        lines = [
            "ERROR: Edited content must contain at least one [[concept]] in double brackets.",
            "Concepts are the building blocks of the knowledge graph—they create connections between files.",
            "",
        ]

        if existing_concepts:
            lines.append("Concepts already in this file that you could reference:")
            for concept in existing_concepts[:3]:  # Show up to 3
                lines.append(f"  • [[{concept}]]")
            lines.append("")

        lines.extend([
            "Examples of how to add concepts:",
            '  • "This relates to [[machine-learning]] and [[neural-networks]]"',
            '  • "See also: [[data-processing]], [[feature-engineering]]"',
            '  • "The [[algorithm]] uses [[graph-theory]] concepts"',
        ])

        return "\n".join(lines)

    # Perform operation
    operation_lower = operation.lower()

    try:
        if operation_lower == "append":
            new_content = existing_content + "\n\n" + content
        elif operation_lower == "prepend":
            new_content = content + "\n\n" + existing_content
        elif operation_lower == "find_replace":
            if not find_text:
                return "ERROR: find_text parameter required for find_replace operation"
            new_content = _perform_find_replace(existing_content, find_text, content, expected_count)
        elif operation_lower == "replace_section":
            if not section_name:
                return "ERROR: section_name parameter required for replace_section operation"
            new_content = _replace_section(existing_content, section_name, content)
        else:
            return f"ERROR: Unknown operation: {operation}"
    except Exception as e:
        return f"ERROR: Operation failed: {e}"

    # Validate result still has [[concept]]
    result_concepts = extract_wiki_links(new_content)
    if not result_concepts:
        return (
            "ERROR: This edit would remove all [[concepts]] from the file.\n"
            "Every memory file must maintain at least one [[concept]] to stay connected to the knowledge graph.\n"
            "\n"
            "To fix this, ensure your edited content includes at least one [[concept]].\n"
            "Examples:\n"
            '  • Add a reference: "Related to [[my-topic]]"\n'
            '  • Tag the section: "[[process]]: Step-by-step instructions..."\n'
            '  • Link concepts: "Uses [[tool]] with [[feature]]"'
        )

    # Update frontmatter
    if not frontmatter:
        frontmatter = {}
    if "type" not in frontmatter:
        frontmatter["type"] = "memory"
    if "status" not in frontmatter:
        frontmatter["status"] = "saved"
    frontmatter["modified"] = get_utc_now_iso()

    # Write updated file
    try:
        update_markdown(path, frontmatter, new_content, checksum)
    except Exception as e:
        return f"ERROR: Failed to write file: {e}"

    # Generate new checksum
    with open(path, "r", encoding="utf-8") as f:
        full_text = f.read()
    new_checksum = generate_checksum(full_text)
    uri = path_to_uri(path, Config.memory_path)

    return f"Updated memory FILE: {uri}\nNew checksum: {new_checksum}"


def _perform_find_replace(
    content: str, find_text: str, replace_text: str, expected_count: Optional[int]
) -> str:
    """
    Perform simple string replacement with validation.

    WARNING: This creates nested WikiLinks if replacing text INSIDE existing [[brackets]].
    Example: [[machine learning]] with find_text="machine learning" and replace_text="[[ML]]"
    becomes [[[[ML]]]] (nested brackets).

    To replace entire WikiLinks, include brackets in find_text: "[[machine learning]]" → "[[ML]]".

    Args:
        content: Content to search in
        find_text: Text to find
        replace_text: Text to replace with
        expected_count: Optional expected match count for validation

    Returns:
        Modified content

    Raises:
        ValueError: If expected_count doesn't match actual matches
    """
    matches = content.count(find_text)

    if expected_count is not None and matches != expected_count:
        raise ValueError(
            f"Expected {expected_count} matches but found {matches}. Find text: '{find_text}'"
        )

    return content.replace(find_text, replace_text)


def _replace_section(content: str, section_name: str, new_content: str) -> str:
    """
    Replace markdown section content.

    Matches: section heading + all content until next heading or end of file

    Args:
        content: Content to search in
        section_name: Section heading name
        new_content: New section content

    Returns:
        Modified content

    Raises:
        ValueError: If section not found
    """
    # Pattern breakdown:
    # (^|\n) - Start of line (group 1, preserves newline)
    # (#+\s*{section}[^\n]*\n) - Heading with section name (group 2)
    # (.*?) - Section content (group 3, to be replaced)
    # (?=\n#|\Z) - Lookahead: next heading or end of string
    escaped_section = re.escape(section_name)
    pattern = rf"(^|\n)(#+\s*{escaped_section}[^\n]*\n)(.*?)(?=\n#|\Z)"

    match = re.search(pattern, content, re.MULTILINE | re.DOTALL)

    if not match:
        raise ValueError(
            f"Section not found: '{section_name}'. Cannot replace a section that doesn't exist."
        )

    # Replace: preserve leading newline + heading + new content
    replacement = rf"\1\2{new_content}"
    result = re.sub(pattern, replacement, content, flags=re.MULTILINE | re.DOTALL)

    return result


def delete_memory(
    identifier: str,
    confirm: bool = False,
    learn: bool = False,
) -> str:
    """
    Permanently remove knowledge file.

    Args:
        identifier: memory:// URI or file title
        confirm: Must be True to delete (safety mechanism)
        learn: Return help documentation instead of executing

    Returns:
        Success message with deleted URI
        Error message if file not found or confirm not set
    """
    if learn:
        return "Help documentation not yet implemented"

    if not confirm:
        return "ERROR: Must set confirm=true to delete a memory file"

    # Resolve identifier to file path
    if identifier.startswith("memory://"):
        try:
            path = uri_to_path(identifier, Config.memory_path)
        except ValueError as e:
            return f"ERROR: {e}"
    else:
        path = _find_file_by_title(identifier)

    if not path or not os.path.isfile(path):
        return f"ERROR: Memory file not found: {identifier}"

    uri = path_to_uri(path, Config.memory_path)

    try:
        os.remove(path)
    except Exception as e:
        return f"ERROR: Failed to delete file: {e}"

    return f"Deleted memory FILE: {uri}"


def move_memory(
    source: str,
    destination: str,
    learn: bool = False,
) -> str:
    """
    Relocate and rename knowledge file.

    Args:
        source: Source memory:// URI or file title
        destination: Destination file name or path
        learn: Return help documentation instead of executing

    Returns:
        Success message showing old → new URIs
        Error message if validation fails
    """
    if learn:
        return "Help documentation not yet implemented"

    if not destination or not destination.strip():
        return "ERROR: Destination parameter is required and cannot be empty."

    # Resolve source to file path
    if source.startswith("memory://"):
        try:
            source_path = uri_to_path(source, Config.memory_path)
        except ValueError as e:
            return f"ERROR: {e}"
    else:
        source_path = _find_file_by_title(source)

    if not source_path or not os.path.isfile(source_path):
        return f"ERROR: Source memory file not found: {source}"

    # Build destination path
    dest_rel = destination.replace("/", os.sep)

    # Slugify filename component for consistency with WriteMemory
    dest_dir = os.path.dirname(dest_rel) if os.path.dirname(dest_rel) else ""
    dest_file = os.path.basename(dest_rel)
    dest_file_base = os.path.splitext(dest_file)[0]
    dest_file = slugify(dest_file_base)

    if not dest_file.endswith(".md"):
        dest_file += ".md"

    dest_rel = dest_file if not dest_dir else os.path.join(dest_dir, dest_file)
    dest_path = os.path.join(Config.memory_path, dest_rel)

    # Security: validate destination doesn't escape memory root
    base_full_path = os.path.abspath(Config.memory_path).rstrip(os.sep) + os.sep
    dest_full_path = os.path.abspath(dest_path)

    if not dest_full_path.startswith(base_full_path):
        return "ERROR: Destination path escapes memory root"

    # Check if destination already exists
    if os.path.isfile(dest_full_path):
        return f"ERROR: Destination file already exists: {destination}. Cannot overwrite without explicit confirmation."

    # Read source file and update modified timestamp
    try:
        frontmatter, content, _ = read_markdown(source_path)
    except Exception as e:
        return f"ERROR: Failed to read source file: {e}"

    if not frontmatter:
        frontmatter = {}
    frontmatter["modified"] = get_utc_now_iso()

    # Write to destination
    try:
        os.makedirs(os.path.dirname(dest_full_path), exist_ok=True)
        write_markdown(dest_full_path, frontmatter, content)
    except Exception as e:
        return f"ERROR: Failed to write destination file: {e}"

    # Delete source
    try:
        os.remove(source_path)
    except Exception as e:
        # Rollback: delete destination file
        try:
            os.remove(dest_full_path)
        except:
            pass
        return f"ERROR: Failed to delete source file: {e}"

    source_uri = path_to_uri(source_path, Config.memory_path)
    dest_uri = path_to_uri(dest_full_path, Config.memory_path)

    return f"Moved memory FILE: {source_uri} → {dest_uri}"


def extract_concepts_from_file(
    identifier: str,
    learn: bool = False,
) -> str:
    """
    Extract [[WikiLink]] concepts from knowledge file.

    Args:
        identifier: memory:// URI or file title
        learn: Return help documentation instead of executing

    Returns:
        List of concepts with count
        Error message if file not found
    """
    if learn:
        return "Help documentation not yet implemented"

    # Resolve identifier to file path
    if identifier.startswith("memory://"):
        try:
            path = uri_to_path(identifier, Config.memory_path)
        except ValueError as e:
            return f"ERROR: {e}"
    else:
        path = _find_file_by_title(identifier)

    if not path or not os.path.isfile(path):
        return f"ERROR: Memory file not found: {identifier}"

    # Read file and extract concepts
    try:
        _, content, _ = read_markdown(path)
    except Exception as e:
        return f"ERROR: Failed to read file: {e}"

    concepts = extract_wiki_links(content)

    if not concepts:
        return "No [[WikiLink]] concepts found in file"

    concept_lines = [f"  • [[{c}]]" for c in concepts]
    return f"Found {len(concepts)} CONCEPTS in file:\n" + "\n".join(concept_lines)
