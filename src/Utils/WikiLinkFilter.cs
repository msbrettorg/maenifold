namespace Maenifold.Utils;

// T-WLFILTER-001.1: WikiLink filter for blocking hub/ephemeral concepts from the knowledge graph
public static class WikiLinkFilter
{
    private static readonly object FilterLock = new();
    private static DateTime _lastMtime;
    private static Dictionary<string, string> _filterEntries = new(StringComparer.OrdinalIgnoreCase);

    public static string FilterPath => Path.Combine(Config.MemoryPath, ".wikilink-filter");

    public static (List<string> blocked, Dictionary<string, string> reasons) CheckFilter(List<string> concepts)
    {
        var blocked = new List<string>();
        var reasons = new Dictionary<string, string>();

        var entries = LoadFilter();
        if (entries.Count == 0)
            return (blocked, reasons);

        foreach (var concept in concepts)
        {
            if (entries.TryGetValue(concept, out var reason))
            {
                blocked.Add(concept);
                reasons[concept] = reason;
            }
        }

        return (blocked, reasons);
    }

    private static Dictionary<string, string> LoadFilter()
    {
        lock (FilterLock)
        {
            var path = FilterPath;
            if (!File.Exists(path))
                return _filterEntries = new(StringComparer.OrdinalIgnoreCase);

            var mtime = File.GetLastWriteTimeUtc(path);
            if (mtime == _lastMtime && _filterEntries.Count > 0)
                return _filterEntries;

            _lastMtime = mtime;
            var entries = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);

            foreach (var line in File.ReadLines(path))
            {
                var trimmed = line.Trim();
                if (trimmed.Length == 0 || trimmed.StartsWith('#'))
                    continue;

                var pipeIndex = trimmed.IndexOf('|');
                if (pipeIndex < 0)
                    continue;

                var concept = trimmed[..pipeIndex].Trim();
                var reason = trimmed[(pipeIndex + 1)..].Trim();

                if (concept.Length > 0)
                    entries[concept] = reason;
            }

            _filterEntries = entries;
            return _filterEntries;
        }
    }
}
