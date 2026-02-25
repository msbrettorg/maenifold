"""Graph traversal and visualization tools for concept relationship exploration."""

import json
import re
from typing import Dict, List, Optional

from maenifold.config import Config
from maenifold.utils.markdown_io import normalize_concept, read_markdown, uri_to_path
from maenifold.utils.sqlite_extensions import open_read_only, query_single, query
from maenifold.tools.graph_database import initialize_database


class RelatedConcept:
    """A concept related to the query concept."""

    def __init__(self, name: str, co_occurrence_count: int, files: List[str]):
        self.name = name
        self.co_occurrence_count = co_occurrence_count
        self.files = files
        self.content_preview: Dict[str, str] = {}

    def to_dict(self) -> dict:
        """Convert to dictionary for JSON serialization."""
        result = {
            "name": self.name,
            "coOccurrenceCount": self.co_occurrence_count,
            "files": self.files
        }
        if self.content_preview:
            result["contentPreview"] = self.content_preview
        return result


class BuildContextResult:
    """Result of building context around a concept."""

    def __init__(self, concept_name: str, depth: int):
        self.concept_name = concept_name
        self.depth = depth
        self.direct_relations: List[RelatedConcept] = []
        self.expanded_relations: List[str] = []

    def to_dict(self) -> dict:
        """Convert to dictionary for JSON serialization."""
        return {
            "conceptName": self.concept_name,
            "depth": self.depth,
            "directRelations": [r.to_dict() for r in self.direct_relations],
            "expandedRelations": self.expanded_relations
        }


def build_context(
    concept_name: str,
    depth: int = 2,
    max_entities: int = 20,
    include_content: bool = False
) -> dict:
    """
    Traverses concept relationships to discover connected knowledge networks.

    Args:
        concept_name: CONCEPT name to build context around (NOT a file)
        depth: How many hops in the concept graph (must be >= 0)
        max_entities: Max entities to return
        include_content: Include content previews for files

    Returns:
        Dictionary with concept_name, depth, direct_relations, expanded_relations
    """
    # ISSUE-005: Validate depth parameter
    if depth < 0:
        raise ValueError("depth must be >= 0")

    # Normalize concept name
    concept_name = normalize_concept(concept_name)

    # Initialize result
    result = BuildContextResult(concept_name, depth)

    # Ensure database exists
    initialize_database()

    # Open read-only connection
    with open_read_only(Config.database_path) as conn:
        # Check if concept exists
        exists = query_single(
            conn,
            "SELECT COUNT(*) > 0 FROM concepts WHERE concept_name = ?",
            (concept_name,)
        )

        if not exists:
            return result.to_dict()

        # Query direct relations
        direct_relations = query(
            conn,
            """
            SELECT
                CASE WHEN concept_a = ? THEN concept_b ELSE concept_a END as related,
                co_occurrence_count as count,
                source_files as files
            FROM concept_graph
            WHERE concept_a = ? OR concept_b = ?
            ORDER BY co_occurrence_count DESC
            LIMIT ?
            """,
            (concept_name, concept_name, concept_name, max_entities)
        )

        # Process direct relations
        for related, count, files_json in direct_relations:
            # Deserialize file list
            file_list = json.loads(files_json)

            # Take top 3 files
            top_files = file_list[:3]

            related_concept = RelatedConcept(related, count, top_files)

            # Add content previews if requested
            if include_content:
                for file_uri in top_files:
                    try:
                        full_path = uri_to_path(file_uri, Config.memory_path)
                        _, content, _ = read_markdown(full_path)

                        # GRAPH-001: Extract section containing concept mention
                        preview = extract_section_with_concept(
                            content, concept_name, max_length=500
                        )

                        related_concept.content_preview[file_uri] = preview
                    except Exception:
                        # Silently skip files that can't be read
                        pass

            result.direct_relations.append(related_concept)

        # Expand to deeper relations if depth > 1
        if depth > 1:
            expanded_concepts = set()
            visited = {concept_name}

            def traverse_recursive(current_concept: str, current_depth: int):
                """Recursively traverse the concept graph."""
                if current_depth >= depth:
                    return

                # Get next level of concepts
                next_level = query(
                    conn,
                    """
                    SELECT DISTINCT
                        CASE WHEN concept_a = ? THEN concept_b ELSE concept_a END
                    FROM concept_graph
                    WHERE concept_a = ? OR concept_b = ?
                    """,
                    (current_concept, current_concept, current_concept)
                )

                # Note: query() returns list of scalars for single-column queries
                for next_concept in next_level:
                    if next_concept not in visited:
                        visited.add(next_concept)
                        expanded_concepts.add(next_concept)
                        traverse_recursive(next_concept, current_depth + 1)

            # Start traversal from top 5 direct relations
            for related_concept in result.direct_relations[:5]:
                traverse_recursive(related_concept.name, 1)

            # Limit expanded relations
            result.expanded_relations = list(expanded_concepts)[:max_entities]

    return result.to_dict()


