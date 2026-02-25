"""
Concept synchronization for maenifold knowledge graph.

Scans memory files, extracts [[WikiLink]] concepts, and builds:
- Concept index (concepts table)
- Concept-to-file mappings (concept_mentions table)
- Co-occurrence graph (concept_graph table)
- Vector embeddings (vec_concepts, vec_memory_files tables)
"""

import hashlib
import json
import os
import sqlite3
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

from maenifold.config import Config
from maenifold.tools.graph_database import initialize_database
from maenifold.utils.markdown_io import (
    extract_wiki_links,
    generate_checksum,
    path_to_uri,
    read_markdown,
    uri_to_path,
)
from maenifold.utils.sqlite_extensions import execute, load_vector_extension, open_with_wal, query, query_single
from maenifold.utils.vector_tools import VectorTools


def sync(learn: bool = False) -> str:
    """
    Synchronize knowledge graph from all markdown files in memory path.

    Process:
    1. Scan all .md files in memory_path
    2. Extract [[WikiLink]] concepts from each file
    3. Build concepts table: insert/update concept_name, first_seen, occurrence_count
    4. Build concept_mentions table: concept_name → source_file mapping
    5. Build concept_graph table: co-occurrence relationships between concepts
    6. Update file_content table with file metadata and MD5 hash
    7. Generate embeddings for concepts (vec_concepts) and files (vec_memory_files)
    8. Clean up orphaned database entries
    9. Return summary: files processed, concepts found, relations built

    Args:
        learn: If True, return help documentation instead of executing

    Returns:
        Summary string with sync statistics

    Note:
        - Uses single transaction for consistency
        - Skips unchanged files (MD5 hash comparison)
        - Gracefully degrades if sqlite-vec extension unavailable
        - Logs telemetry to stderr
    """
    if learn:
        return _get_help_text()

    total_start = time.time()

    # Initialize database schema
    initialize_database()

    # Open connection with WAL mode
    config = Config()
    conn = sqlite3.connect(Config.database_path)
    # Enable extension loading for sqlite-vec support
    if hasattr(conn, 'enable_load_extension'):
        conn.enable_load_extension(True)
    open_with_wal(conn)

    # Try to load vector extension
    vector_ready = _try_ensure_vector_support(conn)

    # Initialize stats
    stats = {
        "files_processed": 0,
        "concepts_found": 0,
        "relations_created": 0,
        "orphaned_concepts": 0
    }

    # Timing tracking
    extraction_time_ms = 0
    embedding_time_ms = 0

    # Collect unique concepts for telemetry
    unique_concepts = set()
    concept_embeddings_generated = 0
    file_embeddings_generated = 0

    # Get all markdown files
    memory_path = Config.memory_path
    files = []
    for root, _, filenames in os.walk(memory_path):
        for filename in filenames:
            if filename.endswith(".md"):
                files.append(os.path.join(root, filename))

    print(f"[SYNC TELEMETRY] Processing {len(files)} files", file=sys.stderr)

    # Count file_search rows before sync
    file_search_count_before = query_single(conn, "SELECT COUNT(*) FROM file_search") or 0
    print(f"[SYNC TELEMETRY] file_search rows before sync: {file_search_count_before}", file=sys.stderr)

    # Begin transaction
    transaction = None
    try:
        transaction = conn.cursor()
        transaction.execute("BEGIN")

        # Process each file
        for file_path in files:
            try:
                # Read file
                frontmatter, content, _ = read_markdown(file_path)

                # Extract concepts
                extraction_start = time.time()
                concepts = extract_wiki_links(content)
                extraction_time_ms += (time.time() - extraction_start) * 1000

                # Track unique concepts
                unique_concepts.update(concepts)

                # Generate memory URI
                memory_uri = path_to_uri(file_path, memory_path)

                # Get title from frontmatter or filename
                title = _get_title(file_path, frontmatter)

                # Get status from frontmatter
                file_status = frontmatter.get("status") if frontmatter else None

                # Get file timestamp
                timestamp = datetime.fromtimestamp(
                    os.path.getmtime(file_path), tz=timezone.utc
                ).isoformat()

                # Compute file hash
                file_hash = generate_checksum(content)

                # Check if file needs updating (by timestamp or hash)
                existing = query(
                    conn,
                    "SELECT last_indexed, file_md5 FROM file_content WHERE file_path = ?",
                    (memory_uri,)
                )

                if existing and len(existing) > 0:
                    existing_row = existing[0]
                    existing_time, existing_hash = existing_row

                    # Skip if unchanged (timestamp matches or hash matches)
                    if timestamp == existing_time or file_hash == existing_hash:
                        if timestamp != existing_time and file_hash == existing_hash:
                            # Update timestamp only
                            execute(
                                conn,
                                "UPDATE file_content SET last_indexed = ? WHERE file_path = ?",
                                (timestamp, memory_uri)
                            )
                        continue

                # Insert/update file content
                execute(
                    conn,
                    """INSERT OR REPLACE INTO file_content
                       (file_path, title, content, last_indexed, status, file_md5)
                       VALUES (?, ?, ?, ?, ?, ?)""",
                    (memory_uri, title, content, timestamp, file_status, file_hash)
                )

                # Get file creation time
                file_created = datetime.fromtimestamp(
                    os.path.getctime(file_path), tz=timezone.utc
                ).isoformat()

                # Process concepts
                for concept in concepts:
                    # Insert concept (idempotent with first_seen)
                    execute(
                        conn,
                        "INSERT OR IGNORE INTO concepts (concept_name, first_seen) VALUES (?, ?)",
                        (concept, file_created)
                    )

                    # Count occurrences in content
                    count = _count_concept_occurrences(content, concept)

                    # Insert/update mention
                    execute(
                        conn,
                        """INSERT OR REPLACE INTO concept_mentions
                           (concept_name, source_file, mention_count)
                           VALUES (?, ?, ?)""",
                        (concept, memory_uri, count)
                    )

                    stats["concepts_found"] += 1

                # Generate embeddings
                embedding_start = time.time()
                concept_embeddings_generated += _generate_concept_embeddings(
                    conn, concepts, vector_ready
                )
                if _generate_file_embedding(conn, memory_uri, content, vector_ready):
                    file_embeddings_generated += 1
                embedding_time_ms += (time.time() - embedding_start) * 1000

                # Build graph edges
                stats["relations_created"] += _build_graph_edges(conn, concepts, memory_uri)

                stats["files_processed"] += 1

            except Exception as ex:
                # Log error and continue with next file
                print(
                    f"[SYNC ERROR] Failed to process file '{file_path}': {type(ex).__name__}: {ex}",
                    file=sys.stderr
                )
                # Continue processing other files
                continue

        # Clean up deleted files
        file_paths = query(conn, "SELECT file_path FROM file_content")
        deleted_count = 0

        for file_path_uri in file_paths:
            disk_path = uri_to_path(file_path_uri, memory_path)
            if not os.path.exists(disk_path):
                deleted_count += 1

                # Delete mentions
                execute(conn, "DELETE FROM concept_mentions WHERE source_file = ?", (file_path_uri,))

                # Update graph edges
                edges = query(
                    conn,
                    "SELECT concept_a, concept_b, source_files FROM concept_graph WHERE source_files LIKE ?",
                    (f"%{file_path_uri}%",)
                )

                for concept_a, concept_b, source_files_json in edges:
                    file_list = json.loads(source_files_json)
                    if file_path_uri in file_list:
                        file_list.remove(file_path_uri)

                    if len(file_list) == 0:
                        # Delete edge
                        execute(
                            conn,
                            "DELETE FROM concept_graph WHERE concept_a = ? AND concept_b = ?",
                            (concept_a, concept_b)
                        )
                    else:
                        # Update edge
                        execute(
                            conn,
                            """UPDATE concept_graph
                               SET co_occurrence_count = ?, source_files = ?
                               WHERE concept_a = ? AND concept_b = ?""",
                            (len(file_list), json.dumps(file_list), concept_a, concept_b)
                        )

                # Delete vector embedding
                execute(conn, "DELETE FROM vec_memory_files WHERE file_path = ?", (file_path_uri,))

                # Delete file content
                execute(conn, "DELETE FROM file_content WHERE file_path = ?", (file_path_uri,))

        if deleted_count > 0:
            print(f"[SYNC TELEMETRY] Removed {deleted_count} orphaned file entries from database", file=sys.stderr)

        # Clean up orphaned concepts (not referenced in concept_mentions)
        orphaned_count = query_single(
            conn,
            """DELETE FROM concepts
               WHERE concept_name NOT IN (
                   SELECT DISTINCT concept_name FROM concept_mentions
               )"""
        ) or 0
        stats["orphaned_concepts"] = orphaned_count

        # Commit transaction
        conn.commit()

    except Exception:
        # Rollback on error
        if transaction:
            try:
                conn.rollback()
            except Exception as rollback_ex:
                print(
                    f"[SYNC ERROR] Transaction rollback failed: {type(rollback_ex).__name__}: {rollback_ex}",
                    file=sys.stderr
                )
        raise

    # Log telemetry
    print(f"[SYNC TELEMETRY] Extracted {len(unique_concepts)} unique concepts in {extraction_time_ms:.0f}ms", file=sys.stderr)
    print(f"[SYNC TELEMETRY] Generated {concept_embeddings_generated} concept embeddings in {embedding_time_ms:.0f}ms", file=sys.stderr)
    print(f"[SYNC TELEMETRY] Generated {file_embeddings_generated} file embeddings in {embedding_time_ms:.0f}ms", file=sys.stderr)

    # Count file_search rows after sync
    file_search_count_after = query_single(conn, "SELECT COUNT(*) FROM file_search") or 0
    print(f"[SYNC TELEMETRY] file_search rows after sync: {file_search_count_after}", file=sys.stderr)

    total_time_ms = (time.time() - total_start) * 1000
    print(f"[SYNC TELEMETRY] Total sync completed in {total_time_ms:.0f}ms", file=sys.stderr)

    # Build result
    lines = [
        "Sync complete:",
        f"- {stats['files_processed']} FILES processed",
        f"- {stats['concepts_found']} CONCEPT mentions found",
        f"- {stats['relations_created']} CONCEPT relations created",
    ]

    if stats["orphaned_concepts"] > 0:
        lines.append(f"- {stats['orphaned_concepts']} ORPHANED concepts cleaned up")

    # Add vector verification report
    if vector_ready:
        lines.extend(_build_vector_verification_report(conn))
    else:
        lines.append("- WARNING: sqlite-vec extension unavailable; semantic tooling disabled")

    # VACUUM database
    execute(conn, "VACUUM")
    print("[SYNC TELEMETRY] Database VACUUMed to reclaim space", file=sys.stderr)

    conn.close()

    return "\n".join(lines)


