using Microsoft.Data.Sqlite;
using Maenifold.Utils;

namespace Maenifold.Tools;

public partial class IncrementalSyncTools
{
    private static string MemoryPath => Config.MemoryPath;
    private static string SyncLogPath => Path.Combine(Config.MaenifoldRoot, "logs", "incremental-sync.log");

    private static string PathToUri(string path) => MarkdownIO.PathToUri(path, MemoryPath);

    private static void LogSync(string message, Exception? ex = null)
    {
        if (!Config.EnableSyncLogging)
            return;

        try
        {
            var directory = Path.GetDirectoryName(SyncLogPath);
            if (!string.IsNullOrEmpty(directory))
            {
                Directory.CreateDirectory(directory);
            }

            var line = ex == null
                ? message
                : $"{message}{Environment.NewLine}{ex}";
            var entry = $"[{DateTime.UtcNow:O}] {line}{Environment.NewLine}";
            File.AppendAllText(SyncLogPath, entry);
        }
        catch
        {

        }
    }

    private static void RunMaintenance(bool runOptimize, bool runVacuum)
    {
        try
        {
            using var conn = new SqliteConnection(Config.DatabaseConnectionString);
            conn.OpenWithWAL();

            if (runOptimize)
            {
                conn.Execute("INSERT INTO file_search(file_search) VALUES('optimize')");
            }

            if (runVacuum)
            {
                conn.Execute("INSERT INTO file_search(file_search) VALUES('rebuild')");
                conn.Execute("PRAGMA wal_checkpoint(TRUNCATE)");
                conn.Execute("VACUUM");
                conn.Execute("PRAGMA journal_mode=WAL");
                conn.Execute("PRAGMA synchronous=NORMAL");
            }

            if (runOptimize || runVacuum)
            {
                var message = runOptimize && runVacuum
                    ? "Incremental maintenance: optimized FTS and vacuumed database."
                    : runOptimize
                        ? "Incremental maintenance: optimized FTS index."
                        : "Incremental maintenance: vacuumed database.";
                LogSync(message);
            }
        }
        catch (Exception ex)
        {
            LogSync("Incremental maintenance failed.", ex);
        }
    }
}
