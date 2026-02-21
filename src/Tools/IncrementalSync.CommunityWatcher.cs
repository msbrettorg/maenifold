using Maenifold.Utils;
using Microsoft.Data.Sqlite;

namespace Maenifold.Tools;

// T-COMMUNITY-001.6: RTM FR-13.5, NFR-13.5.1, NFR-13.5.2, NFR-13.5.3
// DB file watcher triggers community recomputation after incremental sync settles.
public partial class IncrementalSyncTools
{
    private static FileSystemWatcher? _dbWatcher;
    private static Timer? _dbDebounceTimer;
    private static volatile bool _communityWriteInProgress;
    private static long _communityWriteFinishedTicks;

    // T-COMMUNITY-001.11: RTM FAIL-001 remediation — allow ConceptSync.Sync to set the write guard
    // so the DB watcher skips during full sync community detection.
    public static void SetCommunityWriteInProgress(bool value) => _communityWriteInProgress = value;

    // Call from StartWatcher after the existing _watcher setup
    private static void StartDbWatcher()
    {
        var dbPath = Config.DatabasePath;
        var dbDir = Path.GetDirectoryName(dbPath);
        var dbFile = Path.GetFileName(dbPath);

        if (string.IsNullOrEmpty(dbDir) || !Directory.Exists(dbDir))
            return;

        _dbWatcher = new FileSystemWatcher(dbDir)
        {
            Filter = dbFile,
            NotifyFilter = NotifyFilters.LastWrite,
            EnableRaisingEvents = true
        };

        _dbWatcher.Changed += OnDbChanged;
    }

    // Call from StopWatcher to clean up
    private static void StopDbWatcher()
    {
        _dbDebounceTimer?.Dispose();
        _dbDebounceTimer = null;

        if (_dbWatcher != null)
        {
            _dbWatcher.EnableRaisingEvents = false;
            _dbWatcher.Dispose();
            _dbWatcher = null;
        }
    }

    private static void OnDbChanged(object sender, FileSystemEventArgs e)
    {
        // NFR-13.5.3: Skip if we're the ones writing community data
        if (_communityWriteInProgress)
            return;

        // Skip delayed FSW events that arrive after the write guard clears.
        // Hold off for the full debounce window past the write finishing
        // to prevent the write→notify→refire feedback loop.
        var ticksSinceWrite = Environment.TickCount64 - Interlocked.Read(ref _communityWriteFinishedTicks);
        if (ticksSinceWrite < Config.CommunityWatcherDebounceMs)
            return;

        lock (_lock)
        {
            _dbDebounceTimer?.Dispose();
            _dbDebounceTimer = new Timer(_ =>
            {
                RunCommunityDetection();
            }, null, Config.CommunityWatcherDebounceMs, Timeout.Infinite);
        }
    }

    private static void RunCommunityDetection()
    {
        try
        {
            _communityWriteInProgress = true;

            using var conn = new SqliteConnection(Config.DatabaseConnectionString);
            conn.OpenWithWAL();

            var (count, mod) = CommunityDetection.RunAndPersist(conn, Config.LouvainGamma);
            LogSync($"Community recomputation: {count} communities, modularity {mod:F3}");
        }
        catch (Exception ex)
        {
            LogSync("Community recomputation failed.", ex);
        }
        finally
        {
            Interlocked.Exchange(ref _communityWriteFinishedTicks, Environment.TickCount64);
            _communityWriteInProgress = false;
        }
    }
}