def visualize(
    concept_name: str,
    depth: int = 2,
    max_nodes: int = 30,
    learn: bool = False
) -> str:
    """
    Generates Mermaid diagram representation of concept relationships.

    Args:
        concept_name: Central concept to visualize
        depth: Graph depth (1-5)
        max_nodes: Maximum nodes to include (5-100)
        learn: Return help documentation instead of executing

    Returns:
        Mermaid diagram code as string
    """
    if learn:
        return """# Visualize

Generates Mermaid diagram representations of concept relationships from maenifold's graph database.

## Parameters
- concept_name: Central concept to visualize
- depth: Graph depth (1-5, default: 2)
- max_nodes: Maximum nodes to include (5-100, default: 30)

## Returns
Mermaid diagram code ready for rendering.

## Example
visualize("machine-learning", depth=2, max_nodes=30)
"""

    # Validate parameters
    if not concept_name or not concept_name.strip():
        return "ERROR: conceptName is required"

    if depth < 1 or depth > 5:
        return "ERROR: depth must be between 1 and 5"

    if max_nodes < 5 or max_nodes > 100:
        return "ERROR: maxNodes must be between 5 and 100"

    # Normalize concept name
    concept_name = normalize_concept(concept_name)

    # Ensure database exists
    initialize_database()

    # Open read-only connection
    with open_read_only(Config.database_path) as conn:
        # Check if concept exists
        exists = query_single(
            conn,
            "SELECT COUNT(*) > 0 FROM concepts WHERE concept_name = ?",
            (concept_name,)
        )

        if not exists:
            return f"CONCEPT '{concept_name}' not found. Run sync first."

        # Query relationships using recursive CTE
        relations = query(
            conn,
            """
            WITH RECURSIVE search(depth, from_concept, to_concept, count, path) AS (
                -- Seed: direct neighbors of the root concept
                SELECT
                    1 as depth,
                    ? as from_concept,
                    CASE WHEN concept_a = ? THEN concept_b ELSE concept_a END as to_concept,
                    co_occurrence_count as count,
                    '|' || ? || '|' || (CASE WHEN concept_a = ? THEN concept_b ELSE concept_a END) || '|' as path
                FROM concept_graph
                WHERE concept_a = ? OR concept_b = ?
                UNION ALL
                -- Recursive step: expand from the last 'to_concept' to its neighbors
                SELECT
                    s.depth + 1,
                    s.to_concept as from_concept,
                    CASE WHEN g.concept_a = s.to_concept THEN g.concept_b ELSE g.concept_a END as to_concept,
                    g.co_occurrence_count as count,
                    s.path || (CASE WHEN g.concept_a = s.to_concept THEN g.concept_b ELSE g.concept_a END) || '|' as path
                FROM search s
                JOIN concept_graph g ON (g.concept_a = s.to_concept OR g.concept_b = s.to_concept)
                WHERE s.depth < ?
                  AND instr(s.path, '|' || (CASE WHEN g.concept_a = s.to_concept THEN g.concept_b ELSE g.concept_a END) || '|') = 0
            )
            SELECT DISTINCT from_concept, to_concept, count
            FROM search
            ORDER BY count DESC
            LIMIT ?
            """,
            (concept_name, concept_name, concept_name, concept_name,
             concept_name, concept_name, depth, max_nodes)
        )

        # Generate Mermaid diagram
        lines = ["graph TD"]

        if not relations:
            # No connections found
            node_id = concept_name.replace(" ", "_").replace("-", "_")
            lines.append(f"    {node_id} [No connections found]")
        else:
            # Add edges with weights
            for from_concept, to_concept, count in relations:
                node_a = from_concept.replace(" ", "_").replace("-", "_")
                node_b = to_concept.replace(" ", "_").replace("-", "_")
                lines.append(f"    {node_a} -->|{count}| {node_b}")

        return "\n".join(lines) + "\n"


