using Maenifold.Utils;

namespace Maenifold.Tools;

public static class SessionCleanup
{
    public static void HandleSessionCleanup(Dictionary<string, object?> frontmatter, string filePath, string content)
    {
        var fileType = frontmatter.TryGetValue("type", out var typeValue) ? typeValue?.ToString() : null;
        var status = frontmatter.TryGetValue("status", out var statusValue) ? statusValue?.ToString() : null;

        if ((fileType == "workflow" || fileType == "sequential") && status == "active")
        {
            var modified = frontmatter.TryGetValue("modified", out var modifiedValue) ? modifiedValue?.ToString() : null;
            if (DateTime.TryParse(modified, out var lastModifiedTime))
            {
                var timeSinceUpdate = DateTime.UtcNow - lastModifiedTime.ToUniversalTime();
                if (timeSinceUpdate.TotalMinutes > Config.SessionAbandonmentMinutes)
                {

                    frontmatter["status"] = "abandoned";
                    MarkdownIO.WriteMarkdown(filePath, frontmatter!, content);
                    MarkdownIO.AppendH2Section(filePath, "Session Abandoned",
                        $"⚠️ Session marked as abandoned due to {timeSinceUpdate.TotalMinutes:F0} minutes of inactivity\n*{DateTime.UtcNow:yyyy-MM-dd HH:mm:ss}*");
                }
            }
        }
    }

    // T-CLEANUP-001.1: RTM FR-14.6 - DB-driven abandonment detection for the mtime-guard pre-pass.
    // Uses last_indexed (from DB) as staleness signal instead of frontmatter["modified"].
    // Does NOT write to disk — caller handles persistence and DB status update.
    public static bool TryMarkAbandonedFromLastIndexed(
        Dictionary<string, object> frontmatter,
        string content,
        DateTime lastIndexedUtc,
        DateTime nowUtc,
        out string updatedContent)
    {
        var timeSinceIndexed = nowUtc - lastIndexedUtc;

        if (timeSinceIndexed.TotalMinutes <= Config.SessionAbandonmentMinutes)
        {
            updatedContent = content;
            return false;
        }

        frontmatter["status"] = "abandoned";

        var section = $"\n\n## Session Abandoned\n\n⚠️ Session marked as abandoned due to {timeSinceIndexed.TotalMinutes:F0} minutes of inactivity\n*{CultureInvariantHelpers.FormatDateTime(nowUtc, "yyyy-MM-dd HH:mm:ss")}*\n";
        updatedContent = content + section;
        return true;
    }
}