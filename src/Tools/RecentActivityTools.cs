using ModelContextProtocol.Server;
using System.ComponentModel;
using Maenifold.Utils;

namespace Maenifold.Tools;

[McpServerToolType]
public class RecentActivityTools
{
    [McpServerTool, Description(@"Monitors Maenifold system activity with time-based filtering for memory and thinking session tracking.
Select when AI needs to understand recent work patterns, find current sessions, or analyze usage trends.
Requires optional filters for activity type, time span, and result limits for focused monitoring.
Integrates with ReadMemory for session access, SearchMemories for activity analysis, MemoryStatus for system health.
Returns chronological activity list with timestamps, types, and identifiers for session continuation or analysis.")]
    public static string RecentActivity(
        [Description("Max results (default 10)")] int limit = 10,
        [Description("Filter: thinking, memory, chat, or all (default all)")] string? filter = null,
        [Description("Time span for filtering (e.g. 24.00:00:00 for 24 hours). Must be positive.")] TimeSpan? timespan = null,
        [Description("Include full section content (true) or headers only (false, default)")] bool includeContent = false,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn)
        {
            var toolName = nameof(RecentActivity).ToLowerInvariant();
            var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
            if (!File.Exists(helpPath))
                return $"ERROR: Help file not found for {nameof(RecentActivity)}";
            return File.ReadAllText(helpPath);
        }

        try
        {

            if (timespan.HasValue && timespan.Value < TimeSpan.Zero)
            {
                return "ERROR: timespan parameter must be positive. Example: '24.00:00:00' for 24 hours.";
            }


            var results = RecentActivityReader.CollectRecentActivity(timespan, filter, limit);


            return RecentActivityFormatter.FormatActivityReport(results, includeContent);
        }
        catch (InvalidOperationException ex) when (ex.Message.Contains("No database found"))
        {
            return "# Recent Activity\n\nNo database found. Run the `Sync` command first to index memory files.";
        }
        catch (Exception ex)
        {
            return $"ERROR: {ex.Message}";
        }
    }
}
