using System.Text.Json;

namespace Maenifold.Utils;

/// <summary>
/// SEC-001: Safe JSON deserialization options to prevent DoS via deeply nested JSON
/// </summary>
public static class SafeJson
{
    /// <summary>
    /// Shared JsonSerializerOptions with MaxDepth=32 to prevent stack overflow from deeply nested JSON
    /// </summary>
    public static readonly JsonSerializerOptions Options = new() { MaxDepth = 32 };

    /// <summary>
    /// Shared JsonDocumentOptions with MaxDepth=32 to prevent stack overflow from deeply nested JSON
    /// </summary>
    public static readonly JsonDocumentOptions DocumentOptions = new() { MaxDepth = 32 };
}
