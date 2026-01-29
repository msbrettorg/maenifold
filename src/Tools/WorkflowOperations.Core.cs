using System.Globalization;
using System.Text;
using System.Text.Json;
using Maenifold.Utils;

namespace Maenifold.Tools;

internal static partial class WorkflowOperations
{
    public static string Start(string workflowId)
    {

        var ids = new List<string>();
        if (workflowId.StartsWith('['))
        {
            // SEC-001: Use safe JSON options with depth limit
            ids = JsonSerializer.Deserialize<List<string>>(workflowId, Maenifold.Utils.SafeJson.Options)!;
        }
        else
        {
            ids.Add(workflowId);
        }


        var sessionId = $"workflow-{DateTimeOffset.UtcNow.ToUnixTimeMilliseconds().ToString(CultureInfo.InvariantCulture)}";


        var firstWorkflowPath = Path.Combine(WorkflowCommon.WorkflowsPath, $"{ids[0]}.json");
        if (!File.Exists(firstWorkflowPath))
        {
            return $"ERROR: Workflow '{ids[0]}' not found";
        }

        // SEC-001: Use safe JSON options with depth limit
        var firstWorkflow = JsonSerializer.Deserialize<JsonElement>(File.ReadAllText(firstWorkflowPath), Maenifold.Utils.SafeJson.Options);
        if (!firstWorkflow.TryGetProperty("steps", out var stepsProperty))
        {
            return $"ERROR: Workflow '{ids[0]}' is missing 'steps' property";
        }

        var steps = stepsProperty.EnumerateArray().ToList();
        if (steps.Count == 0)
        {
            return $"ERROR: Workflow '{ids[0]}' has no steps";
        }


        var content = new StringBuilder();
        content.AppendLine("# Workflow Session");
        content.AppendLine();
        content.AppendLine(CultureInfo.InvariantCulture, $"Started: {DateTime.UtcNow.ToString("yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture)}");
        content.AppendLine();
        content.AppendLine(CultureInfo.InvariantCulture, $"## Step 1/{steps.Count}");
        content.AppendLine();
        content.AppendLine(steps[0].GetRawText());


        var frontmatter = new Dictionary<string, object>
        {
            ["title"] = $"Workflow Session {sessionId}",
            ["permalink"] = sessionId,
            ["type"] = "workflow",
            ["status"] = "active",
            ["queue"] = ids.ToArray(),
            ["currentWorkflow"] = 0,
            ["currentStep"] = 0
        };


        MarkdownIO.CreateSession("workflow", sessionId, frontmatter, content.ToString());

        return $"Created session: {sessionId}\n\nStep 1/{steps.Count}\n\n{steps[0].GetRawText()}";
    }

    public static string View(string sessionId)
    {
        if (!MarkdownIO.SessionExists("workflow", sessionId))
        {
            return $"ERROR: Session {sessionId} doesn't exist";
        }


        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (frontmatter == null)
        {
            return "ERROR: Invalid session file format";
        }

        var currentWorkflowIndex = Convert.ToInt32(frontmatter["currentWorkflow"], CultureInfo.InvariantCulture);
        var currentStep = Convert.ToInt32(frontmatter["currentStep"], CultureInfo.InvariantCulture);


        var queueObj = frontmatter["queue"];
        string[] queue;
        if (queueObj is object[] objArray)
            queue = objArray.Select(x => x?.ToString() ?? "").ToArray();
        else if (queueObj is List<object> list)
            queue = list.Select(x => x?.ToString() ?? "").ToArray();
        else
            queue = Array.Empty<string>();


        var currentWorkflowPath = Path.Combine(WorkflowCommon.WorkflowsPath, $"{queue[currentWorkflowIndex]}.json");
        // SEC-001: Use safe JSON options with depth limit
        var currentWorkflow = JsonSerializer.Deserialize<JsonElement>(File.ReadAllText(currentWorkflowPath), Maenifold.Utils.SafeJson.Options);
        var steps = currentWorkflow.GetProperty("steps").EnumerateArray().ToList();

        var result = new StringBuilder();
        result.AppendLine(CultureInfo.InvariantCulture, $"Queue: [{string.Join(", ", queue)}]");
        result.AppendLine(CultureInfo.InvariantCulture, $"Position: {queue[currentWorkflowIndex]} (workflow {currentWorkflowIndex + 1}/{queue.Length}, step {currentStep + 1}/{steps.Count})");

        if (currentStep + 1 < steps.Count)
        {
            result.AppendLine(CultureInfo.InvariantCulture, $"Next: {steps[currentStep + 1].GetProperty("name").GetString()}");
        }
        else if (currentWorkflowIndex + 1 < queue.Length)
        {
            result.AppendLine(CultureInfo.InvariantCulture, $"Next workflow: {queue[currentWorkflowIndex + 1]}");
        }
        else
        {
            result.AppendLine("Next: Queue will complete");
        }

        return result.ToString();
    }
}
