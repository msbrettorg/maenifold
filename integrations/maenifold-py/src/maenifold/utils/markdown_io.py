"""
Markdown I/O utilities for reading and writing files with YAML frontmatter and WikiLinks.

This module provides functions for:
- Reading/writing markdown files with YAML frontmatter
- Extracting [[WikiLink]] concepts from content (excluding code blocks)
- Normalizing concept names
- Generating checksums for content integrity
- Converting between file paths and memory:// URIs
"""

import hashlib
import os
import re
from pathlib import Path
from typing import Optional

import yaml


# WikiLink pattern: [[concept]]
_WIKILINK_PATTERN = re.compile(r"\[\[([^\]]+)\]\]")

# Code block patterns (fenced and indented)
_FENCED_CODE_PATTERN = re.compile(r"^```.*?^```", re.MULTILINE | re.DOTALL)
_INLINE_CODE_PATTERN = re.compile(r"`[^`]+`")


def write_markdown(
    file_path: str, frontmatter: Optional[dict], content: str
) -> None:
    """
    Write markdown file with YAML frontmatter.

    Args:
        file_path: Path to write the file
        frontmatter: Optional dictionary of frontmatter metadata
        content: Markdown content

    Note:
        Creates parent directories if they don't exist.
        Strips embedding-related keys from frontmatter (database-only fields).
    """
    parts = []

    if frontmatter:
        # Strip embedding-related keys (database-only)
        sanitized = {
            k: v
            for k, v in frontmatter.items()
            if k not in {"embedding", "embedding_base64", "embedding_model", "embedding_date"}
        }

        if sanitized:
            parts.append("---")
            yaml_str = yaml.dump(sanitized, default_flow_style=False, allow_unicode=True)
            parts.append(yaml_str.rstrip())
            parts.append("---")
            parts.append("")

    parts.append(content)

    # Create parent directories
    parent = os.path.dirname(file_path)
    if parent:
        os.makedirs(parent, exist_ok=True)

    with open(file_path, "w", encoding="utf-8") as f:
        f.write("\n".join(parts))


def read_markdown(file_path: str) -> tuple[Optional[dict], str, str]:
    """
    Read markdown file with YAML frontmatter.

    Args:
        file_path: Path to the file to read

    Returns:
        Tuple of (frontmatter_dict, content_str, checksum_str)
        - frontmatter_dict: Parsed YAML frontmatter or None
        - content_str: Markdown content after frontmatter
        - checksum_str: SHA256 checksum of the full file

    Raises:
        FileNotFoundError: If file doesn't exist
    """
    if not os.path.exists(file_path):
        raise FileNotFoundError(f"File not found: {file_path}")

    with open(file_path, "r", encoding="utf-8") as f:
        full_text = f.read()

    checksum = generate_checksum(full_text)

    # Check for YAML frontmatter
    frontmatter = None
    content = full_text

    if full_text.startswith("---"):
        # Find the closing delimiter
        lines = full_text.split("\n")
        end_idx = None
        for i in range(1, len(lines)):
            if lines[i].strip() == "---":
                end_idx = i
                break

        if end_idx is not None:
            # Extract YAML text
            yaml_text = "\n".join(lines[1:end_idx])

            # Try to parse YAML
            if yaml_text.strip():
                try:
                    frontmatter = yaml.safe_load(yaml_text)
                    if not isinstance(frontmatter, dict):
                        frontmatter = None
                except yaml.YAMLError:
                    # Gracefully handle malformed YAML
                    frontmatter = None

            # Extract content after frontmatter
            content = "\n".join(lines[end_idx + 1 :]).lstrip("\n\r")

    return (frontmatter, content, checksum)


def update_markdown(
    file_path: str,
    frontmatter: Optional[dict],
    content: str,
    expected_checksum: Optional[str] = None,
) -> None:
    """
    Update markdown file with checksum validation.

    Args:
        file_path: Path to the file to update
        frontmatter: Optional dictionary of frontmatter metadata
        content: Markdown content
        expected_checksum: Optional checksum to validate before writing

    Raises:
        ValueError: If checksum doesn't match
    """
    if expected_checksum is not None:
        _, _, actual_checksum = read_markdown(file_path)
        if actual_checksum != expected_checksum:
            raise ValueError(
                "Checksum mismatch. File has been modified by another process."
            )

    write_markdown(file_path, frontmatter, content)


