using Maenifold.Utils;
using Microsoft.Data.Sqlite;

namespace Maenifold.Tools;

// T-SYNC-UNIFY-001.4: Delegates all processing to ConceptSync unified API
public partial class IncrementalSyncTools
{
    private static void SyncFile(string fullPath)
    {
        try
        {
            ConceptSync.SyncFiles(new[] { fullPath });
            LogSync($"Incremental sync applied for {PathToUri(fullPath)}.");
            ScheduleMaintenanceIfNeeded();
        }
        catch (Exception ex)
        {
            LogSync($"Failed to process incremental sync for '{fullPath}'.", ex);
        }
    }

    private static void ProcessFileCreated(string fullPath) => SyncFile(fullPath);

    private static void ProcessFileChanged(string fullPath) => SyncFile(fullPath);

    private static void ProcessFileDeleted(string fullPath)
    {
        try
        {
            var memoryUri = PathToUri(fullPath);

            using var conn = new SqliteConnection(Config.DatabaseConnectionString);
            conn.OpenWithWAL();

            var vectorReady = ConceptSyncVectorSupport.TryEnsureVectorSupport(conn);
            ConceptSync.RemoveFile(conn, memoryUri, vectorReady);

            LogSync($"Incremental sync removed {memoryUri}.");
            ScheduleMaintenanceIfNeeded();
        }
        catch (Exception ex)
        {
            LogSync($"Failed to process deletion for '{fullPath}'.", ex);
        }
    }

    private static void ProcessFileRenamed(string oldPath, string newPath)
    {
        try
        {
            ProcessFileDeleted(oldPath);
            SyncFile(newPath);
        }
        catch (Exception ex)
        {
            LogSync($"Failed to process rename from '{oldPath}' to '{newPath}'.", ex);
        }
    }
}