def _build_graph_edges(conn: sqlite3.Connection, concepts: list[str], memory_uri: str) -> int:
    """
    Build bidirectional co-occurrence edges between concepts.

    Args:
        conn: SQLite connection
        concepts: List of normalized concept names
        memory_uri: Source file URI

    Returns:
        Number of new edges created
    """
    relations_created = 0

    if len(concepts) <= 1:
        return 0

    # Generate all pairs
    for i in range(len(concepts) - 1):
        for j in range(i + 1, len(concepts)):
            # Sort lexicographically for bidirectional consistency
            if concepts[i] < concepts[j]:
                concept_a, concept_b = concepts[i], concepts[j]
            else:
                concept_a, concept_b = concepts[j], concepts[i]

            # Check if edge exists
            existing = query(
                conn,
                "SELECT co_occurrence_count, source_files FROM concept_graph WHERE concept_a = ? AND concept_b = ?",
                (concept_a, concept_b)
            )

            if existing and len(existing) > 0:
                existing_row = existing[0]
                count, source_files_json = existing_row
                file_list = json.loads(source_files_json)

                # Add file if not already in list
                if memory_uri not in file_list:
                    file_list.append(memory_uri)
                    execute(
                        conn,
                        """UPDATE concept_graph
                           SET co_occurrence_count = ?, source_files = ?
                           WHERE concept_a = ? AND concept_b = ?""",
                        (len(file_list), json.dumps(file_list), concept_a, concept_b)
                    )
            else:
                # Create new edge
                execute(
                    conn,
                    """INSERT OR REPLACE INTO concept_graph
                       (concept_a, concept_b, co_occurrence_count, source_files)
                       VALUES (?, ?, ?, ?)""",
                    (concept_a, concept_b, 1, json.dumps([memory_uri]))
                )
                relations_created += 1

    return relations_created


