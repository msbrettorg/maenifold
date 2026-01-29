using System;
using System.Text.Json;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Tests for SEC-001: JSON deserialization depth limits to prevent DoS attacks
/// </summary>
public class SafeJsonTests
{
    [Test]
    public void SafeJson_WithNormalDepth_Deserializes()
    {
        // Arrange - Create JSON with depth of 10 (well under limit of 32)
        var json = CreateNestedJson(10);

        // Act & Assert - Should succeed
        Assert.DoesNotThrow(() => JsonSerializer.Deserialize<JsonElement>(json, SafeJson.Options));
    }

    [Test]
    public void SafeJson_WithMaxDepth32_Deserializes()
    {
        // Arrange - Create JSON with exactly depth 32 (at the limit)
        var json = CreateNestedJson(32);

        // Act & Assert - Should succeed
        Assert.DoesNotThrow(() => JsonSerializer.Deserialize<JsonElement>(json, SafeJson.Options));
    }

    [Test]
    public void SafeJson_WithDepthOver32_ThrowsException()
    {
        // Arrange - Create JSON with depth 33 (exceeds limit)
        var json = CreateNestedJson(33);

        // Act & Assert - Should throw JsonException
        var ex = Assert.Throws<JsonException>(() =>
            JsonSerializer.Deserialize<JsonElement>(json, SafeJson.Options));

        Assert.That(ex!.Message, Does.Contain("depth"));
    }

    [Test]
    public void SafeJson_WithDepth64_ThrowsException()
    {
        // Arrange - Create deeply nested JSON (depth 64)
        var json = CreateNestedJson(64);

        // Act & Assert - Should throw JsonException
        var ex = Assert.Throws<JsonException>(() =>
            JsonSerializer.Deserialize<JsonElement>(json, SafeJson.Options));

        Assert.That(ex!.Message, Does.Contain("depth"));
    }

    /// <summary>
    /// Helper method to create nested JSON of a specific depth
    /// </summary>
    private static string CreateNestedJson(int depth)
    {
        var json = "\"value\"";
        for (int i = 0; i < depth; i++)
        {
            json = $"{{\"nested\":{json}}}";
        }
        return json;
    }
}