def extract_wiki_links(content: str) -> list[str]:
    """
    Extract all [[WikiLink]] concepts from content, excluding code blocks.

    This function:
    1. Removes fenced code blocks (```...```)
    2. Masks inline code (`...`)
    3. Extracts [[concept]] patterns
    4. Normalizes concepts to lowercase-with-hyphens
    5. Returns unique list

    Args:
        content: Markdown content

    Returns:
        List of unique normalized concept names
    """
    # Remove fenced code blocks
    text = _FENCED_CODE_PATTERN.sub("", content)

    # Mask inline code (replace with spaces to preserve positions)
    def mask_inline_code(match):
        return " " * len(match.group(0))

    text = _INLINE_CODE_PATTERN.sub(mask_inline_code, text)

    # Extract WikiLinks
    concepts = set()
    for match in _WIKILINK_PATTERN.finditer(text):
        concept = match.group(1)
        normalized = normalize_concept(concept)
        concepts.add(normalized)

    return list(concepts)


def normalize_concept(concept: str) -> str:
    """
    Normalize concept name to lowercase-with-hyphens format.

    Normalization rules (NORMALIZE-HYPHEN-COMPRESSION-001):
    - Convert to lowercase
    - Replace spaces, underscores, slashes with hyphens
    - Collapse multiple hyphens into single hyphen
    - Trim leading/trailing hyphens

    Args:
        concept: Raw concept name

    Returns:
        Normalized concept name

    Examples:
        >>> normalize_concept("Machine Learning")
        'machine-learning'
        >>> normalize_concept("foo__bar")
        'foo-bar'
        >>> normalize_concept("--test--")
        'test'
    """
    # Basic replacements
    normalized = (
        concept.strip()
        .lower()
        .replace(" ", "-")
        .replace("_", "-")
        .replace("/", "-")
    )

    # Collapse multiple hyphens
    normalized = re.sub(r"-{2,}", "-", normalized)

    # Trim leading/trailing hyphens
    normalized = normalized.strip("-")

    return normalized


def slugify(title: str) -> str:
    """
    Convert title to URL-safe slug.

    Args:
        title: Title text

    Returns:
        Slugified text (lowercase, alphanumeric + hyphens only)

    Examples:
        >>> slugify("My Document Title!")
        'my-document-title'
        >>> slugify("###")
        'untitled'
    """
    slug = title.lower()

    # Replace spaces/underscores with hyphens
    slug = re.sub(r"[\s_]+", "-", slug)

    # Remove non-alphanumeric characters (except hyphens)
    slug = re.sub(r"[^a-z0-9\-]", "", slug)

    # Collapse multiple hyphens
    slug = re.sub(r"-+", "-", slug)

    # Trim leading/trailing hyphens
    slug = slug.strip("-")

    return slug if slug else "untitled"


def path_to_uri(path: str, base_path: str) -> str:
    """
    Convert file path to memory:// URI.

    Args:
        path: Absolute file path
        base_path: Base directory path

    Returns:
        memory:// URI (relative path with .md extension removed)

    Examples:
        >>> path_to_uri("/base/research/notes.md", "/base")
        'memory://research/notes'
    """
    rel_path = os.path.relpath(path, base_path)

    # Remove .md extension
    if rel_path.endswith(".md"):
        rel_path = rel_path[:-3]

    # Normalize path separators to forward slashes
    rel_path = rel_path.replace(os.sep, "/")

    return f"memory://{rel_path}"


def uri_to_path(uri: str, base_path: str) -> str:
    """
    Convert memory:// URI to file path with security validation.

    Args:
        uri: memory:// URI
        base_path: Base directory path

    Returns:
        Absolute file path with .md extension

    Raises:
        ValueError: If URI attempts path traversal outside base directory

    Examples:
        >>> uri_to_path("memory://research/notes", "/base")
        '/base/research/notes.md'
    """
    # Strip memory:// prefix and normalize separators
    rel_path = uri.replace("memory://", "").replace("/", os.sep)

    # Combine with base path
    path = os.path.join(base_path, rel_path)

    # Add .md extension if missing
    if not path.endswith(".md"):
        path += ".md"

    # Security: validate path doesn't escape base directory
    full_path = os.path.abspath(path)
    base_full = os.path.abspath(base_path).rstrip(os.sep) + os.sep

    if not full_path.startswith(base_full):
        raise ValueError(f"Invalid URI attempts to escape memory directory: {uri}")

    return path


def generate_checksum(content: str) -> str:
    """
    Generate SHA256 checksum of content.

    Args:
        content: Text content

    Returns:
        Base64-encoded SHA256 hash
    """
    import base64

    data = content.encode("utf-8")
    hash_bytes = hashlib.sha256(data).digest()
    return base64.b64encode(hash_bytes).decode("ascii")
