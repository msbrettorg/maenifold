namespace Maenifold.Utils;

public static class Config
{
    private static readonly object OverrideLock = new();

    public static readonly string UserHome = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);

    private static string? _rootOverride;
    private static string? _databasePathOverride;
    private static string? _testMemoryPathOverride;

    private static string DefaultRoot => GetEnvString("MAENIFOLD_ROOT", Path.Combine(UserHome, "maenifold"));

    private static string GetRoot()
    {
        lock (OverrideLock)
        {
            return _rootOverride ?? DefaultRoot;
        }
    }

    public static string MaenifoldRoot => GetRoot();

    public static string MemoryPath => Path.Combine(MaenifoldRoot, "memory");

    public static string AssetsPath => Path.Combine(MaenifoldRoot, "assets");

    public static string ThinkingPath => Path.Combine(MemoryPath, "thinking");

    public static string SequentialPath => Path.Combine(ThinkingPath, "sequential");

    public static string DatabasePath
    {
        get
        {
            lock (OverrideLock)
            {
                var defaultPath = Path.Combine(MaenifoldRoot, "memory.db");
                return _databasePathOverride ?? GetEnvString("MAENIFOLD_DATABASE_PATH", defaultPath);
            }
        }
    }

    // RTM: CONNECTION-POOLING-FIX-001 - Disable connection pooling to prevent lock contention
    public static string DatabaseConnectionString => $"Data Source={DatabasePath};Pooling=false";

    public static void SetDatabasePath(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
            throw new ArgumentException("Database path cannot be empty.", nameof(path));

        lock (OverrideLock)
        {
            _databasePathOverride = Path.GetFullPath(path);
        }
    }

    public static string TestMemoryPath
    {
        get
        {
            lock (OverrideLock)
            {
                if (_testMemoryPathOverride != null)
                    return _testMemoryPathOverride;
            }

            var envValue = Environment.GetEnvironmentVariable("MAENIFOLD_TEST_MEMORY");
            if (!string.IsNullOrWhiteSpace(envValue))
                return envValue!;

            return Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", "..", "test-outputs", "memory");
        }
    }

    public static string TestDatabasePath => Path.Combine(Path.GetDirectoryName(TestMemoryPath)!, "test-memory.db");

    public static readonly int DefaultDebounceMs = GetEnvInt("MAENIFOLD_DEBOUNCE_MS", 150);
    public static readonly int IncrementalOptimizeEvery = Math.Max(1, GetEnvInt("MAENIFOLD_INCREMENTAL_OPTIMIZE_EVERY", 40));
    public static readonly int IncrementalVacuumMinutes = GetEnvInt("MAENIFOLD_INCREMENTAL_VACUUM_MINUTES", 720);

    public static readonly bool EnableIncrementalSync = GetEnvBool("MAENIFOLD_AUTO_SYNC", true);
    public static readonly int WatcherBufferSize = GetEnvInt("MAENIFOLD_WATCHER_BUFFER", 65536);

    public static readonly int RecentActivitySnippetLength = GetEnvInt("MAENIFOLD_SNIPPET_LENGTH", 1000);

    public static readonly int SqliteBusyTimeoutMs = GetEnvInt("MAENIFOLD_SQLITE_BUSY_TIMEOUT", 5000);

    public static readonly int SessionAbandonmentMinutes = GetEnvInt("MAENIFOLD_SESSION_ABANDON_MINUTES", 30);
    public static readonly bool EnableSessionCleanup = GetEnvBool("MAENIFOLD_SESSION_CLEANUP", true);

    public static readonly bool EnableSyncLogging = GetEnvBool("MAENIFOLD_SYNC_LOGGING", true);

    // T-GRAPH-DECAY-001.2: RTM NFR-7.5.1 - Sequential thinking grace period
    public static readonly int DecayGraceDaysSequential = GetEnvInt("MAENIFOLD_DECAY_GRACE_DAYS_SEQUENTIAL", 7);

    // T-GRAPH-DECAY-001.2a: RTM NFR-7.5.1a - Workflows grace period
    public static readonly int DecayGraceDaysWorkflows = GetEnvInt("MAENIFOLD_DECAY_GRACE_DAYS_WORKFLOWS", 14);

    // T-GRAPH-DECAY-001.3: RTM NFR-7.5.2 - Default grace period
    public static readonly int DecayGraceDaysDefault = GetEnvInt("MAENIFOLD_DECAY_GRACE_DAYS_DEFAULT", 28);

    // T-GRAPH-DECAY-001.4: RTM NFR-7.5.3 - Half-life
    public static readonly int DecayHalfLifeDays = GetEnvInt("MAENIFOLD_DECAY_HALF_LIFE_DAYS", 30);

    // T-GRAPH-DECAY-001.6: RTM NFR-7.5.5 - Decay function (power-law default per ACT-R, exponential opt-in)
    public static readonly string DecayFunction = GetEnvString("MAENIFOLD_DECAY_FUNCTION", "power-law");

    // T-COMMUNITY-001.3: RTM NFR-13.11.1 - Louvain resolution parameter (gamma)
    public static readonly double LouvainGamma = GetEnvDouble("MAENIFOLD_LOUVAIN_GAMMA", 1.0);

    // T-COMMUNITY-001.3: RTM NFR-13.9.1 - Community sibling thresholds
    public static readonly int CommunitySiblingMinSharedNeighbors = GetEnvInt("MAENIFOLD_COMMUNITY_MIN_SHARED", 3);
    public static readonly double CommunitySiblingMinOverlap = GetEnvDouble("MAENIFOLD_COMMUNITY_MIN_OVERLAP", 0.4);
    public static readonly int CommunitySiblingMaxResults = GetEnvInt("MAENIFOLD_COMMUNITY_MAX_SIBLINGS", 10);

    // T-COMMUNITY-001.3: RTM NFR-13.5.1 - DB watcher debounce for community recomputation
    public static readonly int CommunityWatcherDebounceMs = GetEnvInt("MAENIFOLD_COMMUNITY_DEBOUNCE_MS", 2000);

    public static bool EnableEmbeddingLogs => GetEnvBool("MAENIFOLD_EMBEDDING_LOGS", false);
    public static bool EnableVectorSearchLogs => GetEnvBool("MAENIFOLD_VECTOR_LOGS", false);

    public static void OverrideRoot(string rootPath, bool overrideDatabase = true)
    {
        if (string.IsNullOrWhiteSpace(rootPath))
            throw new ArgumentException("Root path cannot be empty.", nameof(rootPath));

        var fullPath = Path.GetFullPath(rootPath);

        lock (OverrideLock)
        {
            _rootOverride = fullPath;
            if (overrideDatabase)
            {
                _databasePathOverride = Path.Combine(fullPath, "memory.db");
            }
            _testMemoryPathOverride = Path.Combine(fullPath, "test-memory", "memory");
        }
    }

    public static void ResetOverrides()
    {
        lock (OverrideLock)
        {
            _rootOverride = null;
            _databasePathOverride = null;
            _testMemoryPathOverride = null;
        }
    }

    public static void EnsureDirectories()
    {
        Directory.CreateDirectory(MaenifoldRoot);
        Directory.CreateDirectory(MemoryPath);
        Directory.CreateDirectory(AssetsPath);
    }

    public static string GetConfigSummary()
    {
        return $@"Maenifold Configuration:
  Memory Path: {MemoryPath}
  Database: {DatabasePath}
  Debounce: {DefaultDebounceMs}ms
  Auto Sync: {EnableIncrementalSync}
  Decay Config:
    Grace Days (Sequential): {DecayGraceDaysSequential}
    Grace Days (Workflows): {DecayGraceDaysWorkflows}
    Grace Days (Default): {DecayGraceDaysDefault}
    Half-Life Days: {DecayHalfLifeDays}
    Function: {DecayFunction}
  Community Detection:
    Louvain Gamma: {LouvainGamma}
    Sibling Min Shared Neighbors: {CommunitySiblingMinSharedNeighbors}
    Sibling Min Overlap: {CommunitySiblingMinOverlap}
    Sibling Max Results: {CommunitySiblingMaxResults}
    Watcher Debounce: {CommunityWatcherDebounceMs}ms";
    }

    private static string GetEnvString(string name, string defaultValue)
    {
        var value = Environment.GetEnvironmentVariable(name);
        return string.IsNullOrWhiteSpace(value) ? defaultValue : value;
    }

    private static int GetEnvInt(string name, int defaultValue)
    {
        var value = Environment.GetEnvironmentVariable(name);
        return int.TryParse(value, out var result) ? result : defaultValue;
    }

    // T-COMMUNITY-001.3: RTM NFR-13.11.1 - Double env var parser for gamma and overlap thresholds
    private static double GetEnvDouble(string name, double defaultValue)
    {
        var value = Environment.GetEnvironmentVariable(name);
        return double.TryParse(value, System.Globalization.NumberStyles.Float,
            System.Globalization.CultureInfo.InvariantCulture, out var result) ? result : defaultValue;
    }

    private static bool GetEnvBool(string name, bool defaultValue)
    {
        var value = Environment.GetEnvironmentVariable(name);
        if (string.IsNullOrWhiteSpace(value)) return defaultValue;

        return value.ToLowerInvariant() switch
        {
            "true" or "1" or "yes" or "on" => true,
            "false" or "0" or "no" or "off" => false,
            _ => defaultValue
        };
    }
}
