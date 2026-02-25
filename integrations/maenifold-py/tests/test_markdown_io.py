"""
Tests for markdown_io utilities.
"""

import os
import tempfile
from pathlib import Path

import pytest

from maenifold.utils.markdown_io import (
    extract_wiki_links,
    generate_checksum,
    normalize_concept,
    path_to_uri,
    read_markdown,
    slugify,
    uri_to_path,
    write_markdown,
    update_markdown,
)


def test_normalize_concept():
    """Test concept normalization."""
    assert normalize_concept("Machine Learning") == "machine-learning"
    assert normalize_concept("foo__bar") == "foo-bar"
    assert normalize_concept("--test--") == "test"
    assert normalize_concept("foo/bar/baz") == "foo-bar-baz"
    assert normalize_concept("Multiple   Spaces") == "multiple-spaces"
    assert normalize_concept("double--hyphen") == "double-hyphen"


def test_slugify():
    """Test title slugification."""
    assert slugify("My Document Title!") == "my-document-title"
    assert slugify("###") == "untitled"
    assert slugify("Test_With_Underscores") == "test-with-underscores"
    assert slugify("   spaces   ") == "spaces"
    assert slugify("") == "untitled"


def test_extract_wiki_links_basic():
    """Test WikiLink extraction from simple content."""
    content = "This discusses [[machine-learning]] and [[neural-networks]]."
    links = extract_wiki_links(content)
    assert set(links) == {"machine-learning", "neural-networks"}


def test_extract_wiki_links_ignores_code_blocks():
    """Test that WikiLinks in code blocks are ignored."""
    content = """
Some text with [[valid-concept]].

```python
# This [[code-concept]] should be ignored
x = "[[string-concept]]"
```

More text with [[another-concept]].
"""
    links = extract_wiki_links(content)
    assert set(links) == {"valid-concept", "another-concept"}
    assert "code-concept" not in links
    assert "string-concept" not in links


def test_extract_wiki_links_ignores_inline_code():
    """Test that WikiLinks in inline code are ignored."""
    content = "Use `[[code-link]]` but not [[real-link]] here."
    links = extract_wiki_links(content)
    assert links == ["real-link"]
    assert "code-link" not in links


def test_extract_wiki_links_normalizes():
    """Test that extracted WikiLinks are normalized."""
    content = "[[Machine Learning]] and [[Neural_Networks]] and [[Deep/Learning]]"
    links = extract_wiki_links(content)
    assert set(links) == {"machine-learning", "neural-networks", "deep-learning"}


def test_write_and_read_markdown_with_frontmatter():
    """Test writing and reading markdown with frontmatter."""
    with tempfile.TemporaryDirectory() as tmpdir:
        file_path = os.path.join(tmpdir, "test.md")

        frontmatter = {"title": "Test Document", "tags": ["test", "demo"], "count": 42}
        content = "# Test\n\nThis is a test with [[concept]]."

        write_markdown(file_path, frontmatter, content)

        # Read it back
        fm, text, checksum = read_markdown(file_path)

        assert fm is not None
        assert fm["title"] == "Test Document"
        assert fm["tags"] == ["test", "demo"]
        assert fm["count"] == 42
        assert text == content
        assert len(checksum) > 0


def test_write_and_read_markdown_without_frontmatter():
    """Test writing and reading markdown without frontmatter."""
    with tempfile.TemporaryDirectory() as tmpdir:
        file_path = os.path.join(tmpdir, "test.md")

        content = "# Test\n\nThis is a test."

        write_markdown(file_path, None, content)

        # Read it back
        fm, text, checksum = read_markdown(file_path)

        assert fm is None
        assert text == content
        assert len(checksum) > 0


def test_read_markdown_with_empty_frontmatter():
    """Test reading markdown with empty frontmatter."""
    with tempfile.TemporaryDirectory() as tmpdir:
        file_path = os.path.join(tmpdir, "test.md")

        # Write file with empty frontmatter manually
        with open(file_path, "w") as f:
            f.write("---\n---\n\nContent here.")

        fm, text, checksum = read_markdown(file_path)

        assert fm is None
        assert text == "Content here."