def _count_concept_occurrences(content: str, concept: str) -> int:
    """
    Count occurrences of [[concept]] in content.

    Simple implementation that counts WikiLink patterns.
    Does not parse markdown AST like C# version.

    Args:
        content: Markdown content
        concept: Normalized concept name

    Returns:
        Number of occurrences
    """
    import re
    from maenifold.utils.markdown_io import normalize_concept

    # Find all [[...]] patterns
    pattern = r"\[\[([^\]]+)\]\]"
    matches = re.findall(pattern, content)

    # Count normalized matches
    count = 0
    for match in matches:
        if normalize_concept(match) == concept:
            count += 1

    return count


def _get_title(file_path: str, frontmatter: Optional[dict]) -> str:
    """
    Extract title from frontmatter or filename.

    Args:
        file_path: Path to file
        frontmatter: Parsed YAML frontmatter

    Returns:
        Title string
    """
    # Try frontmatter first
    if frontmatter and "title" in frontmatter:
        title = str(frontmatter["title"]).strip()
        if title:
            return title

    # Fall back to filename
    title = Path(file_path).stem
    return title if title else "Untitled"


def _try_ensure_vector_support(conn: sqlite3.Connection) -> bool:
    """
    Try to load sqlite-vec extension.

    Args:
        conn: SQLite connection

    Returns:
        True if vector support available, False otherwise
    """
    try:
        load_vector_extension(conn)
        return True
    except Exception as ex:
        print(f"Warning: sqlite-vec unavailable: {ex}", file=sys.stderr)
        return False


