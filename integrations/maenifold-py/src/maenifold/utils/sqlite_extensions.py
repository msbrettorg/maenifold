"""
SQLite connection extensions for maenifold.

Provides utility functions for:
- Opening SQLite connections with WAL mode
- Loading platform-specific vector extensions (vec0)
- Executing queries with parameter binding
- Query result mapping
"""

import platform
import sqlite3
import sys
from contextlib import contextmanager
from pathlib import Path
from typing import Any, Generator, Optional, Union

from maenifold.config import Config


def open_with_wal(conn: sqlite3.Connection) -> None:
    """
    Configure SQLite connection with WAL mode and production settings.

    Sets:
    - PRAGMA busy_timeout from config
    - PRAGMA journal_mode=WAL
    - PRAGMA synchronous=NORMAL

    Args:
        conn: SQLite connection object (must be opened)
    """
    execute(conn, f"PRAGMA busy_timeout={Config.sqlite_busy_timeout_ms}")
    execute(conn, "PRAGMA journal_mode=WAL")
    execute(conn, "PRAGMA synchronous=NORMAL")


@contextmanager
def open_read_only(db_path: str) -> Generator[sqlite3.Connection, None, None]:
    """
    Open a read-only SQLite connection with WAL mode as a context manager.

    Sets:
    - Enable extension loading (required for sqlite-vec) if available
    - PRAGMA busy_timeout from config
    - PRAGMA journal_mode=WAL
    - PRAGMA query_only=ON
    - PRAGMA read_uncommitted=ON

    Args:
        db_path: Path to SQLite database file

    Yields:
        SQLite connection object configured for read-only access

    Example:
        with open_read_only(Config.database_path) as conn:
            results = query(conn, "SELECT * FROM table")
    """
    conn = sqlite3.connect(db_path)
    try:
        # Enable extension loading BEFORE setting any pragmas (if available)
        # Some Python builds (macOS system Python) don't have this compiled in
        if hasattr(conn, 'enable_load_extension'):
            conn.enable_load_extension(True)

        execute(conn, f"PRAGMA busy_timeout={Config.sqlite_busy_timeout_ms}")
        execute(conn, "PRAGMA journal_mode=WAL")
        execute(conn, "PRAGMA query_only=ON")
        execute(conn, "PRAGMA read_uncommitted=ON")
        yield conn
    finally:
        conn.close()


def load_vector_extension(conn: sqlite3.Connection) -> None:
    """
    Load platform-specific sqlite-vec (vec0) extension.

    Detects platform and loads the appropriate native library:
    - Windows (win-x64): vec0.dll
    - Linux (linux-x64): vec0.so
    - macOS ARM64 (osx-arm64): vec0.dylib
    - macOS x64 (osx-x64): vec0.dylib

    Extension path: assets/native/{rid}/vec0.{ext}

    Args:
        conn: SQLite connection object (must have enable_load_extension(True) already called)

    Raises:
        PlatformNotSupportedException: If platform is not supported
        FileNotFoundError: If extension library not found at expected path
        RuntimeError: If extension loading is not supported by Python's sqlite3
    """
    # Check if extension loading is available
    if not hasattr(conn, 'enable_load_extension') or not hasattr(conn, 'load_extension'):
        raise RuntimeError(
            "Extension loading not supported by Python's sqlite3 module.\n"
            "Install Python with sqlite3 extension support, or use pysqlite3-binary:\n"
            "  pip install pysqlite3-binary"
        )

    # Detect platform
    system = platform.system()
    machine = platform.machine().lower()

    if system == "Windows":
        rid = "win-x64"
        ext = "dll"
    elif system == "Linux":
        rid = "linux-x64"
        ext = "so"
    elif system == "Darwin":  # macOS
        if machine in ("arm64", "aarch64"):
            rid = "osx-arm64"
        else:
            rid = "osx-x64"
        ext = "dylib"
    else:
        raise NotImplementedError(f"Platform not supported: {system}")

    # Construct path to extension
    extension_path = Path(Config.assets_path) / "native" / rid / f"vec0.{ext}"

    if not extension_path.exists():
        raise FileNotFoundError(
            f"Vector extension not found at: {extension_path}\n"
            f"Expected platform: {rid}"
        )

    # Load extension (caller must have already called conn.enable_load_extension(True))
    conn.load_extension(str(extension_path))


def execute(
    conn: sqlite3.Connection,
    sql: str,
    params: Optional[Union[dict, tuple, list]] = None
) -> None:
    """
    Execute SQL statement with optional parameter binding.

    Supports both positional (?) and named (:name) parameter styles.

    Args:
        conn: SQLite connection object
        sql: SQL statement to execute
        params: Optional parameters (dict for named, tuple/list for positional)

    Example:
        execute(conn, "INSERT INTO users (name) VALUES (?)", ("Alice",))
        execute(conn, "INSERT INTO users (name) VALUES (:name)", {"name": "Bob"})
    """
    cursor = conn.cursor()
    try:
        if params is None:
            cursor.execute(sql)
        else:
            cursor.execute(sql, params)
        conn.commit()
    finally:
        cursor.close()


def query(
    conn: sqlite3.Connection,
    sql: str,
    params: Optional[Union[dict, tuple, list]] = None
) -> list:
    """
    Execute query and return all results as a list.

    Returns rows as tuples. For single column queries, returns list of scalar values.

    Args:
        conn: SQLite connection object
        sql: SQL query to execute
        params: Optional parameters (dict for named, tuple/list for positional)

    Returns:
        List of result tuples or scalar values

    Example:
        # Returns list of tuples: [(1, "Alice"), (2, "Bob")]
        users = query(conn, "SELECT id, name FROM users")

        # Returns list of scalars: ["Alice", "Bob"]
        names = query(conn, "SELECT name FROM users")
    """
    cursor = conn.cursor()
    try:
        if params is None:
            cursor.execute(sql)
        else:
            cursor.execute(sql, params)

        rows = cursor.fetchall()

        # If single column, unwrap to scalar values
        if rows and len(rows[0]) == 1:
            return [row[0] for row in rows]

        return rows
    finally:
        cursor.close()


def query_single(
    conn: sqlite3.Connection,
    sql: str,
    params: Optional[Union[dict, tuple, list]] = None
) -> Any:
    """
    Execute query and return single scalar result.

    Returns first column of first row, or None if no results.

    Args:
        conn: SQLite connection object
        sql: SQL query to execute (should return single value)
        params: Optional parameters (dict for named, tuple/list for positional)

    Returns:
        Scalar value (first column of first row) or None

    Example:
        count = query_single(conn, "SELECT COUNT(*) FROM users")
        name = query_single(conn, "SELECT name FROM users WHERE id = ?", (1,))
    """
    cursor = conn.cursor()
    try:
        if params is None:
            cursor.execute(sql)
        else:
            cursor.execute(sql, params)

        row = cursor.fetchone()
        if row is None:
            return None

        return row[0]
    finally:
        cursor.close()
