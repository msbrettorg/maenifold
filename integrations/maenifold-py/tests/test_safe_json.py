"""
Tests for safe_json module (SEC-001 protection).

These tests verify that SafeJson prevents deeply nested JSON attacks
while maintaining correct functionality for normal JSON operations.
"""

import json
import pytest
from maenifold.utils.safe_json import SafeJson, MAX_DEPTH


class TestSafeJsonSerialization:
    """Tests for SafeJson.serialize() method."""

    def test_serialize_simple_dict(self):
        """Test serialization of simple dictionary."""
        data = {"name": "test", "value": 42}
        result = SafeJson.serialize(data)
        assert json.loads(result) == data

    def test_serialize_simple_list(self):
        """Test serialization of simple list."""
        data = [1, 2, 3, "four", 5.0]
        result = SafeJson.serialize(data)
        assert json.loads(result) == data

    def test_serialize_nested_within_limit(self):
        """Test serialization of nested structure within depth limit."""
        # Create nested structure with depth of 10 (well within limit)
        data = {"level": 1}
        current = data
        for i in range(2, 11):
            current["nested"] = {"level": i}
            current = current["nested"]

        result = SafeJson.serialize(data)
        parsed = json.loads(result)
        assert parsed["level"] == 1

    def test_serialize_at_max_depth(self):
        """Test serialization at exactly max depth (should succeed)."""
        # Create nested structure at exactly MAX_DEPTH
        data = {"level": 0}
        current = data
        for i in range(1, MAX_DEPTH):
            current["nested"] = {"level": i}
            current = current["nested"]

        result = SafeJson.serialize(data)
        assert result is not None

    def test_serialize_exceeds_max_depth(self):
        """Test that serialization fails when exceeding max depth (SEC-001)."""
        # Create nested structure exceeding MAX_DEPTH
        data = {"level": 0}
        current = data
        for i in range(1, MAX_DEPTH + 5):
            current["nested"] = {"level": i}
            current = current["nested"]

        with pytest.raises(ValueError) as exc_info:
            SafeJson.serialize(data)

        assert "exceeds maximum allowed depth" in str(exc_info.value)
        assert "SEC-001" in str(exc_info.value)

    def test_serialize_custom_max_depth(self):
        """Test serialization with custom max depth."""
        # Create structure with depth of 10
        data = {"level": 0}
        current = data
        for i in range(1, 10):
            current["nested"] = {"level": i}
            current = current["nested"]

        # Should succeed with max_depth=10
        result = SafeJson.serialize(data, max_depth=10)
        assert result is not None

        # Should fail with max_depth=5
        with pytest.raises(ValueError):
            SafeJson.serialize(data, max_depth=5)

    def test_serialize_mixed_nesting(self):
        """Test serialization with mixed dict/list nesting."""
        data = {
            "users": [
                {"name": "Alice", "data": {"settings": {"theme": "dark"}}},
                {"name": "Bob", "data": {"settings": {"theme": "light"}}},
            ]
        }
        result = SafeJson.serialize(data)
        assert json.loads(result) == data


class TestSafeJsonDeserialization:
    """Tests for SafeJson.deserialize() method."""

    def test_deserialize_simple_dict(self):
        """Test deserialization of simple dictionary."""
        json_str = '{"name": "test", "value": 42}'
        result = SafeJson.deserialize(json_str)
        assert result == {"name": "test", "value": 42}

    def test_deserialize_simple_list(self):
        """Test deserialization of simple list."""
        json_str = '[1, 2, 3, "four", 5.0]'
        result = SafeJson.deserialize(json_str, list)
        assert result == [1, 2, 3, "four", 5.0]

    def test_deserialize_nested_within_limit(self):
        """Test deserialization of nested structure within depth limit."""
        # Create JSON with depth of 10
        data = {"level": 0}
        current = data
        for i in range(1, 10):
            current["nested"] = {"level": i}
            current = current["nested"]

        json_str = json.dumps(data)
        result = SafeJson.deserialize(json_str)
        assert result["level"] == 0

    def test_deserialize_at_max_depth(self):
        """Test deserialization at exactly max depth (should succeed)."""
        # Create JSON at exactly MAX_DEPTH
        data = {"level": 0}
        current = data
        for i in range(1, MAX_DEPTH):
            current["nested"] = {"level": i}
            current = current["nested"]

        json_str = json.dumps(data)
        result = SafeJson.deserialize(json_str)
        assert result is not None

    def test_deserialize_exceeds_max_depth(self):
        """Test that deserialization fails when exceeding max depth (SEC-001)."""
        # Create deeply nested JSON exceeding MAX_DEPTH
        data = {"level": 0}
        current = data
        for i in range(1, MAX_DEPTH + 5):
            current["nested"] = {"level": i}
            current = current["nested"]

        json_str = json.dumps(data)

        with pytest.raises(ValueError) as exc_info:
            SafeJson.deserialize(json_str)

        assert "exceeds maximum allowed depth" in str(exc_info.value)
        assert "SEC-001" in str(exc_info.value)

    def test_deserialize_custom_max_depth(self):
        """Test deserialization with custom max depth."""
        # Create JSON with depth of 10
        data = {"level": 0}
        current = data
        for i in range(1, 10):
            current["nested"] = {"level": i}
            current = current["nested"]

        json_str = json.dumps(data)

        # Should succeed with max_depth=10
        result = SafeJson.deserialize(json_str, max_depth=10)
        assert result is not None

        # Should fail with max_depth=5
        with pytest.raises(ValueError):
            SafeJson.deserialize(json_str, max_depth=5)

    def test_deserialize_malformed_json(self):
        """Test that malformed JSON raises appropriate error."""
        with pytest.raises(json.JSONDecodeError):
            SafeJson.deserialize("{invalid json")


