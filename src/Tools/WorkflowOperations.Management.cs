using System.Globalization;
using System.Text;
using System.Text.Json;
using Maenifold.Utils;

namespace Maenifold.Tools;

internal static partial class WorkflowOperations
{
    public static string Append(string sessionId, string append)
    {
        if (!MarkdownIO.SessionExists("workflow", sessionId))
        {
            return $"ERROR: Session {sessionId} doesn't exist";
        }


        var (frontmatter, content, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (frontmatter == null)
        {
            return "ERROR: Invalid session file format";
        }

        // Read existing queue - handle multiple possible types from YAML deserialization
        var queueObj = frontmatter["queue"];
        string[] queueArray;
        if (queueObj is object[] objArray)
            queueArray = objArray.Select(x => x?.ToString() ?? "").ToArray();
        else if (queueObj is List<object> list)
            queueArray = list.Select(x => x?.ToString() ?? "").ToArray();
        else
            queueArray = Array.Empty<string>();

        var queue = new List<string>(queueArray);


        var toAppend = new List<string>();
        if (append.StartsWith('['))
        {
            toAppend = JsonSerializer.Deserialize<List<string>>(append, SafeJson.Options)!;
        }
        else
        {
            toAppend.Add(append);
        }


        foreach (var wf in toAppend)
        {
            var wfPath = Path.Combine(WorkflowCommon.WorkflowsPath, $"{wf}.json");
            if (!File.Exists(wfPath))
            {
                return $"ERROR: Workflow '{wf}' not found";
            }
        }


        queue.AddRange(toAppend);


        frontmatter["queue"] = queue.ToArray();
        MarkdownIO.UpdateSession("workflow", sessionId, frontmatter, content);


        var appendContent = $"Added workflows to queue: {string.Join(", ", toAppend)}\n*{DateTime.UtcNow.ToString("yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture)}*";
        MarkdownIO.AppendToSession("workflow", sessionId, "Queue Updated", appendContent);

