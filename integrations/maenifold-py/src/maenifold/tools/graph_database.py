"""
Graph database initialization for maenifold.

Creates and manages SQLite database schema for:
- Concept tracking and co-occurrence graphs
- File content storage with FTS5 full-text search
- Vector embeddings (concepts and memory files)
"""

import sqlite3
import sys

from maenifold.config import Config
from maenifold.utils import sqlite_extensions


def initialize_database() -> None:
    """
    Initialize the maenifold database with required tables and indices.

    Creates:
    - Concept tables (concepts, concept_mentions)
    - File tables (file_content, file_search with FTS5)
    - Graph tables (concept_graph)
    - Vector tables (vec_concepts, vec_memory_files)

    Configures database with:
    - Foreign keys enabled
    - WAL journal mode
    - NORMAL synchronous mode
    """
    config = Config()
    conn = sqlite3.connect(Config.database_path)

    try:
        # Configure connection
        sqlite_extensions.open_with_wal(conn)
        sqlite_extensions.execute(conn, "PRAGMA foreign_keys=ON")

        # Create all tables
        _create_concept_tables(conn)
        _create_file_tables(conn)
        _create_graph_tables(conn)
        _create_vector_tables(conn)
        _add_missing_columns(conn)

    finally:
        conn.close()


def _create_concept_tables(conn: sqlite3.Connection) -> None:
    """
    Create concept tracking tables.

    Tables:
    - concepts: Normalized concept names with first_seen timestamp and occurrence counts
    - concept_mentions: Links concepts to source files with mention counts
    """
    sqlite_extensions.execute(conn, """
        CREATE TABLE IF NOT EXISTS concepts (
            concept_name TEXT PRIMARY KEY,
            first_seen TEXT,
            occurrence_count INTEGER DEFAULT 0
        )
    """)

    sqlite_extensions.execute(conn, """
        CREATE TABLE IF NOT EXISTS concept_mentions (
            concept_name TEXT NOT NULL,
            source_file TEXT NOT NULL,
            mention_count INTEGER DEFAULT 1,
            PRIMARY KEY (concept_name, source_file),
            FOREIGN KEY (concept_name) REFERENCES concepts(concept_name) ON DELETE CASCADE
        )
    """)

    sqlite_extensions.execute(conn,
        "CREATE INDEX IF NOT EXISTS idx_mentions_by_file ON concept_mentions(source_file)"
    )


def _create_file_tables(conn: sqlite3.Connection) -> None:
    """
    Create file content tables with FTS5 full-text search.

    Tables:
    - file_content: Stores file metadata and content
    - file_search: FTS5 virtual table for full-text search

    Triggers:
    - file_content_ai: AFTER INSERT - sync to FTS5
    - file_content_ad: AFTER DELETE - remove from FTS5
    - file_content_au: AFTER UPDATE - update FTS5
    """
    sqlite_extensions.execute(conn, """
        CREATE TABLE IF NOT EXISTS file_content (
            file_path TEXT PRIMARY KEY,
            title TEXT,
            content TEXT,
            summary TEXT,
            last_indexed TEXT,
            status TEXT,
            file_md5 TEXT
        )
    """)

    sqlite_extensions.execute(conn, """
        CREATE VIRTUAL TABLE IF NOT EXISTS file_search USING fts5(
            title,
            content,
            summary,
            content=file_content,
            content_rowid=rowid,
            tokenize='unicode61 remove_diacritics 0'
        )
    """)

    # Trigger: INSERT
    sqlite_extensions.execute(conn, """
        CREATE TRIGGER IF NOT EXISTS file_content_ai AFTER INSERT ON file_content BEGIN
            INSERT INTO file_search(rowid, title, content, summary)
            VALUES (new.rowid, new.title, new.content, new.summary);
        END
    """)

    # Trigger: DELETE
    sqlite_extensions.execute(conn, """
        CREATE TRIGGER IF NOT EXISTS file_content_ad AFTER DELETE ON file_content BEGIN
            INSERT INTO file_search(file_search, rowid, title, content, summary)
            VALUES('delete', old.rowid, old.title, old.content, old.summary);
        END
    """)

    # Trigger: UPDATE
    sqlite_extensions.execute(conn, """
        CREATE TRIGGER IF NOT EXISTS file_content_au AFTER UPDATE ON file_content BEGIN
            INSERT INTO file_search(file_search, rowid, title, content, summary)
            VALUES('delete', old.rowid, old.title, old.content, old.summary);
            INSERT INTO file_search(rowid, title, content, summary)
            VALUES (new.rowid, new.title, new.content, new.summary);
        END
    """)


def _create_graph_tables(conn: sqlite3.Connection) -> None:
    """
    Create concept graph tables for co-occurrence tracking.

    Tables:
    - concept_graph: Bidirectional edges between concepts with co-occurrence counts
    """
    sqlite_extensions.execute(conn, """
        CREATE TABLE IF NOT EXISTS concept_graph (
            concept_a TEXT NOT NULL,
            concept_b TEXT NOT NULL,
            co_occurrence_count INTEGER,
            source_files TEXT,
            PRIMARY KEY (concept_a, concept_b),
            FOREIGN KEY (concept_a) REFERENCES concepts(concept_name) ON DELETE CASCADE,
            FOREIGN KEY (concept_b) REFERENCES concepts(concept_name) ON DELETE CASCADE
        )
    """)

    sqlite_extensions.execute(conn,
        "CREATE INDEX IF NOT EXISTS idx_graph_concept_b ON concept_graph(concept_b)"
    )


def _create_vector_tables(conn: sqlite3.Connection) -> None:
    """
    Create vector embedding tables using sqlite-vec extension.

    Tables:
    - vec_concepts: 384-dimensional embeddings for concept names
    - vec_memory_files: 384-dimensional embeddings for memory files

    Note:
        Vector table creation is wrapped in try-except for graceful degradation
        if the vec0 extension is not available.
    """
    try:
        # Load vector extension
        sqlite_extensions.load_vector_extension(conn)

        # Create concept embeddings table
        sqlite_extensions.execute(conn, """
            CREATE VIRTUAL TABLE IF NOT EXISTS vec_concepts
            USING vec0(concept_name TEXT PRIMARY KEY, embedding FLOAT[384])
        """)

        # Create memory file embeddings table
        sqlite_extensions.execute(conn, """
            CREATE VIRTUAL TABLE IF NOT EXISTS vec_memory_files
            USING vec0(file_path TEXT PRIMARY KEY, embedding FLOAT[384])
        """)

    except Exception as e:
        # Log warning but don't fail initialization
        print(f"Warning: Failed to create vector tables: {e}", file=sys.stderr)


def _add_missing_columns(conn: sqlite3.Connection) -> None:
    """
    Add missing columns for backward compatibility.

    Attempts to add:
    - status column to file_content
    - file_md5 column to file_content

    Silently ignores errors if columns already exist.
    """
    # Add status column
    try:
        sqlite_extensions.execute(conn, "ALTER TABLE file_content ADD COLUMN status TEXT")
    except sqlite3.OperationalError:
        # Column already exists, ignore
        pass

    # Add file_md5 column
    try:
        sqlite_extensions.execute(conn, "ALTER TABLE file_content ADD COLUMN file_md5 TEXT")
    except sqlite3.OperationalError:
        # Column already exists, ignore
        pass
