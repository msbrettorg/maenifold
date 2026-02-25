"""
Time utility functions for maenifold.

Provides ISO 8601 timestamp generation for consistent datetime formatting
across the knowledge graph.
"""

from datetime import datetime, timezone


def get_utc_now_iso() -> str:
    """
    Get current UTC time as ISO 8601 string.

    Returns:
        ISO 8601 formatted timestamp with 'Z' suffix (e.g., "2024-01-14T20:30:00Z")

    Examples:
        >>> get_utc_now_iso()
        '2024-01-14T20:30:00Z'
    """
    now_utc = datetime.now(timezone.utc)
    # Format as ISO 8601 with 'Z' suffix
    return now_utc.strftime("%Y-%m-%dT%H:%M:%SZ")