def _generate_concept_embeddings(
    conn: sqlite3.Connection,
    concepts: list[str],
    vector_ready: bool
) -> int:
    """
    Generate embeddings for concepts that don't have them.

    Args:
        conn: SQLite connection
        concepts: List of concept names
        vector_ready: Whether vector extension is loaded

    Returns:
        Number of embeddings generated
    """
    if not vector_ready or len(concepts) == 0:
        return 0

    embeddings_generated = 0
    expected_bytes = VectorTools.embedding_length * 4  # 4 bytes per float32

    try:
        for concept in concepts:
            # Check if embedding exists
            has_embedding = query_single(
                conn,
                """SELECT COUNT(*) > 0 FROM vec_concepts
                   WHERE concept_name = ?
                   AND typeof(embedding) = 'blob'
                   AND length(embedding) = ?""",
                (concept, expected_bytes)
            )

            if has_embedding:
                continue

            # Generate embedding
            embedding = VectorTools.generate_embedding(concept)
            embedding_blob = VectorTools.to_sqlite_vector_blob(embedding)

            if len(embedding_blob) != expected_bytes:
                continue

            # Insert embedding
            execute(
                conn,
                "INSERT OR REPLACE INTO vec_concepts (concept_name, embedding) VALUES (?, ?)",
                (concept, embedding_blob)
            )

            embeddings_generated += 1

    except Exception:
        # Silently fail on vector errors
        pass

    return embeddings_generated


def _generate_file_embedding(
    conn: sqlite3.Connection,
    file_path: str,
    content: str,
    vector_ready: bool
) -> bool:
    """
    Generate embedding for file content.

    Args:
        conn: SQLite connection
        file_path: File path URI
        content: File content
        vector_ready: Whether vector extension is loaded

    Returns:
        True if embedding generated, False otherwise
    """
    if not vector_ready:
        return False

    expected_bytes = VectorTools.embedding_length * 4  # 4 bytes per float32

    try:
        # Check if embedding exists
        has_embedding = query_single(
            conn,
            """SELECT COUNT(*) > 0 FROM vec_memory_files
               WHERE file_path = ?
               AND typeof(embedding) = 'blob'
               AND length(embedding) = ?""",
            (file_path, expected_bytes)
        )

        if has_embedding:
            return False

        # Truncate content for embedding (first 1000 chars)
        text_to_embed = content[:1000] if len(content) > 1000 else content

        # Generate embedding
        embedding = VectorTools.generate_embedding(text_to_embed)
        embedding_blob = VectorTools.to_sqlite_vector_blob(embedding)

        if len(embedding_blob) != expected_bytes:
            return False

        # Insert embedding
        execute(
            conn,
            "INSERT OR REPLACE INTO vec_memory_files (file_path, embedding) VALUES (?, ?)",
            (file_path, embedding_blob)
        )

        return True

    except Exception:
        # Silently fail on vector errors
        return False


