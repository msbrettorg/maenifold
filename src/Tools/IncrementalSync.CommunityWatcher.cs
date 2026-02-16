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
            _communityWriteInProgress = false;
        }
    }
}
