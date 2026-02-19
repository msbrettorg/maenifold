using ModelContextProtocol.Server;
using System.ComponentModel;
using Maenifold.Utils;

namespace Maenifold.Tools;


[McpServerToolType]
public partial class IncrementalSyncTools
{

    private static FileSystemWatcher? _watcher;
    private static readonly object _lock = new();
    private static readonly Dictionary<string, (Timer timer, WatcherChangeTypes changeType, object? eventArgs)> _debounceTimers = new();
    private static int _debounceMs = Config.DefaultDebounceMs;
    private static long _changesSinceOptimize;
    private static DateTime _lastVacuumUtc = DateTime.UtcNow;
    private static readonly int _optimizeThreshold = Config.IncrementalOptimizeEvery;
    private static readonly TimeSpan _vacuumInterval = TimeSpan.FromMinutes(Config.IncrementalVacuumMinutes);


    public static string StartWatcher(
            [Description("Debounce delay in milliseconds (default 150)")] int? debounceMs = null)
    {
        lock (_lock)
        {
            if (_watcher != null)
                return "Watcher already running";

            if (debounceMs.HasValue)
            {
                if (debounceMs.Value < 10 || debounceMs.Value > 10000)
                    return "ERROR: debounceMs must be between 10 and 10000 milliseconds";

                _debounceMs = debounceMs.Value;
            }

            // Ensure the directory exists before creating the watcher
            Directory.CreateDirectory(MemoryPath);

            _watcher = new FileSystemWatcher(MemoryPath)
            {
                Filter = "*.md",
                IncludeSubdirectories = true,
                NotifyFilter = NotifyFilters.FileName | NotifyFilters.LastWrite,
                InternalBufferSize = Config.WatcherBufferSize
            };

            _watcher.Created += OnFileCreated;
            _watcher.Changed += OnFileChanged;
            _watcher.Deleted += OnFileDeleted;
            _watcher.Renamed += OnFileRenamed;
            _watcher.Error += OnWatcherError;

            _watcher.EnableRaisingEvents = true;

            StartDbWatcher();

            return $"Started watching {MemoryPath} for incremental sync (debounce: {_debounceMs}ms)";
        }
    }


    public static string StopWatcher()
    {
        lock (_lock)
        {
            if (_watcher == null)
                return "Watcher not running";

            _watcher.EnableRaisingEvents = false;
            _watcher.Dispose();
            _watcher = null;


            foreach (var entry in _debounceTimers.Values)
            {
                entry.timer?.Dispose();
            }
            _debounceTimers.Clear();

            StopDbWatcher();

            return "Stopped incremental sync watcher";
        }
    }

    private static void ScheduleMaintenanceIfNeeded()
    {
        bool runOptimize = false;
        bool runVacuum = false;

        lock (_lock)
        {
            _changesSinceOptimize++;

            if (_changesSinceOptimize >= _optimizeThreshold)
            {
                _changesSinceOptimize = 0;
                runOptimize = true;
            }

            if (_vacuumInterval > TimeSpan.Zero && DateTime.UtcNow - _lastVacuumUtc >= _vacuumInterval)
            {
                _lastVacuumUtc = DateTime.UtcNow;
                runVacuum = true;
                runOptimize = true;
            }
        }

        if (runOptimize || runVacuum)
        {
            RunMaintenance(runOptimize, runVacuum);
        }
    }
}
