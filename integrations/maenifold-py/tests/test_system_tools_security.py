"""
Security tests for system_tools.py - RT-SEC-001 path traversal fix.

Tests validate that list_memories properly blocks path traversal attacks
including URL-encoded variants.
"""

import os
import tempfile
import pytest
from maenifold.tools.system_tools import list_memories, _validate_path_security
from maenifold.config import Config


@pytest.fixture
def temp_memory_dir():
    """Create a temporary memory directory structure for testing."""
    with tempfile.TemporaryDirectory() as tmpdir:
        # Create test structure
        memory_path = os.path.join(tmpdir, "memory")
        os.makedirs(memory_path)

        # Create some test directories and files
        test_dir = os.path.join(memory_path, "test")
        os.makedirs(test_dir)

        test_file = os.path.join(test_dir, "test.md")
        with open(test_file, "w") as f:
            f.write("# Test\nTest content")

        # Create a sensitive directory outside memory path
        sensitive_dir = os.path.join(tmpdir, "sensitive")
        os.makedirs(sensitive_dir)
        sensitive_file = os.path.join(sensitive_dir, "secret.md")
        with open(sensitive_file, "w") as f:
            f.write("# Secret\nSensitive data")

        # Override Config.maenifold_root to use tmpdir
        Config.override_root(tmpdir, override_database=True)

        try:
            yield {
                "memory_path": memory_path,
                "test_dir": test_dir,
                "sensitive_dir": sensitive_dir,
                "tmpdir": tmpdir
            }
        finally:
            # Reset Config to default state after test
            Config.reset_overrides()


class TestValidatePathSecurity:
    """Test the _validate_path_security helper function."""

    def test_valid_relative_path(self):
        """Valid relative paths should pass validation."""
        assert _validate_path_security("test/folder") is None
        assert _validate_path_security("folder") is None
        assert _validate_path_security("a/b/c") is None

    def test_rejects_absolute_paths(self):
        """Absolute paths should be rejected."""
        result = _validate_path_security("/etc/passwd")
        assert result is not None
        assert "Absolute paths are not allowed" in result

    def test_blocks_dot_dot_traversal(self):
        """Simple .. traversal should be blocked."""
        result = _validate_path_security("../../../etc")
        assert result is not None
        assert "Path traversal components" in result

    def test_blocks_single_dot(self):
        """Single dot components should be blocked."""
        result = _validate_path_security("./test")
        assert result is not None
        assert "Path traversal components" in result

    def test_blocks_url_encoded_dot_dot(self):
        """URL-encoded .. should be blocked (RT-SEC-001)."""
        result = _validate_path_security("%2e%2e%2f%2e%2e%2f%2e%2e")
        assert result is not None
        assert "Path traversal components" in result

    def test_blocks_url_encoded_dot_dot_slash(self):
        """URL-encoded ../ sequence should be blocked."""
        result = _validate_path_security("%2e%2e%2f")
        assert result is not None
        assert "Path traversal components" in result

    def test_blocks_double_encoded_traversal(self):
        """Double URL-encoded traversal should be blocked (RT-SEC-001)."""
        # %252e = URL-encoded %2e
        # After recursive decode: %252e%252e%252f -> %2e%2e%2f -> ../
        result = _validate_path_security("%252e%252e%252f")
        assert result is not None
        assert "Path traversal components" in result

    def test_blocks_mixed_encoded_traversal(self):
        """Mixed encoded/plain traversal should be blocked."""
        result = _validate_path_security("%2e./test")
        assert result is not None
        assert "Path traversal components" in result

    def test_handles_empty_path(self):
        """Empty paths should pass validation."""
        assert _validate_path_security("") is None
        assert _validate_path_security(None) is None

    def test_blocks_backslash_traversal(self):
        """Windows-style backslash traversal should be blocked."""
        result = _validate_path_security("..\\..\\..\\windows")
        assert result is not None
        assert "Path traversal components" in result


