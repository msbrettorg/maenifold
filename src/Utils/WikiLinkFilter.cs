using System.Text.Json;

namespace Maenifold.Utils;

// T-WLFILTER-001.1: WikiLink filter for blocking hub/ephemeral concepts from the knowledge graph
public static class WikiLinkFilter
{
    private static readonly object FilterLock = new();
    private static DateTime _lastMtime;
    private static Dictionary<string, string> _filterEntries = new(StringComparer.OrdinalIgnoreCase);

    public static string FilterPath => Path.Combine(Config.MemoryPath, ".wikilink-filter.json");

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

            try
            {
                var json = File.ReadAllText(path);
                var dict = JsonSerializer.Deserialize<Dictionary<string, string>>(json);
                if (dict != null)
                {
                    foreach (var kvp in dict)
                        entries[kvp.Key] = kvp.Value;
                }
            }
            catch (JsonException)
            {
                // Malformed JSON — treat as empty filter
            }

            _filterEntries = entries;
            return _filterEntries;
        }
    }
}