# Helper functions for content preview


def extract_section_with_concept(
    content: str,
    concept_name: str,
    max_length: int = 500
) -> str:
    """
    Extracts the H2 section (or parent section) containing a concept mention.
    Falls back to create_smart_preview if concept not found in any section.

    Args:
        content: Full markdown content to search
        concept_name: Concept to find (normalized form expected)
        max_length: Maximum length of returned section

    Returns:
        Section containing concept or smart preview fallback
    """
    if not content or not content.strip():
        return ""

    # Normalize concept for matching (remove brackets if present)
    normalized_concept = concept_name.strip("[]")

    # Search patterns: [[concept]] WikiLink or plain text mention
    wiki_link_pattern = f"[[{normalized_concept}]]"

    # Find all section boundaries (## headers)
    sections = []
    lines = content.split("\n")
    current_pos = 0
    h1_start = None
    h1_header = None

    for line in lines:
        # Track H1 position for fallback
        if line.startswith("# ") and h1_start is None:
            h1_start = current_pos
            h1_header = line

        # Track H2 sections
        if line.startswith("## "):
            sections.append((current_pos, -1, line))

        current_pos += len(line) + 1  # +1 for newline

    # Set end positions for each section
    for i in range(len(sections)):
        start, _, header = sections[i]
        end = sections[i + 1][0] if i < len(sections) - 1 else len(content)
        sections[i] = (start, end, header)

    # If no H2 sections found, use H1 section if available
    if not sections and h1_start is not None:
        sections.append((h1_start, len(content), h1_header))

    # Search for concept in each section (case-insensitive)
    for start, end, header in sections:
        section_content = content[start:end]

        # Check for WikiLink first (more specific)
        if wiki_link_pattern.lower() in section_content.lower():
            return truncate_section(section_content, max_length)

        # Check for plain text mention
        if normalized_concept.lower() in section_content.lower():
            return truncate_section(section_content, max_length)

    # Fallback: concept not found in any section, use smart preview from start
    return create_smart_preview(content, target_length=200, tolerance=50)


def truncate_section(section: str, max_length: int) -> str:
    """Truncate section to max_length using smart preview."""
    if len(section) <= max_length:
        return section.rstrip()

    # Use smart truncation to preserve sentence boundaries
    return create_smart_preview(section, target_length=max_length - 50, tolerance=50)


def create_smart_preview(
    content: str,
    target_length: int = 200,
    tolerance: int = 50
) -> str:
    """
    Creates a preview of content with sentence-aware truncation.

    Args:
        content: Content to preview
        target_length: Target length for preview
        tolerance: Tolerance for finding good break points

    Returns:
        Truncated content with proper boundaries
    """
    if len(content) <= target_length:
        return content

    max_length = target_length + tolerance
    search_space = content[:min(max_length, len(content))]

    # Try sentence boundary (. ! ?) followed by space or newline
    sentence_end = find_last_sentence_boundary(search_space)
    if sentence_end >= target_length - tolerance:
        return content[:sentence_end + 1].rstrip()

    # Fallback to paragraph boundary (double newline)
    paragraph_end = search_space.rfind("\n\n")
    if paragraph_end >= target_length - tolerance:
        return content[:paragraph_end].rstrip() + "..."

    # Fallback to word boundary (space)
    word_end = search_space.rfind(" ")
    if word_end > 0:
        return content[:word_end].rstrip() + "..."

    # Final fallback - hard truncate
    return content[:target_length] + "..."


def find_last_sentence_boundary(text: str) -> int:
    """
    Find the last sentence boundary in text.

    Args:
        text: Text to search

    Returns:
        Index of last sentence boundary, or -1 if not found
    """
    # Find last occurrence of . ! ? followed by space/newline/end
    matches = [". ", ".\n", "! ", "!\n", "? ", "?\n"]
    last_index = -1

    for match in matches:
        index = text.rfind(match)
        if index > last_index:
            last_index = index

    # Also check for sentence-ending punctuation at very end
    if text.endswith((".", "!", "?")):
        last_index = max(last_index, len(text) - 1)

    return last_index