def test_read_markdown_with_malformed_frontmatter():
    """Test reading markdown with malformed YAML frontmatter."""
    with tempfile.TemporaryDirectory() as tmpdir:
        file_path = os.path.join(tmpdir, "test.md")

        # Write file with invalid YAML
        with open(file_path, "w") as f:
            f.write("---\n{bad yaml: [unclosed\n---\n\nContent here.")

        fm, text, checksum = read_markdown(file_path)

        # Should handle gracefully
        assert fm is None
        assert text == "Content here."


def test_update_markdown_with_checksum():
    """Test updating markdown with checksum validation."""
    with tempfile.TemporaryDirectory() as tmpdir:
        file_path = os.path.join(tmpdir, "test.md")

        # Write initial file
        content1 = "Initial content."
        write_markdown(file_path, None, content1)

        # Read to get checksum
        _, _, checksum1 = read_markdown(file_path)

        # Update with correct checksum
        content2 = "Updated content."
        update_markdown(file_path, None, content2, expected_checksum=checksum1)

        # Verify update
        _, text, _ = read_markdown(file_path)
        assert text == content2


def test_update_markdown_checksum_mismatch():
    """Test that checksum validation catches concurrent modifications."""
    with tempfile.TemporaryDirectory() as tmpdir:
        file_path = os.path.join(tmpdir, "test.md")

        # Write initial file
        write_markdown(file_path, None, "Initial content.")

        # Get checksum
        _, _, checksum1 = read_markdown(file_path)

        # Modify file externally
        write_markdown(file_path, None, "Modified by someone else.")

        # Try to update with stale checksum
        with pytest.raises(ValueError, match="Checksum mismatch"):
            update_markdown(file_path, None, "My update.", expected_checksum=checksum1)


def test_path_to_uri():
    """Test path to URI conversion."""
    base = "/home/user/maenifold/memory"

    uri = path_to_uri("/home/user/maenifold/memory/research/notes.md", base)
    assert uri == "memory://research/notes"

    uri = path_to_uri("/home/user/maenifold/memory/test.md", base)
    assert uri == "memory://test"


def test_uri_to_path():
    """Test URI to path conversion."""
    base = "/home/user/maenifold/memory"

    path = uri_to_path("memory://research/notes", base)
    assert path == "/home/user/maenifold/memory/research/notes.md"

    path = uri_to_path("memory://test", base)
    assert path == "/home/user/maenifold/memory/test.md"


def test_uri_to_path_security():
    """Test that uri_to_path prevents path traversal."""
    base = "/home/user/maenifold/memory"

    with pytest.raises(ValueError, match="escape memory directory"):
        uri_to_path("memory://../../../etc/passwd", base)

    with pytest.raises(ValueError, match="escape memory directory"):
        uri_to_path("memory://../../secrets", base)


def test_generate_checksum():
    """Test checksum generation."""
    content1 = "Test content"
    content2 = "Test content"
    content3 = "Different content"

    checksum1 = generate_checksum(content1)
    checksum2 = generate_checksum(content2)
    checksum3 = generate_checksum(content3)

    # Same content should produce same checksum
    assert checksum1 == checksum2

    # Different content should produce different checksum
    assert checksum1 != checksum3

    # Should be base64 encoded
    assert len(checksum1) > 0


def test_frontmatter_strips_embedding_keys():
    """Test that embedding keys are stripped from frontmatter."""
    with tempfile.TemporaryDirectory() as tmpdir:
        file_path = os.path.join(tmpdir, "test.md")

        frontmatter = {
            "title": "Test",
            "embedding": [0.1, 0.2, 0.3],
            "embedding_base64": "abc123",
            "embedding_model": "text-embedding-3",
            "embedding_date": "2024-01-01",
            "tags": ["test"],
        }
        content = "Content"

        write_markdown(file_path, frontmatter, content)

        # Read back
        fm, _, _ = read_markdown(file_path)

        # Embedding keys should be stripped
        assert "embedding" not in fm
        assert "embedding_base64" not in fm
        assert "embedding_model" not in fm
        assert "embedding_date" not in fm

        # Other keys should remain
        assert fm["title"] == "Test"
        assert fm["tags"] == ["test"]