def _build_vector_verification_report(conn: sqlite3.Connection) -> list[str]:
    """
    Build vector verification report lines.

    Args:
        conn: SQLite connection

    Returns:
        List of report lines
    """
    lines = []

    try:
        # Count concept embeddings
        concept_count = query_single(
            conn,
            "SELECT COUNT(*) FROM vec_concepts WHERE embedding IS NOT NULL"
        ) or 0

        # Count file embeddings
        file_count = query_single(
            conn,
            "SELECT COUNT(*) FROM vec_memory_files WHERE embedding IS NOT NULL"
        ) or 0

        if concept_count == 0 and file_count == 0:
            lines.append("- WARNING: vector tables are empty after sync")
            return lines

        lines.append("Vector index verification:")

        # Concept embeddings
        if concept_count > 0:
            concept_bytes = query_single(
                conn,
                "SELECT length(embedding) FROM vec_concepts WHERE embedding IS NOT NULL LIMIT 1"
            ) or 0
            concept_dims = concept_bytes // 4  # 4 bytes per float32

            # Get self-distance
            concept_self = query_single(
                conn,
                "SELECT vec_distance_cosine(embedding, embedding) FROM vec_concepts WHERE embedding IS NOT NULL LIMIT 1"
            )
            concept_self_str = f"{concept_self:.6f}" if concept_self is not None else "n/a"

            lines.append(f"- CONCEPT embeddings: {concept_count} rows ({concept_dims} dims, self-distance {concept_self_str})")
        else:
            lines.append("- CONCEPT embeddings: none")

        # File embeddings
        if file_count > 0:
            file_bytes = query_single(
                conn,
                "SELECT length(embedding) FROM vec_memory_files WHERE embedding IS NOT NULL LIMIT 1"
            ) or 0
            file_dims = file_bytes // 4  # 4 bytes per float32

            # Get self-distance
            file_self = query_single(
                conn,
                "SELECT vec_distance_cosine(embedding, embedding) FROM vec_memory_files WHERE embedding IS NOT NULL LIMIT 1"
            )
            file_self_str = f"{file_self:.6f}" if file_self is not None else "n/a"

            lines.append(f"- MEMORY embeddings: {file_count} rows ({file_dims} dims, self-distance {file_self_str})")
        else:
            lines.append("- MEMORY embeddings: none")

    except Exception as ex:
        lines.clear()
        lines.append(f"- WARNING: vector verification skipped ({ex})")

    return lines


def _get_help_text() -> str:
    """Return help documentation for sync tool."""
    return """Sync - Rebuild knowledge graph from memory files

Scans all .md files in memory path, extracts [[WikiLink]] concepts, and builds:
- concepts table: normalized concept names with first_seen timestamps
- concept_mentions table: concept-to-file mappings with occurrence counts
- concept_graph table: bidirectional co-occurrence edges
- file_content table: file metadata and content
- vec_concepts table: 384-dim embeddings for concepts (if sqlite-vec available)
- vec_memory_files table: 384-dim embeddings for files (if sqlite-vec available)

Process:
1. Initialize database schema
2. Begin transaction
3. For each .md file:
   - Extract [[WikiLink]] concepts
   - Skip if unchanged (MD5 hash check)
   - Update file_content table
   - Update concepts table (INSERT OR IGNORE for first_seen)
   - Update concept_mentions table
   - Generate embeddings (concepts + file)
   - Build graph edges (bidirectional pairs)
4. Clean up deleted files
5. Clean up orphaned concepts
6. Commit transaction
7. VACUUM database

Returns:
Summary string with statistics:
- Files processed
- Concept mentions found
- Relations created
- Orphaned concepts cleaned
- Vector verification report (if enabled)

Options:
- learn=True: Return this help text instead of executing
"""
