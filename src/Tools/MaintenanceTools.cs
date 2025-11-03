using System.ComponentModel;
using System.Globalization;
using System.Text;
using Maenifold.Utils;
using ModelContextProtocol.Server;

namespace Maenifold.Tools;

[McpServerToolType]
public static class MaintenanceTools
{
    private static string BasePath => Config.MemoryPath;

    [McpServerTool, Description(@"One-time migration: prepend H1 (# {title}) to legacy memory files that lack a top-level H1.
- Dry-run by default: shows what would change without modifying files
- Safe: skips thinking session files and only touches '*.md' under the memory root (or a subfolder)
- Idempotent: re-running after migration makes no further changes
Parameters:
- dryRun: true to preview, false to apply
- limit: maximum files to modify when dryRun=false (default 100)
- folder: optional subfolder under the memory root to restrict scope
- createBackups: optional backup strategy, creates {filename}.bak before modification (default false)")]
    public static string AddMissingH1(
        [Description("Preview changes without modifying files")] bool dryRun = true,
        [Description("Max files to modify when dryRun=false")] int limit = 100,
        [Description("Optional subfolder under memory root to restrict scope")] string? folder = null,
        [Description("Create .bak backup files before modification")] bool createBackups = false,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn)
        {
            var toolName = nameof(AddMissingH1).ToLowerInvariant();
            var helpPath = Path.Combine(Config.AssetsPath, "usage", "tools", $"{toolName}.md");
            if (!File.Exists(helpPath))
                return $"ERROR: Help file not found for {nameof(AddMissingH1)}";
            return File.ReadAllText(helpPath);
        }

        try
        {
            var root = GetScopedRoot(folder);
            if (root == null)
                return $"ERROR: Folder must be inside memory root: {folder}";

            if (!Directory.Exists(root))
                return $"ERROR: Folder not found: {folder ?? "."}";

            var candidates = Directory.EnumerateFiles(root, "*.md", SearchOption.AllDirectories)
                .Where(f => !IsThinkingPath(f))
                .ToList();

            var missing = new List<string>();
            foreach (var file in candidates)
            {
                if (FileLacksTopH1(file))
                    missing.Add(file);
            }

            if (dryRun)
            {
                var sb = new StringBuilder();
                sb.AppendLineInvariant($"Scan complete. Found {CultureInvariantHelpers.ToString(missing.Count)} files missing a top-level H1.");
                foreach (var path in missing.Take(20))
                {
                    var rel = Path.GetRelativePath(BasePath, path);
                    sb.AppendLineInvariant($"  • {rel}");
                }
                if (missing.Count > 20)
                    sb.AppendLineInvariant($"  … and {CultureInvariantHelpers.ToString(missing.Count - 20)} more");

                return ToolResponse.WithNextSteps(sb.ToString(), "Sync");
            }
            else
            {
                if (limit < 0) limit = 0;
                var toProcess = missing.Take(limit).ToList();

                int updated = 0;
                int errors = 0;
                var details = new StringBuilder();

                foreach (var path in toProcess)
                {
                    try
                    {
                        var (frontmatter, content, _) = MarkdownIO.ReadMarkdown(path);
                        var title = DeriveTitle(frontmatter, path);
                        var newContent = $"# {title}\n\n" + content;

                        // Create backup if requested
                        string? backupPath = null;
                        if (createBackups)
                        {
                            backupPath = path + ".bak";
                            try
                            {
                                // Only create backup if it doesn't already exist
                                if (!File.Exists(backupPath))
                                {
                                    File.Copy(path, backupPath, overwrite: false);
                                }
                            }
                            catch (Exception backupEx)
                            {
                                details.AppendLineInvariant($"  ⚠ Warning: Could not create backup for {Path.GetRelativePath(BasePath, path)} — {backupEx.Message}");
                                // Continue with modification even if backup fails
                            }
                        }

                        if (frontmatter != null)
                            frontmatter["modified"] = TimeZoneConverter.GetUtcNowIso();

                        MarkdownIO.WriteMarkdown(path, frontmatter, newContent);

                        var relPath = Path.GetRelativePath(BasePath, path);
                        if (backupPath != null)
                        {
                            details.AppendLineInvariant($"  ✓ Updated: {relPath} (backup: {Path.GetFileName(backupPath)})");
                        }
                        else
                        {
                            details.AppendLineInvariant($"  ✓ Updated: {relPath}");
                        }
                        updated++;
                    }
                    catch (Exception ex)
                    {
                        details.AppendLineInvariant($"  ✗ Error: {Path.GetRelativePath(BasePath, path)} — {ex.Message}");
                        errors++;
                    }
                }

                var summary = $"Migration complete. Updated {CultureInvariantHelpers.ToString(updated)} file(s). Errors: {CultureInvariantHelpers.ToString(errors)}. Remaining without H1: {CultureInvariantHelpers.ToString(Math.Max(0, missing.Count - toProcess.Count))}.";
                if (createBackups && updated > 0)
                {
                    summary += $"\nBackups created with .bak extension. To restore a backup: 'mv file.md.bak file.md'";
                }
                return ToolResponse.WithNextSteps(summary + "\n" + details.ToString(), "Sync");
            }
        }
        catch (Exception ex)
        {
            return $"ERROR: {ex.Message}";
        }
    }

    private static string? GetScopedRoot(string? folder)
    {
        var baseFull = Path.GetFullPath(BasePath);
        var root = string.IsNullOrWhiteSpace(folder)
            ? baseFull
            : Path.GetFullPath(Path.Combine(BasePath, folder));
        if (!root.StartsWith(baseFull, StringComparison.OrdinalIgnoreCase))
            return null;
        return root;
    }

    private static bool IsThinkingPath(string filePath)
    {
        var rel = Path.GetRelativePath(BasePath, filePath);

        var parts = rel.Split(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        return parts.Any(p => p.Equals("thinking", StringComparison.OrdinalIgnoreCase));
    }

    private static bool FileLacksTopH1(string path)
    {
        var (_, content, _) = MarkdownIO.ReadMarkdown(path);
        using var reader = new StringReader(content);
        string? line;
        while ((line = reader.ReadLine()) != null)
        {
            if (string.IsNullOrWhiteSpace(line))
                continue;
            return !line.TrimStart().StartsWith("# ", StringComparison.Ordinal);
        }

        return true;
    }

    private static string DeriveTitle(Dictionary<string, object>? frontmatter, string path)
    {
        if (frontmatter != null && frontmatter.TryGetValue("title", out var t))
        {
            var s = t?.ToString();
            if (!string.IsNullOrWhiteSpace(s))
                return s!;
        }
        var name = Path.GetFileNameWithoutExtension(path)
            .Replace('-', ' ')
            .Replace('_', ' ');
        return CultureInfo.InvariantCulture.TextInfo.ToTitleCase(name);
    }

}
