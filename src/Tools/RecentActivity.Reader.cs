using System.Globalization;
using Microsoft.Data.Sqlite;
using Maenifold.Utils;

namespace Maenifold.Tools;

public static class RecentActivityReader
{
    public static List<(string filePath, string title, string lastIndexed, string content, string status)>
    CollectRecentActivity(TimeSpan? timespan, string? filter, int limit)
    {
        using var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenReadOnly();


        var tableExists = conn.QuerySingle<int>(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='file_content'") > 0;

        if (!tableExists)
        {
            throw new InvalidOperationException("No database found. Run the `Sync` command first to index memory files.");
        }


        var sql = @"
            SELECT file_path, title, last_indexed, content, status 
            FROM file_content 
            WHERE 1=1";

        var parameters = new Dictionary<string, object>();


        if (limit < 0) limit = 0;


        if (!string.IsNullOrEmpty(filter) && filter != "all")
        {
            if (filter == "thinking")
            {
                sql += " AND file_path LIKE @filterPattern";
                parameters["filterPattern"] = $"memory://{Path.GetFileName(Config.ThinkingPath)}/%";
            }
            else if (filter == "chat")
            {
                sql += " AND file_path LIKE @filterPattern";
                parameters["filterPattern"] = "memory://chat/%";
            }
            else if (filter == "memory")
            {
                sql += " AND file_path NOT LIKE @filterPattern AND file_path NOT LIKE @chatPattern";
                parameters["filterPattern"] = $"memory://{Path.GetFileName(Config.ThinkingPath)}/%";
                parameters["chatPattern"] = "memory://chat/%";
            }
        }


        if (timespan.HasValue)
        {
            var cutoffDateTime = DateTime.UtcNow.Subtract(timespan.Value);
            var cutoff = cutoffDateTime.ToString("yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture);
            sql += " AND datetime(last_indexed) >= datetime(@cutoff)";
            parameters["cutoff"] = cutoff;
        }


        sql += " ORDER BY last_indexed DESC LIMIT @limit";
        parameters["limit"] = limit;


        var results = new List<(string filePath, string title, string lastIndexed, string content, string status)>();
        using var cmd = conn.CreateCommand();
        cmd.CommandText = sql;
        foreach (var param in parameters)
        {
            cmd.Parameters.AddWithValue($"@{param.Key}", param.Value);
        }

        using var reader = cmd.ExecuteReader();
        while (reader.Read())
        {
            results.Add((
                reader.GetString(0),
                reader.IsDBNull(1) ? "" : reader.GetString(1),
                reader.GetString(2),
                reader.GetString(3),
                reader.IsDBNull(4) ? "active" : reader.GetString(4)
            ));
        }

        return results;
    }

    public static string ExtractSnippet(string content, int maxLength = -1)
    {

        if (maxLength == -1)
            maxLength = Config.RecentActivitySnippetLength;


        var lines = content.Split('\n', StringSplitOptions.RemoveEmptyEntries);


        var contentLines = lines.Where(l => !l.TrimStart().StartsWithOrdinal("*")).ToList();
        if (contentLines.Count == 0 && lines.Length > 0)
            contentLines = lines.ToList();

        var snippet = string.Join(" ", contentLines).Trim();

        if (snippet.Length > maxLength)
        {
            snippet = string.Concat(snippet.AsSpan(0, maxLength - 3), "...");
        }

        return snippet;
    }

    public static string ExtractSessionId(string filePath)
    {

        var path = filePath.Replace("memory://", "").Replace($"{Path.GetFileName(Config.ThinkingPath)}/", "");
        var segments = path.Split('/');
        return Path.GetFileNameWithoutExtension(segments[^1]);
    }

    public static string DetermineFileType(string filePath)
    {
        if (filePath.ContainsOrdinal($"memory://{Path.GetFileName(Config.ThinkingPath)}/"))
        {
            if (filePath.ContainsOrdinal($"/{Path.GetFileName(Config.SequentialPath)}/"))
                return "sequential";
            else if (filePath.ContainsOrdinal("/workflow/"))
                return "workflow";
            else
                return "thinking";
        }
        return "memory";
    }
}
