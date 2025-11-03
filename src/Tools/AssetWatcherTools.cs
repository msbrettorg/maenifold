using System.IO;
using System.ComponentModel;
using System.Timers;
using ModelContextProtocol.Server;
using Maenifold.Utils;

namespace Maenifold.Tools;

/// <summary>
/// Asset hot-loading via FileSystemWatcher for JSON asset files.
/// Monitors ~/maenifold/assets/ for file changes and debounces notifications.
/// Wave 4: Sends notifications/resources/list_changed to MCP clients when assets change.
/// </summary>
[McpServerToolType]
public static class AssetWatcherTools
{
    private static FileSystemWatcher? _watcher;
    private static System.Timers.Timer? _debounceTimer;
    private static readonly object _lock = new();
    private static McpServer? _mcpServer;

    /// <summary>
    /// Start watching the assets directory for JSON file changes.
    /// RTM-011: Monitor ~/maenifold/assets/ with *.json filter
    /// RTM-012: IncludeSubdirectories = true
    /// RTM-013: NotifyFilter = FileName | LastWrite
    /// RTM-014: Debounce delay = 150ms (Config.DefaultDebounceMs)
    /// RTM-016: InternalBufferSize = 64KB (Config.WatcherBufferSize)
    /// Wave 4: Register MCP server for resource change notifications
    /// </summary>
    [McpServerTool]
    [Description("Start watching assets directory for JSON file changes with debouncing")]
    public static string StartAssetWatcher(McpServer mcpServer)
    {
        lock (_lock)
        {
            if (_watcher != null)
                return "Asset watcher already running";

            // Store MCP server reference for Wave 4: Resource update notifications (RTM-017 to RTM-020)
            _mcpServer = mcpServer;

            // Ensure the assets directory exists before creating the watcher
            Directory.CreateDirectory(Config.AssetsPath);

            // RTM-011: Monitor assets directory with *.json filter
            // RTM-012: IncludeSubdirectories = true for nested asset folders
            // RTM-013: NotifyFilter = FileName | LastWrite for relevant changes
            // RTM-016: InternalBufferSize = 64KB (Config.WatcherBufferSize)
            _watcher = new FileSystemWatcher
            {
                Path = Config.AssetsPath,
                Filter = "*.json",
                IncludeSubdirectories = true,
                NotifyFilter = NotifyFilters.FileName | NotifyFilters.LastWrite,
                InternalBufferSize = Config.WatcherBufferSize,
                EnableRaisingEvents = false  // Will enable after handler registration
            };

            // RTM-015: Register event handlers for all 5 event types
            _watcher.Created += OnFileCreated;
            _watcher.Changed += OnFileChanged;
            _watcher.Deleted += OnFileDeleted;
            _watcher.Renamed += OnFileRenamed;
            _watcher.Error += OnWatcherError;

            // RTM-014: Debounce timer with 150ms interval (Config.DefaultDebounceMs)
            _debounceTimer = new System.Timers.Timer(Config.DefaultDebounceMs)
            {
                AutoReset = false
            };
            _debounceTimer.Elapsed += OnDebounceElapsed;

            // Enable raising events after all handlers are registered
            _watcher.EnableRaisingEvents = true;

            return $"Started watching {Config.AssetsPath} for asset changes (debounce: {Config.DefaultDebounceMs}ms)";
        }
    }

    /// <summary>
    /// Stop watching the assets directory.
    /// </summary>
    [McpServerTool]
    [Description("Stop watching assets directory")]
    public static string StopAssetWatcher()
    {
        lock (_lock)
        {
            if (_watcher == null)
                return "Asset watcher not running";

            _watcher.EnableRaisingEvents = false;
            _watcher.Dispose();
            _watcher = null;

            _debounceTimer?.Dispose();
            _debounceTimer = null;

            return "Stopped asset watcher";
        }
    }

    /// <summary>
    /// Handler for file creation events.
    /// Wave 4 will implement resource addition to MCP server.
    /// </summary>
    private static void OnFileCreated(object sender, FileSystemEventArgs e)
    {
        // MA PROTOCOL: No fake AI - don't make decisions about what changed
        // Just restart debounce timer to batch changes
        RestartDebounceTimer();
    }

    /// <summary>
    /// Handler for file modification events.
    /// Wave 4 will implement resource update in MCP server.
    /// </summary>
    private static void OnFileChanged(object sender, FileSystemEventArgs e)
    {
        RestartDebounceTimer();
    }

    /// <summary>
    /// Handler for file deletion events.
    /// Wave 4 will implement resource removal from MCP server.
    /// </summary>
    private static void OnFileDeleted(object sender, FileSystemEventArgs e)
    {
        RestartDebounceTimer();
    }

    /// <summary>
    /// Handler for file rename events.
    /// Wave 4 will implement old removal + new addition to MCP server.
    /// </summary>
    private static void OnFileRenamed(object sender, RenamedEventArgs e)
    {
        RestartDebounceTimer();
    }

    /// <summary>
    /// Handler for FileSystemWatcher errors.
    /// MA PROTOCOL: NO FAKE AI - errors propagate, no recovery logic.
    /// This ensures the LLM has complete information to make decisions.
    /// </summary>
    private static void OnWatcherError(object sender, ErrorEventArgs e)
    {
        // NO FAKE AI: Let error propagate with complete information
        var exception = e.GetException() ?? new IOException("FileSystemWatcher encountered unknown error");
        throw exception;
    }

    /// <summary>
    /// Restart the debounce timer to batch asset changes.
    /// </summary>
    private static void RestartDebounceTimer()
    {
        lock (_lock)
        {
            _debounceTimer?.Stop();
            _debounceTimer?.Start();
        }
    }

    /// <summary>
    /// Called when debounce timer expires.
    /// Wave 4: RTM-017 to RTM-020 - Send resource list changed notification to MCP clients
    /// MA PROTOCOL: Simple, direct notification. Let MCP SDK handle resource refresh.
    /// </summary>
    private static void OnDebounceElapsed(object? sender, ElapsedEventArgs e)
    {
        lock (_lock)
        {
            if (_mcpServer == null)
            {
                // MA PROTOCOL: NO FAKE AI - Let error propagate
                // If _mcpServer is null, something went wrong during initialization
                // The LLM needs to know this happened
                throw new InvalidOperationException("MCP server not initialized in AssetWatcherTools");
            }

            // Wave 4: RTM-017, RTM-018, RTM-019, RTM-020
            // Send notifications/resources/list_changed to notify MCP clients
            // Clients automatically call ListResources to get updated asset list
            // MA PROTOCOL: Let async operation complete - errors propagate naturally
            _mcpServer.SendNotificationAsync<object>(
                "notifications/resources/list_changed",
                new object(),
                null,
                CancellationToken.None
            );
        }
    }
}
