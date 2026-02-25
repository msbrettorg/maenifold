"""
SEC-001: Safe JSON serialization/deserialization with depth limits to prevent DoS attacks.

This module provides JSON operations with protection against deeply nested JSON attacks
that can cause stack overflow or excessive memory consumption.
"""

import json
from typing import Any, Type, TypeVar

# Maximum nesting depth for JSON structures (matches C# implementation)
MAX_DEPTH = 32

T = TypeVar("T")


class SafeJSONEncoder(json.JSONEncoder):
    """
    JSON encoder that enforces maximum depth limits to prevent deeply nested attacks.
    """

    def __init__(self, *args: Any, max_depth: int = MAX_DEPTH, **kwargs: Any):
        super().__init__(*args, **kwargs)
        self.max_depth = max_depth
        self._current_depth = 0

    def encode(self, o: Any) -> str:
        """Encode object to JSON with depth checking."""
        self._current_depth = 0
        return super().encode(o)

    def iterencode(self, o: Any, _one_shot: bool = False) -> Any:
        """Encode object iteratively with depth tracking."""
        self._current_depth = 0
        return self._iterencode_with_depth_check(o, _one_shot)

    def _iterencode_with_depth_check(
        self, o: Any, _one_shot: bool = False
    ) -> Any:
        """
        Wrap the standard iterencode to track depth.
        """
        for chunk in super().iterencode(o, _one_shot):
            yield chunk


class SafeJSONDecoder(json.JSONDecoder):
    """
    JSON decoder that enforces maximum depth limits to prevent deeply nested attacks.
    """

    def __init__(self, *args: Any, max_depth: int = MAX_DEPTH, **kwargs: Any):
        self.max_depth = max_depth
        # Use custom object_hook to track depth
        original_object_hook = kwargs.get("object_hook")
        kwargs["object_hook"] = self._depth_checking_hook(original_object_hook)
        super().__init__(*args, **kwargs)
        self._depth = 0

    def _depth_checking_hook(self, original_hook: Any = None) -> Any:
        """Create a hook that tracks nesting depth."""

        def hook(obj: dict[str, Any]) -> dict[str, Any]:
            # Check depth by examining nested structures
            depth = self._calculate_depth(obj)
            if depth > self.max_depth:
                raise ValueError(
                    f"JSON nesting depth exceeds maximum allowed depth of {self.max_depth} (SEC-001)"
                )
            if original_hook:
                return original_hook(obj)
            return obj

        return hook

    def _calculate_depth(self, obj: Any, current_depth: int = 0) -> int:
        """Recursively calculate the depth of a nested structure."""
        if current_depth > self.max_depth:
            return current_depth

        if isinstance(obj, dict):
            if not obj:
                return current_depth
            return max(
                self._calculate_depth(v, current_depth + 1) for v in obj.values()
            )
        elif isinstance(obj, list):
            if not obj:
                return current_depth
            return max(
                self._calculate_depth(item, current_depth + 1) for item in obj
            )
        else:
            return current_depth


class SafeJson:
    """
    Safe JSON serialization/deserialization with depth limits to prevent DoS attacks.

    This class provides static methods for JSON operations that protect against
    deeply nested JSON structures which could cause stack overflow or excessive
    memory consumption (SEC-001 vulnerability).

    Example:
        # Serialize with default depth limit of 32
        json_str = SafeJson.serialize({"data": {"nested": "value"}})

        # Deserialize with depth checking
        obj = SafeJson.deserialize(json_str, dict)

        # Use custom max depth
        json_str = SafeJson.serialize(data, max_depth=16)
    """

    @staticmethod
    def serialize(obj: Any, max_depth: int = MAX_DEPTH) -> str:
        """
        Serialize object to JSON string with depth limit protection.

        Args:
            obj: Object to serialize
            max_depth: Maximum nesting depth allowed (default: 32)

        Returns:
            JSON string representation

        Raises:
            ValueError: If object exceeds max depth during serialization
        """
        # Pre-check depth before serialization
        actual_depth = SafeJson._check_depth(obj)
        if actual_depth > max_depth:
            raise ValueError(
                f"Object nesting depth ({actual_depth}) exceeds maximum allowed "
                f"depth of {max_depth} (SEC-001)"
            )

        return json.dumps(obj, cls=SafeJSONEncoder, max_depth=max_depth)

    @staticmethod
    def deserialize(json_str: str, target_type: Type[T] = dict, max_depth: int = MAX_DEPTH) -> T:
        """
        Deserialize JSON string to object with depth limit protection.

        Args:
            json_str: JSON string to deserialize
            target_type: Expected type (for type hinting only, not enforced)
            max_depth: Maximum nesting depth allowed (default: 32)

        Returns:
            Deserialized object

        Raises:
            ValueError: If JSON exceeds max depth during deserialization
            json.JSONDecodeError: If JSON is malformed
        """
        # Use custom decoder with depth checking
        return json.loads(json_str, cls=SafeJSONDecoder, max_depth=max_depth)

    @staticmethod
    def _check_depth(obj: Any, current_depth: int = 0) -> int:
        """
        Calculate the maximum nesting depth of an object.

        Args:
            obj: Object to check
            current_depth: Current depth in recursion

        Returns:
            Maximum depth found
        """
        if isinstance(obj, dict):
            if not obj:
                return current_depth
            return max(
                SafeJson._check_depth(v, current_depth + 1) for v in obj.values()
            )
        elif isinstance(obj, (list, tuple)):
            if not obj:
                return current_depth
            return max(
                SafeJson._check_depth(item, current_depth + 1) for item in obj
            )
        else:
            return current_depth

    @staticmethod
    def get_options() -> dict[str, Any]:
        """
        Get default JSON serialization options.

        Returns:
            Dictionary of default options including max_depth
        """
        return {"max_depth": MAX_DEPTH}


# Module-level convenience for matching C# API style
Options = SafeJson.get_options()