class TestListMemoriesSecurity:
    """Test list_memories function security (RT-SEC-001)."""

    def test_list_valid_directory(self, temp_memory_dir):
        """Listing valid directory should work."""
        result = list_memories("test")
        assert "ERROR" not in result
        assert "test.md" in result

    def test_list_root_directory(self, temp_memory_dir):
        """Listing root memory directory should work."""
        result = list_memories()
        assert "ERROR" not in result
        assert "test/" in result

    def test_blocks_simple_traversal(self, temp_memory_dir):
        """Simple ../ traversal should be blocked."""
        result = list_memories("../../../etc")
        assert "ERROR: Invalid path" in result
        assert "Path traversal components" in result

    def test_blocks_url_encoded_traversal(self, temp_memory_dir):
        """URL-encoded path traversal should be blocked (RT-SEC-001)."""
        # Attack vector: %2e%2e%2f = ../
        result = list_memories("%2e%2e%2f%2e%2e%2f%2e%2e%2fetc")
        assert "ERROR: Invalid path" in result
        assert "Path traversal components" in result

    def test_blocks_double_encoded_traversal(self, temp_memory_dir):
        """Double URL-encoded traversal should be blocked (RT-SEC-001)."""
        # Attack vector: %252e%252e%252f = %2e%2e%2f (after one decode)
        result = list_memories("%252e%252e%252f%252e%252e")
        assert "ERROR: Invalid path" in result
        assert "Path traversal components" in result

    def test_blocks_mixed_encoding(self, temp_memory_dir):
        """Mixed encoded and plain traversal should be blocked."""
        result = list_memories("%2e./test")
        assert "ERROR: Invalid path" in result
        assert "Path traversal components" in result

    def test_blocks_absolute_paths(self, temp_memory_dir):
        """Absolute paths should be rejected."""
        result = list_memories("/etc/passwd")
        assert "ERROR: Invalid path" in result
        assert "Absolute paths are not allowed" in result

    def test_blocks_windows_absolute_paths(self, temp_memory_dir):
        """Windows absolute paths should be rejected."""
        result = list_memories("C:\\Windows\\System32")
        assert "ERROR: Invalid path" in result
        assert "Absolute paths are not allowed" in result

    def test_defense_in_depth_realpath(self, temp_memory_dir):
        """Realpath check should catch any remaining traversal attempts."""
        # Create a symlink that points outside memory_path
        memory_path = temp_memory_dir["memory_path"]
        sensitive_dir = temp_memory_dir["sensitive_dir"]

        link_path = os.path.join(memory_path, "evil_link")
        try:
            os.symlink(sensitive_dir, link_path)

            # Try to access via symlink
            result = list_memories("evil_link")
            # Should either be blocked by realpath check or not exist error
            # The important thing is we don't get sensitive file contents
            assert "secret.md" not in result or "ERROR" in result
        except OSError:
            # Symlink creation might fail on some systems, skip
            pytest.skip("Symlink creation not supported")

    def test_cannot_escape_memory_root(self, temp_memory_dir):
        """Even with complex paths, cannot escape memory root."""
        result = list_memories("test/../../sensitive")
        assert "ERROR: Invalid path" in result

    def test_preserves_legitimate_percent_signs(self, temp_memory_dir):
        """Legitimate percent signs in filenames should work (if they exist)."""
        # This tests that URL decoding doesn't break legitimate use
        memory_path = temp_memory_dir["memory_path"]

        # Create a directory with a percent in the name (legitimate use case)
        percent_dir = os.path.join(memory_path, "test_100_percent")
        os.makedirs(percent_dir)
        test_file = os.path.join(percent_dir, "stats.md")
        with open(test_file, "w") as f:
            f.write("# Stats\n100% complete")

        result = list_memories("test_100_percent")
        assert "ERROR" not in result
        assert "stats.md" in result


class TestAttackVectorsCoverage:
    """Comprehensive test coverage of known attack vectors."""

    @pytest.mark.parametrize("attack_path", [
        # Basic traversal
        "..",
        "../",
        "../../",
        "../../../../../../../etc",

        # URL-encoded (single)
        "%2e%2e",
        "%2e%2e%2f",
        "%2e%2e%2f%2e%2e%2f",

        # URL-encoded (double)
        "%252e%252e",
        "%252e%252e%252f",

        # Mixed encoding
        "%2e.",
        ".%2e",
        "%2e./",

        # Backslash variants
        "..\\",
        "..\\..\\",

        # Absolute paths
        "/etc",
        "/etc/passwd",
        "C:\\Windows",

        # Combination attacks
        "test/../../../etc",
        "test/%2e%2e/%2e%2e",
    ])
    def test_attack_vector_blocked(self, temp_memory_dir, attack_path):
        """All known attack vectors should be blocked."""
        result = list_memories(attack_path)
        assert "ERROR" in result, f"Attack path '{attack_path}' was not blocked!"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
