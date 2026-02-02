namespace Maenifold.Utils;

/// <summary>
/// T-CLI-JSON-001: RTM FR-8.1, FR-8.5
/// Thread-local context for controlling output format.
/// Default is Markdown for backward compatibility.
/// </summary>
public static class OutputContext
{
    private static readonly AsyncLocal<OutputFormat> _format = new();

    /// <summary>
    /// Gets the current output format. Defaults to Markdown if not set.
    /// </summary>
    public static OutputFormat Format
    {
        get => _format.Value;
        set => _format.Value = value;
    }

    /// <summary>
    /// Check if JSON output is enabled.
    /// </summary>
    public static bool IsJsonMode => _format.Value == OutputFormat.Json;

    /// <summary>
    /// Set output format to JSON for the current async context.
    /// Returns a disposable that restores the previous format.
    /// </summary>
    public static IDisposable UseJson()
    {
        var previous = _format.Value;
        _format.Value = OutputFormat.Json;
        return new OutputContextScope(previous);
    }

    /// <summary>
    /// Set output format to Markdown for the current async context.
    /// Returns a disposable that restores the previous format.
    /// </summary>
    public static IDisposable UseMarkdown()
    {
        var previous = _format.Value;
        _format.Value = OutputFormat.Markdown;
        return new OutputContextScope(previous);
    }

    private class OutputContextScope : IDisposable
    {
        private readonly OutputFormat _previousFormat;
        private bool _disposed;

        public OutputContextScope(OutputFormat previousFormat)
        {
            _previousFormat = previousFormat;
        }

        public void Dispose()
        {
            if (!_disposed)
            {
                _format.Value = _previousFormat;
                _disposed = true;
            }
        }
    }
}

/// <summary>
/// Output format enumeration.
/// </summary>
public enum OutputFormat
{
    /// <summary>
    /// Human-readable markdown format (default, backward compatible).
    /// </summary>
    Markdown = 0,

    /// <summary>
    /// Structured JSON format for programmatic access.
    /// </summary>
    Json = 1
}
