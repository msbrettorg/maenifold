"""
Utility modules for maenifold.

This package contains shared utilities for:
- markdown_io: YAML frontmatter parsing, WikiLink extraction, checksum generation
- vector_tools: Embedding generation, vector operations
- safe_json: SEC-001 protected JSON serialization with depth limits
- database: SQLite connection management, schema initialization
- normalization: Concept name normalization, path handling
"""

from maenifold.utils.safe_json import SafeJson

__all__ = ["SafeJson"]
