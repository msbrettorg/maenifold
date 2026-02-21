using System.Globalization;
using System.Text;
using System.Text.Json;
using Maenifold.Utils;

namespace Maenifold.Tools;

internal static partial class WorkflowOperations
{
    private static string[] ReadQueue(Dictionary<string, object> frontmatter)
    {
        var queueObj = frontmatter["queue"];
        if (queueObj is object[] objArray)
            return objArray.Select(x => x?.ToString() ?? "").ToArray();
        if (queueObj is List<object> list)
            return list.Select(x => x?.ToString() ?? "").ToArray();
        return Array.Empty<string>();
    }

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
        var queue = new List<string>(ReadQueue(frontmatter));


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


        var appendContent = $"Added workflows to queue: {string.Join(", ", toAppend)}\n*{DateTime.UtcNow.ToString("yyyy-MM-dd HH:mm:ss' UTC'", CultureInfo.InvariantCulture)}*";
        MarkdownIO.AppendToSession("workflow", sessionId, "Queue Updated", appendContent);

        return $"Added {toAppend.Count} workflow(s) to queue. New queue: [{string.Join(", ", queue)}]";
    }

    public static string Continue(string sessionId, string response, string? thoughts, string? status, string? conclusion = null, string? submachineSessionId = null)
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

        // --- Gate check: if workflow is waiting on a submachine, block or clear ---
        frontmatter.TryGetValue("phase", out var phaseObj);
        var phase = phaseObj?.ToString() ?? "running";
        if (phase == "waiting")
        {
            var activeSubmachineSessionId = frontmatter.TryGetValue("activeSubmachineSessionId", out var activeIdObj)
                ? activeIdObj?.ToString() ?? ""
                : "";
            var activeSubmachineType = frontmatter.TryGetValue("activeSubmachineType", out var activeTypeObj)
                ? activeTypeObj?.ToString() ?? ""
                : "";

            if (!string.IsNullOrEmpty(activeSubmachineSessionId) && !string.IsNullOrEmpty(activeSubmachineType))
            {
                if (!MarkdownIO.SessionExists(activeSubmachineType, activeSubmachineSessionId))
                {
                    // Session file gone — treat as terminal state and clear waiting
                    frontmatter["phase"] = "running";
                    frontmatter["activeSubmachineType"] = "";
                    frontmatter["activeSubmachineSessionId"] = "";
                    MarkdownIO.UpdateSession("workflow", sessionId, frontmatter, existingContent);
                    MarkdownIO.AppendToSession("workflow", sessionId, "Submachine Cleared",
                        $"Registered submachine '{activeSubmachineSessionId}' session not found (deleted or expired). " +
                        $"Clearing waiting state and resuming workflow.\n\n" +
                        $"*{DateTime.UtcNow.ToString("yyyy-MM-dd HH:mm:ss' UTC'", CultureInfo.InvariantCulture)}*");
                    var (clearedFm, clearedContent, _) = MarkdownIO.ReadSession("workflow", sessionId);
                    frontmatter = clearedFm ?? frontmatter;
                    existingContent = clearedContent;
                    // Fall through to normal step advancement below
                }
                else
                {
                    var (submachineFrontmatter, _, _) = MarkdownIO.ReadSession(activeSubmachineType, activeSubmachineSessionId);
                    var submachineStatus = submachineFrontmatter != null &&
                        submachineFrontmatter.TryGetValue("status", out var submachineStatusObj)
                        ? submachineStatusObj?.ToString() ?? "active"
                        : "active";

                    if (submachineStatus is "completed" or "cancelled" or "abandoned")
                    {
                        // Submachine reached terminal state — clear supervisor fields and resume
                        frontmatter["phase"] = "running";
                        frontmatter["activeSubmachineType"] = "";
                        frontmatter["activeSubmachineSessionId"] = "";
                        var (_, contentAfterClear, _) = MarkdownIO.ReadSession("workflow", sessionId);
                        MarkdownIO.UpdateSession("workflow", sessionId, frontmatter, contentAfterClear);
                        MarkdownIO.AppendToSession("workflow", sessionId, "Submachine Resumed",
                            $"Submachine {activeSubmachineSessionId} {submachineStatus}. Resuming workflow.\n\n" +
                            $"*{DateTime.UtcNow.ToString("yyyy-MM-dd HH:mm:ss' UTC'", CultureInfo.InvariantCulture)}*");
                        // Re-read updated content and fall through to normal step advancement
                        var (updatedFm, updatedContent, _) = MarkdownIO.ReadSession("workflow", sessionId);
                        frontmatter = updatedFm ?? frontmatter;
                        existingContent = updatedContent;
                    }
                    else
                    {
                        return $"Workflow is waiting on submachine {activeSubmachineType} {activeSubmachineSessionId}. " +
                               $"Complete it first, then call Workflow again.";
                    }
                }
            }
        }

        // --- Register path: submachineSessionId provided — enter waiting state ---
        if (!string.IsNullOrEmpty(submachineSessionId))
            return HandleRegisterSubmachine(sessionId, frontmatter, response, thoughts, submachineSessionId);

        var currentWorkflowIndex = Convert.ToInt32(frontmatter["currentWorkflow"], CultureInfo.InvariantCulture);
        var currentStep = Convert.ToInt32(frontmatter["currentStep"], CultureInfo.InvariantCulture);

        var queue = ReadQueue(frontmatter);

        var currentWorkflowPath = Path.Combine(WorkflowCommon.WorkflowsPath, $"{queue[currentWorkflowIndex]}.json");
        var currentWorkflow = JsonSerializer.Deserialize<JsonElement>(File.ReadAllText(currentWorkflowPath), SafeJson.Options);
        var steps = currentWorkflow.GetProperty("steps").EnumerateArray().ToList();

        // Build and record response content
        var responseContent = BuildResponseContent(response, thoughts);
        var heading = $"Step {currentStep + 1}/{steps.Count} Response";
        MarkdownIO.AppendToSession("workflow", sessionId, heading, responseContent);

        currentStep++;
        frontmatter["currentStep"] = currentStep;

        var (_, updatedContent2, _) = MarkdownIO.ReadSession("workflow", sessionId);
        existingContent = updatedContent2;

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

    private static string HandleRegisterSubmachine(string sessionId, Dictionary<string, object> frontmatter, string response, string? thoughts, string submachineSessionId)
    {
        // All submachine sessions are sequential thinking for now
        var submachineType = "sequential";

        if (!MarkdownIO.SessionExists(submachineType, submachineSessionId))
        {
            return $"ERROR: Submachine session '{submachineSessionId}' not found under type '{submachineType}'. " +
                   $"Ensure the SequentialThinking session exists before registering.";
        }

        // Persist response content BEFORE writing registration (MEDIUM-001 fix)
        var currentWorkflowIndexForRegister = Convert.ToInt32(frontmatter["currentWorkflow"], CultureInfo.InvariantCulture);
        var currentStepForRegister = Convert.ToInt32(frontmatter["currentStep"], CultureInfo.InvariantCulture);
        var queueForRegister = ReadQueue(frontmatter);

        var currentWorkflowPathForRegister = Path.Combine(WorkflowCommon.WorkflowsPath, $"{queueForRegister[currentWorkflowIndexForRegister]}.json");
        var currentWorkflowForRegister = JsonSerializer.Deserialize<JsonElement>(File.ReadAllText(currentWorkflowPathForRegister), SafeJson.Options);
        var stepsForRegister = currentWorkflowForRegister.GetProperty("steps").EnumerateArray().ToList();
        var responseContentForRegister = BuildResponseContent(response, thoughts);
        var headingForRegister = $"Step {currentStepForRegister + 1}/{stepsForRegister.Count} Response";
        MarkdownIO.AppendToSession("workflow", sessionId, headingForRegister, responseContentForRegister);
        // NOTE: currentStep intentionally NOT advanced (FR-10.4)

        frontmatter["phase"] = "waiting";
        frontmatter["activeSubmachineType"] = submachineType;
        frontmatter["activeSubmachineSessionId"] = submachineSessionId;
        var (_, contentBeforeRegister, _) = MarkdownIO.ReadSession("workflow", sessionId);
        MarkdownIO.UpdateSession("workflow", sessionId, frontmatter, contentBeforeRegister);
        MarkdownIO.AppendToSession("workflow", sessionId, "Submachine Registered",
            $"Submachine registered: {submachineType} {submachineSessionId}\n\n" +
            $"*{DateTime.UtcNow.ToString("yyyy-MM-dd HH:mm:ss' UTC'", CultureInfo.InvariantCulture)}*");

        return $"Registered submachine {submachineSessionId}. Workflow is waiting. " +
               $"Complete the submachine, then call Workflow with sessionId and response to continue.";
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
        content.AppendLine(CultureInfo.InvariantCulture, $"*{DateTime.UtcNow.ToString("yyyy-MM-dd HH:mm:ss' UTC'", CultureInfo.InvariantCulture)}*");

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