        return $"Added {toAppend.Count} workflow(s) to queue. New queue: [{string.Join(", ", queue)}]";
    }

    public static string Continue(string sessionId, string response, string? thoughts, string? status, string? conclusion = null)
    {
        // Validate concepts in response/thoughts
        var conceptError = ValidateConceptsInResponse(response, thoughts);
        if (conceptError != null)
            return conceptError;

        if (!MarkdownIO.SessionExists("workflow", sessionId))
            return $"ERROR: Session {sessionId} doesn't exist";

        var (frontmatter, existingContent, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (frontmatter == null)
            return "ERROR: Invalid session file format";

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
        var currentWorkflow = JsonSerializer.Deserialize<JsonElement>(File.ReadAllText(currentWorkflowPath), SafeJson.Options);
        var steps = currentWorkflow.GetProperty("steps").EnumerateArray().ToList();

        // Build and record response content
        var responseContent = BuildResponseContent(response, thoughts);
        var heading = $"Step {currentStep + 1}/{steps.Count} Response";
        MarkdownIO.AppendToSession("workflow", sessionId, heading, responseContent);

        currentStep++;
        frontmatter["currentStep"] = currentStep;

        var (_, updatedContent, _) = MarkdownIO.ReadSession("workflow", sessionId);
        existingContent = updatedContent;

        // Handle explicit completion or cancellation
        if (status == "completed")
            return HandleCompletedStatus(sessionId, frontmatter, currentStep, conclusion);

        if (status == "cancelled")
            return HandleCancelledStatus(sessionId, frontmatter, existingContent, currentStep);

        // Handle workflow/queue progression
        if (currentStep >= steps.Count)
        {
            currentWorkflowIndex++;
            if (currentWorkflowIndex >= queue.Length)
                return HandleQueueExhaustion(sessionId, frontmatter, currentStep, conclusion);

            return TransitionToNextWorkflow(sessionId, frontmatter, existingContent, currentWorkflowIndex, queue);
        }

        return AdvanceToNextStep(sessionId, frontmatter, existingContent, currentStep, steps);
    }

    private static string? ValidateConceptsInResponse(string response, string? thoughts)
    {
        var responseConcepts = MarkdownIO.ExtractWikiLinks(response);
        var thoughtsConcepts = string.IsNullOrEmpty(thoughts) ? new List<string>() : MarkdownIO.ExtractWikiLinks(thoughts);
        var totalConcepts = responseConcepts.Count + thoughtsConcepts.Count;

        if (totalConcepts == 0)
        {
            return "ERROR: Response or thoughts must contain at least one [[WikiLink]] in double brackets.\n" +
                   "Example: 'Implementing [[REST API]] using [[ASP.NET Core]]'\n" +
                   "This ensures your workflow contributions build the knowledge graph.";
        }

        return null;
    }

    private static string BuildResponseContent(string response, string? thoughts)
    {
        var content = new StringBuilder();
        content.AppendLine(response);

        if (!string.IsNullOrEmpty(thoughts))
        {
            content.AppendLine();
            content.AppendLine(CultureInfo.InvariantCulture, $"*Thoughts: {thoughts}*");
        }

        content.AppendLine();
        content.AppendLine(CultureInfo.InvariantCulture, $"*{DateTime.UtcNow.ToString("yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture)}*");

        return content.ToString();
    }

    private static string HandleCompletedStatus(string sessionId, Dictionary<string, object> frontmatter, int currentStep, string? conclusion)
    {
        if (string.IsNullOrEmpty(conclusion))
            return "ERROR: Conclusion required when completing session. Must synthesize findings with [[WikiLinks]].";

        var conclusionConcepts = MarkdownIO.ExtractWikiLinks(conclusion);
        if (conclusionConcepts.Count == 0)
            return "ERROR: Conclusion must include [[WikiLinks]] for knowledge graph integration.";

        MarkdownIO.AppendToSession("workflow", sessionId, "Conclusion", conclusion);

        frontmatter["status"] = "completed";
        frontmatter["completed"] = DateTime.UtcNow.ToString("o", CultureInfo.InvariantCulture);
        frontmatter["stepCount"] = currentStep;

        var (_, contentAfterConclusion, _) = MarkdownIO.ReadSession("workflow", sessionId);
        MarkdownIO.UpdateSession("workflow", sessionId, frontmatter, contentAfterConclusion);

        return "✅ Workflow session completed";
    }

    private static string HandleCancelledStatus(string sessionId, Dictionary<string, object> frontmatter, string existingContent, int currentStep)
    {
        frontmatter["status"] = "cancelled";
        frontmatter["cancelled"] = DateTime.UtcNow.ToString("o", CultureInfo.InvariantCulture);
        frontmatter["stepCount"] = currentStep;
        MarkdownIO.UpdateSession("workflow", sessionId, frontmatter, existingContent);

        return "❌ Workflow session cancelled";
    }

    private static string HandleQueueExhaustion(string sessionId, Dictionary<string, object> frontmatter, int currentStep, string? conclusion)
    {
        if (string.IsNullOrEmpty(conclusion))
            return "ERROR: All workflows complete. Call workflow tool again with conclusion parameter to finalize.";

        var conclusionConcepts = MarkdownIO.ExtractWikiLinks(conclusion);
        if (conclusionConcepts.Count == 0)
            return "ERROR: Conclusion must include [[WikiLinks]] for knowledge graph integration.";

        MarkdownIO.AppendToSession("workflow", sessionId, "Conclusion", conclusion);

        frontmatter["status"] = "completed";
        frontmatter["completed"] = DateTime.UtcNow.ToString("o", CultureInfo.InvariantCulture);
        frontmatter["stepCount"] = currentStep;

        var (_, contentAfterConclusion, _) = MarkdownIO.ReadSession("workflow", sessionId);
        MarkdownIO.UpdateSession("workflow", sessionId, frontmatter, contentAfterConclusion);

        return "✅ All workflows in queue completed";
    }

    private static string TransitionToNextWorkflow(string sessionId, Dictionary<string, object> frontmatter, string existingContent, int currentWorkflowIndex, string[] queue)
    {
        var currentStep = 0;
        frontmatter["currentWorkflow"] = currentWorkflowIndex;
        frontmatter["currentStep"] = currentStep;
        MarkdownIO.UpdateSession("workflow", sessionId, frontmatter, existingContent);

        var nextWorkflowPath = Path.Combine(WorkflowCommon.WorkflowsPath, $"{queue[currentWorkflowIndex]}.json");
        var nextWorkflow = JsonSerializer.Deserialize<JsonElement>(File.ReadAllText(nextWorkflowPath), SafeJson.Options);
        var nextSteps = nextWorkflow.GetProperty("steps").EnumerateArray().ToList();

        MarkdownIO.AppendToSession("workflow", sessionId, $"Step 1/{nextSteps.Count}", nextSteps[0].GetRawText());
        return $"Step 1/{nextSteps.Count}\n\n{nextSteps[0].GetRawText()}";
    }

    private static string AdvanceToNextStep(string sessionId, Dictionary<string, object> frontmatter, string existingContent, int currentStep, List<JsonElement> steps)
    {
        MarkdownIO.UpdateSession("workflow", sessionId, frontmatter, existingContent);

        var nextStep = steps[currentStep];
        MarkdownIO.AppendToSession("workflow", sessionId, $"Step {currentStep + 1}/{steps.Count}", nextStep.GetRawText());
        return $"Step {currentStep + 1}/{steps.Count}\n\n{nextStep.GetRawText()}";
    }
}
