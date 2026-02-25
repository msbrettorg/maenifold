"""
Configuration management for maenifold.

Provides centralized access to environment variables, paths, and settings.
Thread-safe override support for testing and custom deployments.
"""

import os
import threading
from typing import Optional


class ConfigMeta(type):
    """Metaclass to enable class-level properties for Config."""

    @property
    def maenifold_root(cls) -> str:
        """Root directory for maenifold data."""
        return cls._get_root()

    @property
    def memory_path(cls) -> str:
        """Path to memory directory."""
        return os.path.join(cls._get_root(), "memory")

    @property
    def assets_path(cls) -> str:
        """Path to assets directory."""
        return os.path.join(cls._get_root(), "assets")

    @property
    def thinking_path(cls) -> str:
        """Path to thinking directory."""
        return os.path.join(os.path.join(cls._get_root(), "memory"), "thinking")

    @property
    def sequential_path(cls) -> str:
        """Path to sequential thinking directory."""
        return os.path.join(os.path.join(os.path.join(cls._get_root(), "memory"), "thinking"), "sequential")

    @property
    def database_path(cls) -> str:
        """Path to SQLite database file."""
        with cls._override_lock:
            default_path = os.path.join(cls._get_root(), "memory.db")
            return cls._database_path_override or os.environ.get("MAENIFOLD_DATABASE_PATH", default_path)

    @property
    def database_connection_string(cls) -> str:
        """
        SQLite connection string with pooling disabled.

        RTM: CONNECTION-POOLING-FIX-001 - Disable connection pooling to prevent lock contention.
        """
        return f"Data Source={cls.database_path};Pooling=false"

    @property
    def enable_embedding_logs(cls) -> bool:
        """Enable verbose embedding logs."""
        return os.environ.get("MAENIFOLD_EMBEDDING_LOGS", "false").lower() in ("true", "1", "yes", "on")

    @property
    def enable_vector_search_logs(cls) -> bool:
        """Enable verbose vector search logs."""
        return os.environ.get("MAENIFOLD_VECTOR_LOGS", "false").lower() in ("true", "1", "yes", "on")


class Config(metaclass=ConfigMeta):
    """
    Centralized configuration for maenifold.

    All settings are class-level and loaded from environment variables with defaults.
    Thread-safe override support via override_root() and reset_overrides().
    """

    # Thread safety for overrides (RLock allows reentrant locking)
    _override_lock = threading.RLock()

    # User home directory
    user_home = os.path.expanduser("~")

    # Override state
    _root_override: Optional[str] = None
    _database_path_override: Optional[str] = None

    # Default root from environment
    _default_root = os.environ.get("MAENIFOLD_ROOT", os.path.join(user_home, "maenifold"))

    # Configuration settings from environment
    default_debounce_ms = int(os.environ.get("MAENIFOLD_DEBOUNCE_MS", "150"))
    incremental_optimize_every = max(1, int(os.environ.get("MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY", "40")))
    incremental_vacuum_minutes = int(os.environ.get("MAENIFOLD_INCREMENTAL_VACUUM_MINUTES", "720"))

    enable_incremental_sync = os.environ.get("MAENIFOLD_AUTO_SYNC", "true").lower() in ("true", "1", "yes", "on")
    watcher_buffer_size = int(os.environ.get("MAENIFOLD_WATCHER_BUFFER", "65536"))

    recent_activity_snippet_length = int(os.environ.get("MAENIFOLD_SNIPPET_LENGTH", "1000"))

    sqlite_busy_timeout_ms = int(os.environ.get("MAENIFOLD_SQLITE_BUSY_TIMEOUT", "5000"))

    session_abandonment_minutes = int(os.environ.get("MAENIFOLD_SESSION_ABANDON_MINUTES", "30"))
    enable_session_cleanup = os.environ.get("MAENIFOLD_SESSION_CLEANUP", "true").lower() in ("true", "1", "yes", "on")

    enable_sync_logging = os.environ.get("MAENIFOLD_SYNC_LOGGING", "true").lower() in ("true", "1", "yes", "on")

    @classmethod
    def _get_root(cls) -> str:
        """Get the maenifold root path with override support."""
        with cls._override_lock:
            return cls._root_override if cls._root_override else cls._default_root

    @classmethod
    def override_root(cls, root_path: str, override_database: bool = True) -> None:
        """
        Override the maenifold root path for testing or custom deployments.

        Args:
            root_path: New root directory path
            override_database: If True, also override database path to {root}/memory.db

        Raises:
            ValueError: If root_path is empty or whitespace
        """
        if not root_path or not root_path.strip():
            raise ValueError("Root path cannot be empty.")

        full_path = os.path.abspath(root_path)

        with cls._override_lock:
            cls._root_override = full_path
            if override_database:
                cls._database_path_override = os.path.join(full_path, "memory.db")

    @classmethod
    def reset_overrides(cls) -> None:
        """Reset all path overrides to environment/default values."""
        with cls._override_lock:
            cls._root_override = None
            cls._database_path_override = None

    @classmethod
    def ensure_directories(cls) -> None:
        """Create all required directories if they don't exist."""
        os.makedirs(cls.maenifold_root, exist_ok=True)
        os.makedirs(cls.memory_path, exist_ok=True)
        os.makedirs(cls.assets_path, exist_ok=True)

    @classmethod
    def get_config_summary(cls) -> str:
        """
        Get a human-readable summary of current configuration.

        Returns:
            Multi-line string with key configuration values
        """
        return f"""Maenifold Configuration:
  Memory Path: {cls.memory_path}
  Database: {cls.database_path}
  Debounce: {cls.default_debounce_ms}ms
  Auto Sync: {cls.enable_incremental_sync}"""
