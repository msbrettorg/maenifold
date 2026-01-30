using ModelContextProtocol.Server;
using System.ComponentModel;
using System.Globalization;
using System.Text;
using Maenifold.Utils;

namespace Maenifold.Tools;

[McpServerToolType]
public class AssumptionLedgerTools
{
    [McpServerTool, Description(@"Declare, update, or query assumptions with minimal structure.
Captures agent reasoning without auto-inference or scoring.
Integrates with SequentialThinking and Workflow sessions via context references.
Returns declarative next-step suggestions aligned with Ma Protocol restraint.")]
    public static string AssumptionLedger(
        [Description("Action: append (create new), update (modify existing), or read (view existing)")] string action,
        [Description("Assumption statement")] string? assumption = null,
        [Description("Context reference: workflow://thinking/session-ID or sequential session ID")] string? context = null,
        [Description("Planned validation method")] string? validationPlan = null,
        [Description("Current confidence: free text (e.g., 'high', 'medium', 'low', 'needs-verification')")] string? confidence = null,
        [Description("Agent-supplied [[WikiLink]] tags for knowledge graph integration")] string[]? concepts = null,
        [Description("Memory URI for update/read actions (e.g., memory://assumptions/2025/01/assumption-1234567890)")] string? uri = null,
        [Description("Status update: validated, invalidated, or refined")] string? status = null,
        [Description("Additional notes for updates")] string? notes = null,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn) return ToolHelpers.GetLearnContent(nameof(AssumptionLedger));

        try
        {
            return action.ToLowerInvariant() switch
            {
                "append" => AppendAssumption(assumption!, context, validationPlan, confidence, concepts),
                "update" => UpdateAssumption(uri!, status, notes, validationPlan, confidence),
                "read" => ReadAssumption(uri!),
                _ => "ERROR: Invalid action. Use 'append', 'update', or 'read'."
            };
        }
        catch (Exception ex)
        {
            return $"ERROR: {ex.Message}";
        }
    }

    private static string AppendAssumption(string assumption, string? context, string? validationPlan,
        string? confidence, string[]? concepts)
    {
        var validationError = AssumptionLedgerValidation.ValidateAppendInput(assumption, concepts);
        if (validationError != null)
            return validationError;

        var fullPath = GenerateAssumptionPath();
        var dir = Path.GetDirectoryName(fullPath);
        if (dir != null)
            Directory.CreateDirectory(dir);

        var date = DateTimeOffset.UtcNow.UtcDateTime;
        var frontmatter = BuildAppendFrontmatter(date, context, validationPlan, confidence);
        var content = BuildAppendContent(assumption, validationPlan, concepts!);

        MarkdownIO.WriteMarkdown(fullPath, frontmatter, content);

        var memoryUri = MarkdownIO.PathToUri(fullPath, Config.MemoryPath);
        var response = BuildAppendResponse(memoryUri, assumption, confidence, context, validationPlan);

        return response;
    }

    private static string UpdateAssumption(string uri, string? status, string? notes,
        string? validationPlan, string? confidence)
    {
        if (string.IsNullOrWhiteSpace(uri))
            return "ERROR: URI is required for 'update' action.";

        var statusValidationError = status != null ? AssumptionLedgerValidation.ValidateStatus(status) : null;
        if (statusValidationError != null)
            return statusValidationError;

        try
        {
            var path = MarkdownIO.UriToPath(uri, Config.MemoryPath);

            if (!File.Exists(path))
                return $"ERROR: Assumption not found at {uri}";

            var (frontmatter, content, _) = MarkdownIO.ReadMarkdown(path);

            if (frontmatter == null)
                frontmatter = new Dictionary<string, object>();

            BuildFrontmatterUpdates(frontmatter, status, confidence, validationPlan);

            var updatedContent = AppendUpdateNotes(content, notes);

            MarkdownIO.WriteMarkdown(path, frontmatter, updatedContent);

            var response = BuildUpdateResponse(uri, status, confidence);

            return response;
        }
        catch (ArgumentException ex)
        {
            return $"ERROR: {ex.Message}";
        }
    }

    private static string ReadAssumption(string uri)
    {
        if (string.IsNullOrWhiteSpace(uri))
            return "ERROR: URI is required for 'read' action.";

        try
        {
            var path = MarkdownIO.UriToPath(uri, Config.MemoryPath);

            if (!File.Exists(path))
                return $"ERROR: Assumption not found at {uri}";

            var (frontmatter, content, _) = MarkdownIO.ReadMarkdown(path);

            var response = BuildReadResponse(uri, frontmatter, content);

            return response;
        }
        catch (ArgumentException ex)
        {
            return $"ERROR: {ex.Message}";
        }
    }

    private static string GenerateAssumptionPath()
    {
        var timestamp = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        var date = DateTimeOffset.UtcNow.UtcDateTime;
        var year = date.ToString("yyyy", CultureInfo.InvariantCulture);
        var month = date.ToString("MM", CultureInfo.InvariantCulture);
        var fileName = $"assumption-{timestamp}.md";
        var relativePath = Path.Combine("assumptions", year, month, fileName);
        var fullPath = Path.Combine(Config.MemoryPath, relativePath);

        return fullPath;
    }

    private static Dictionary<string, object> BuildAppendFrontmatter(
        DateTime date, string? context, string? validationPlan, string? confidence)
    {
        var frontmatter = new Dictionary<string, object>
        {
            ["created"] = date.ToString("o", CultureInfo.InvariantCulture),
            ["status"] = "active",
            ["confidence"] = confidence ?? "unspecified"
        };

        if (!string.IsNullOrWhiteSpace(context))
            frontmatter["context"] = context;

        if (!string.IsNullOrWhiteSpace(validationPlan))
            frontmatter["validation_plan"] = validationPlan;

        return frontmatter;
    }

    private static string BuildAppendContent(string assumption, string? validationPlan, string[] concepts)
    {
        var content = new StringBuilder();
        content.AppendLine(CultureInfo.InvariantCulture, $"# Assumption: {assumption}");
        content.AppendLine();
        content.AppendLine("## Statement");
        content.AppendLine();
        content.AppendLine(assumption);
        content.AppendLine();

        if (!string.IsNullOrWhiteSpace(validationPlan))
        {
            content.AppendLine("## Validation Plan");
            content.AppendLine();
            content.AppendLine(validationPlan);
            content.AppendLine();
        }

        content.AppendLine("## Related Concepts");
        content.AppendLine();
        foreach (var concept in concepts)
        {
            content.AppendLine(CultureInfo.InvariantCulture, $"- [[{concept}]]");
        }

        return content.ToString();
    }

    private static string BuildAppendResponse(
        string memoryUri, string assumption, string? confidence, string? context, string? validationPlan)
    {
        var response = new StringBuilder();
        response.AppendLine(CultureInfo.InvariantCulture, $"✅ Assumption recorded at {memoryUri}");
        response.AppendLine();
        response.AppendLine(CultureInfo.InvariantCulture, $"**Statement**: {assumption}");
        response.AppendLine(CultureInfo.InvariantCulture, $"**Confidence**: {confidence ?? "unspecified"}");
        response.AppendLine("**Status**: active");

        if (!string.IsNullOrWhiteSpace(context))
            response.AppendLine(CultureInfo.InvariantCulture, $"**Context**: {context}");

        response.AppendLine();
        response.AppendLine("💡 Next: Consider running Sync() to index this assumption into the knowledge graph for SearchMemories discovery.");

        if (!string.IsNullOrWhiteSpace(validationPlan))
        {
            response.AppendLine(CultureInfo.InvariantCulture, $"💡 Remember to update status after completing: {validationPlan}");
        }

        return response.ToString();
    }

    private static void BuildFrontmatterUpdates(
        Dictionary<string, object> frontmatter, string? status, string? confidence, string? validationPlan)
    {
        if (!string.IsNullOrWhiteSpace(status))
            frontmatter["status"] = status;

        if (!string.IsNullOrWhiteSpace(confidence))
            frontmatter["confidence"] = confidence;

        if (!string.IsNullOrWhiteSpace(validationPlan))
            frontmatter["validation_plan"] = validationPlan;

        frontmatter["updated"] = DateTimeOffset.UtcNow.UtcDateTime.ToString("o", CultureInfo.InvariantCulture);
    }

    private static string AppendUpdateNotes(string content, string? notes)
    {
        if (!string.IsNullOrWhiteSpace(notes))
        {
            var timestamp = DateTimeOffset.UtcNow.UtcDateTime.ToString("yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture);
            content += $"\n## Update: {timestamp}\n\n{notes}\n";
        }

        return content;
    }

    private static string BuildUpdateResponse(string uri, string? status, string? confidence)
    {
        var response = new StringBuilder();
        response.AppendLine(CultureInfo.InvariantCulture, $"✅ Assumption updated at {uri}");

        if (!string.IsNullOrWhiteSpace(status))
            response.AppendLine(CultureInfo.InvariantCulture, $"**New Status**: {status}");

        if (!string.IsNullOrWhiteSpace(confidence))
            response.AppendLine(CultureInfo.InvariantCulture, $"**New Confidence**: {confidence}");

        response.AppendLine();
        response.AppendLine("💡 Next: Run Sync() to update the knowledge graph with these changes.");

        return response.ToString();
    }

    private static string BuildReadResponse(
        string uri, Dictionary<string, object>? frontmatter, string content)
    {
        var response = new StringBuilder();
        response.AppendLine(CultureInfo.InvariantCulture, $"📋 **Assumption**: {uri}");
        response.AppendLine();

        if (frontmatter != null)
        {
            if (frontmatter.TryGetValue("status", out var status))
                response.AppendLine(CultureInfo.InvariantCulture, $"**Status**: {status}");

            if (frontmatter.TryGetValue("confidence", out var confidence))
                response.AppendLine(CultureInfo.InvariantCulture, $"**Confidence**: {confidence}");

            if (frontmatter.TryGetValue("created", out var created))
                response.AppendLine(CultureInfo.InvariantCulture, $"**Created**: {created}");

            if (frontmatter.TryGetValue("updated", out var updated))
                response.AppendLine(CultureInfo.InvariantCulture, $"**Updated**: {updated}");

            if (frontmatter.TryGetValue("context", out var ctx))
                response.AppendLine(CultureInfo.InvariantCulture, $"**Context**: {ctx}");

            if (frontmatter.TryGetValue("validation_plan", out var plan))
                response.AppendLine(CultureInfo.InvariantCulture, $"**Validation Plan**: {plan}");

            response.AppendLine();
        }

        response.AppendLine("## Content");
        response.AppendLine();
        response.AppendLine(content);

        return response.ToString();
    }
}