class TestSafeJsonRoundTrip:
    """Tests for serialize -> deserialize round trips."""

    def test_roundtrip_simple_data(self):
        """Test round trip with simple data structures."""
        test_cases = [
            {"simple": "dict"},
            [1, 2, 3],
            {"mixed": [1, 2, {"nested": "value"}]},
            {"unicode": "测试", "emoji": "🔐"},
        ]

        for data in test_cases:
            serialized = SafeJson.serialize(data)
            deserialized = SafeJson.deserialize(serialized)
            assert deserialized == data

    def test_roundtrip_complex_nested(self):
        """Test round trip with complex nested structure."""
        data = {
            "users": {
                "alice": {
                    "profile": {"name": "Alice", "age": 30},
                    "settings": {"theme": "dark", "notifications": True},
                },
                "bob": {
                    "profile": {"name": "Bob", "age": 25},
                    "settings": {"theme": "light", "notifications": False},
                },
            },
            "metadata": {"version": "1.0", "created": "2024-01-01"},
        }

        serialized = SafeJson.serialize(data)
        deserialized = SafeJson.deserialize(serialized)
        assert deserialized == data


class TestSafeJsonOptions:
    """Tests for SafeJson.get_options() method."""

    def test_get_options(self):
        """Test that get_options returns expected configuration."""
        options = SafeJson.get_options()
        assert "max_depth" in options
        assert options["max_depth"] == MAX_DEPTH


class TestEdgeCases:
    """Tests for edge cases and special scenarios."""

    def test_empty_structures(self):
        """Test handling of empty structures."""
        assert SafeJson.deserialize(SafeJson.serialize({})) == {}
        assert SafeJson.deserialize(SafeJson.serialize([])) == []

    def test_null_values(self):
        """Test handling of null/None values."""
        data = {"key": None, "nested": {"value": None}}
        serialized = SafeJson.serialize(data)
        deserialized = SafeJson.deserialize(serialized)
        assert deserialized == data

    def test_numeric_precision(self):
        """Test that numeric values maintain precision."""
        data = {"int": 42, "float": 3.14159, "scientific": 1.23e-10}
        serialized = SafeJson.serialize(data)
        deserialized = SafeJson.deserialize(serialized)
        assert deserialized["int"] == 42
        assert abs(deserialized["float"] - 3.14159) < 0.00001
        assert abs(deserialized["scientific"] - 1.23e-10) < 1e-15

    def test_deep_list_nesting(self):
        """Test depth calculation with deeply nested lists."""
        # Create deeply nested list structure
        data = [[[[[[[[[["deep"]]]]]]]]]]  # 11 levels
        serialized = SafeJson.serialize(data)
        deserialized = SafeJson.deserialize(serialized)
        assert deserialized == data

    def test_mixed_depth_branches(self):
        """Test structure with different depth branches."""
        data = {
            "shallow": {"level": 1},
            "deep": {
                "l1": {"l2": {"l3": {"l4": {"l5": {"value": "deep"}}}}}
            },
        }
        serialized = SafeJson.serialize(data)
        deserialized = SafeJson.deserialize(serialized)
        assert deserialized == data


class TestSecurityVectorSEC001:
    """Tests specifically for SEC-001 security vulnerability."""

    def test_dos_attack_prevention_serialize(self):
        """Test that serialization prevents DoS via deep nesting."""
        # Simulate attacker-controlled deeply nested structure
        attack_data = {}
        current = attack_data
        for i in range(100):  # Far exceeds MAX_DEPTH
            current["attack"] = {}
            current = current["attack"]

        with pytest.raises(ValueError) as exc_info:
            SafeJson.serialize(attack_data)

        assert "SEC-001" in str(exc_info.value)

    def test_dos_attack_prevention_deserialize(self):
        """Test that deserialization prevents DoS via deep nesting."""
        # Build attack JSON manually to bypass SafeJson serialization
        attack_json = "{"
        for i in range(100):  # Far exceeds MAX_DEPTH
            attack_json += '"attack":{'
        attack_json += '"value":"payload"'
        attack_json += "}" * 100

        with pytest.raises(ValueError) as exc_info:
            SafeJson.deserialize(attack_json)

        assert "SEC-001" in str(exc_info.value)

    def test_array_based_dos_prevention(self):
        """Test prevention of array-based deep nesting attacks."""
        # Create deeply nested array structure
        attack_data = []
        current = attack_data
        for i in range(100):
            nested = []
            current.append(nested)
            current = nested

        with pytest.raises(ValueError):
            SafeJson.serialize(attack_data)
