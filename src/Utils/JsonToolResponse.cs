using System.Text.Json;
using System.Text.Json.Serialization;

namespace Maenifold.Utils;

/// <summary>
/// T-CLI-JSON-001: RTM FR-8.3
/// Structured JSON response for all MCP tools when --json flag is used.
/// </summary>
public class JsonToolResponse
{
    // T-CLI-JSON-001: Cached options for serialization (CA1869 compliance)
    private static readonly JsonSerializerOptions s_options = new(SafeJson.Options)
    {
        WriteIndented = false,
        DefaultIgnoreCondition = JsonIgnoreCondition.Never
    };

    [JsonPropertyName("success")]
    public bool Success { get; set; }

    [JsonPropertyName("data")]
    public object? Data { get; set; }

    [JsonPropertyName("error")]
    public JsonError? Error { get; set; }

    /// <summary>
    /// Create a successful response with data.
    /// </summary>
    public static JsonToolResponse Ok(object? data) => new()
    {
        Success = true,
        Data = data,
        Error = null
    };

    /// <summary>
    /// Create an error response.
    /// </summary>
    public static JsonToolResponse Fail(string code, string message) => new()
    {
        Success = false,
        Data = null,
        Error = new JsonError { Code = code, Message = message }
    };

    /// <summary>
    /// Serialize to JSON string using safe options.
    /// </summary>
    public string ToJson()
    {
        return JsonSerializer.Serialize(this, s_options);
    }
}

/// <summary>
/// T-CLI-JSON-001: RTM FR-8.4
/// Structured error object with code and message.
/// </summary>
public class JsonError
{
    [JsonPropertyName("code")]
    public string Code { get; set; } = string.Empty;

    [JsonPropertyName("message")]
    public string Message { get; set; } = string.Empty;
}
