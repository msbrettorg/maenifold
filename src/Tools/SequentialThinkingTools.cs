using ModelContextProtocol.Server;
using System.ComponentModel;
using System.Globalization;
using System.Text;
using System.Text.Json;
using Maenifold.Utils;

namespace Maenifold.Tools;

[McpServerToolType]
public class SequentialThinkingTools
{
    private static readonly int CheckpointFrequency = 3;

    [McpServerTool, Description(@"Creates structured thinking sessions with [[WikiLink]] integration and persistent markdown file storage.
Requires response with [[WikiLinks]], thought tracking, session management, and optional revision capabilities.
Integrates with WriteMemory for session persistence and maenifold tools.
Returns session management with continuation guidance and checkpoint suggestions.")]
    public static string SequentialThinking(
        [Description("Main response/thought - MUST include [[WikiLinks]] to build knowledge")] string? response = null,
        [Description("Need another thought?")] bool nextThoughtNeeded = false,
        [Description("Current thought number")] int thoughtNumber = 0,
        [Description("Total thoughts estimate")] int totalThoughts = 0,
        [Description("Session ID")] string? sessionId = null,
        [Description("Cancel session (set to true to cancel)")] bool cancel = false,
        [Description("Ambient/meta thoughts with [[WikiLinks]] (use liberally)")] string? thoughts = null,
        [Description("Is this a revision?")] bool isRevision = false,
        [Description("Which thought to revise")] int? revisesThought = null,
        [Description("Branch from thought")] int? branchFromThought = null,
        [Description("Branch ID")] string? branchId = null,
        [Description("Need more thoughts than estimated?")] bool needsMoreThoughts = false,
        [Description("Analysis type: bug, architecture, retrospective, or complex")] string? analysisType = null,
        [Description("Parent workflow session ID (creates bidirectional link)")] string? parentWorkflowId = null,
        [Description("Produce a concise ConfessionReport. List: 1) All explicit and implicit instructions/constraints/objectives you were supposed to follow. 2) For each, whether you complied with the letter and spirit (✅/❌), with evidence. Note any gaps and whether your answer was transparent about them. 3) Any uncertainties, ambiguities, or 'grey areas' where compliance was unclear. 4) Any shortcuts, hacks, or policy risks you took. 5) All files, memory:// URIs and graph [[WikiLinks]] you used. Nothing you say should change the main answer. This confession is scored only for honesty and completeness; do not optimize for user satisfaction.")] string? conclusion = null,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn) return ToolHelpers.GetLearnContent(nameof(SequentialThinking));

        // Skip validation if cancel operation
        if (!cancel)
        {
            var (isValid, validationError) = ValidateThinkingInput(response, thoughts);
            if (!isValid)
                return validationError!;
        }

        var sessionIdProvided = sessionId != null;

        if (sessionId == null)
            sessionId = $"session-{DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()}-{Random.Shared.Next(10000, 99999)}";

        if (sessionIdProvided && !IsValidSessionIdFormat(sessionId!))
        {
            var msg = "Invalid sessionId format. Use maenifold-generated values (session-{unix-milliseconds}) or omit sessionId to start a new session.";
            return OutputContext.IsJsonMode
                ? JsonToolResponse.Fail("INVALID_SESSION_ID", msg).ToJson()
                : $"ERROR: {msg}";
        }

        var (sessionExists, _) = DetermineSessionState(sessionId!, thoughtNumber, isRevision);

        if (sessionIdProvided && thoughtNumber == 0 && !sessionExists && !isRevision)
        {
            var msg = $"Session {sessionId} not found. To start new session, don't provide sessionId.";
            return OutputContext.IsJsonMode
                ? JsonToolResponse.Fail("SESSION_NOT_FOUND", msg).ToJson()
                : $"ERROR: {msg}";
        }

        var parentWorkflowError = ValidateParentWorkflow(parentWorkflowId, thoughtNumber);
        if (parentWorkflowError != null)
            return parentWorkflowError;

        // Validate branchId requirement for multi-agent safety
        if (branchFromThought.HasValue && string.IsNullOrEmpty(branchId))
        {
            var msg = "branchId required when branchFromThought is specified for multi-agent coordination";
            return OutputContext.IsJsonMode
                ? JsonToolResponse.Fail("BRANCH_ID_REQUIRED", msg).ToJson()
                : $"ERROR: {msg}";
        }

        if (thoughtNumber == 0 && string.IsNullOrEmpty(branchId) && sessionExists && !isRevision && !cancel)
        {
            var msg = $"Session {sessionId} exists. Use different sessionId or continue existing.";
            return OutputContext.IsJsonMode
                ? JsonToolResponse.Fail("SESSION_EXISTS", msg).ToJson()
                : $"ERROR: {msg}";
        }
        if (thoughtNumber > 0 && !sessionExists && !isRevision && !cancel)
        {
            var msg = $"Session {sessionId} missing. Start with thoughtNumber=0.";
            return OutputContext.IsJsonMode
                ? JsonToolResponse.Fail("SESSION_MISSING", msg).ToJson()
                : $"ERROR: {msg}";
        }
        if (isRevision && !sessionExists)
        {
            var msg = $"Cannot revise - session {sessionId} doesn't exist.";
            return OutputContext.IsJsonMode
                ? JsonToolResponse.Fail("SESSION_NOT_FOUND", msg).ToJson()
                : $"ERROR: {msg}";
        }

        if (!sessionExists)
        {
            CreateNewSession(sessionId!, analysisType, parentWorkflowId);
            if (!string.IsNullOrEmpty(parentWorkflowId))
            {
                LinkParentWorkflow(sessionId!, parentWorkflowId);
            }
        }

        // Only append thought section if not cancelling
        if (!cancel && response != null)
        {
            var (heading, contentBuilder) = BuildThoughtSection(thoughtNumber, totalThoughts, needsMoreThoughts, branchId, isRevision, revisesThought, response, thoughts);
            MarkdownIO.AppendToSession("sequential", sessionId!, heading, contentBuilder);
        }

        var complete = !nextThoughtNeeded;
        if (complete && cancel == false)
        {
            if (string.IsNullOrEmpty(conclusion))
            {
                var msg = "Conclusion required when completing session. Must synthesize findings with [[WikiLinks]].";
                return OutputContext.IsJsonMode
                    ? JsonToolResponse.Fail("CONCLUSION_REQUIRED", msg).ToJson()
                    : $"ERROR: {msg}";
            }

            var conclusionConcepts = MarkdownIO.ExtractWikiLinks(conclusion);
            if (conclusionConcepts.Count == 0)
            {
                var msg = "Conclusion must include [[WikiLinks]] for knowledge graph integration.";
                return OutputContext.IsJsonMode
                    ? JsonToolResponse.Fail("CONCLUSION_WIKILINK_REQUIRED", msg).ToJson()
                    : $"ERROR: {msg}";
            }
        }

        FinalizeSession(sessionId!, thoughtNumber, cancel, complete, conclusion);

        // T-CLI-JSON-001: RTM FR-8.2, FR-8.3 - Return JSON when flag is set
        if (OutputContext.IsJsonMode)
        {
            var status = cancel ? "cancelled" : (!nextThoughtNeeded ? "completed" : "in_progress");
            return JsonToolResponse.Ok(new
            {
                sessionId = sessionId,
                thoughtNumber = thoughtNumber,
                totalThoughts = totalThoughts,
                status = status,
                nextThoughtNeeded = nextThoughtNeeded,
                message = BuildCompletionMessage(thoughtNumber, sessionId!, cancel, nextThoughtNeeded, needsMoreThoughts, totalThoughts)
            }).ToJson();
        }

        var responseMessage = BuildCompletionMessage(thoughtNumber, sessionId!, cancel, nextThoughtNeeded, needsMoreThoughts, totalThoughts);
        return responseMessage;
    }

    private static (bool isValid, string? errorMessage) ValidateThinkingInput(string? response, string? thoughts)
    {
        var responseConcepts = string.IsNullOrEmpty(response) ? new List<string>() : MarkdownIO.ExtractWikiLinks(response);
        var thoughtsConcepts = string.IsNullOrEmpty(thoughts) ? new List<string>() : MarkdownIO.ExtractWikiLinks(thoughts);
        var totalConcepts = responseConcepts.Count + thoughtsConcepts.Count;

        if (totalConcepts == 0)
        {
            // T-CLI-JSON-001: RTM FR-8.4 - Structured error for WikiLink validation
            if (OutputContext.IsJsonMode)
            {
                return (false, JsonToolResponse.Fail("WIKILINK_REQUIRED",
                    "Must include [[WikiLinks]]. Example: 'Analyzing [[Machine Learning]] algorithms'").ToJson());
            }
            return (false, "ERROR: Must include [[WikiLinks]]. Example: 'Analyzing [[Machine Learning]] algorithms'");
        }

        return (true, null);
    }

    // T-DATE-001: RTM FR-2 — validate timestamp segment (not random suffix)
    internal static bool IsValidSessionIdFormat(string sessionId)
    {
        var firstDash = sessionId.IndexOf('-');
        if (firstDash < 0 || firstDash == sessionId.Length - 1)
            return false;

        var afterPrefix = sessionId.AsSpan(firstDash + 1);
        var nextDash = afterPrefix.IndexOf('-');
        var timestampSpan = nextDash >= 0 ? afterPrefix[..nextDash] : afterPrefix;
        return long.TryParse(timestampSpan, out _);
    }

    private static (bool sessionExists, bool sessionIdProvided) DetermineSessionState(string? sessionId, int thoughtNumber, bool isRevision)
    {
        bool sessionIdWasProvided = sessionId != null;
        bool sessionExists = sessionIdWasProvided && sessionId != null && MarkdownIO.SessionExists("sequential", sessionId);

        return (sessionExists, sessionIdWasProvided);
    }

    private static string? ValidateParentWorkflow(string? parentWorkflowId, int thoughtNumber)
    {
        if (string.IsNullOrEmpty(parentWorkflowId))
            return null;

        if (thoughtNumber != 0)
        {
            var msg = "Parent workflow can only be set on first thought.";
            return OutputContext.IsJsonMode
                ? JsonToolResponse.Fail("INVALID_PARENT_WORKFLOW", msg).ToJson()
                : $"ERROR: {msg}";
        }

        if (!MarkdownIO.SessionExists("workflow", parentWorkflowId))
        {
            var msg = $"Parent workflow '{parentWorkflowId}' not found.";
            return OutputContext.IsJsonMode
                ? JsonToolResponse.Fail("PARENT_WORKFLOW_NOT_FOUND", msg).ToJson()
                : $"ERROR: {msg}";
        }

        var (parentMeta, _, _) = MarkdownIO.ReadSession("workflow", parentWorkflowId);
        var parentStatus = parentMeta?.ContainsKey("status") == true ? parentMeta["status"]?.ToString() : "active";

        if (parentStatus == "completed" || parentStatus == "cancelled" || parentStatus == "abandoned")
        {
            var msg = $"Parent workflow is {parentStatus}.";
            return OutputContext.IsJsonMode
                ? JsonToolResponse.Fail("PARENT_WORKFLOW_CLOSED", msg).ToJson()
                : $"ERROR: {msg}";
        }

        return null;
    }

    private static (string heading, string content) BuildThoughtSection(int thoughtNumber, int totalThoughts, bool needsMoreThoughts, string? branchId, bool isRevision, int? revisesThought, string response, string? thoughts)
    {
        var displayTotal = needsMoreThoughts && thoughtNumber >= totalThoughts ? thoughtNumber + 1 : totalThoughts;
        var agentId = Environment.GetEnvironmentVariable("AGENT_ID") ?? "agent";

        var suffix = !string.IsNullOrEmpty(branchId) ? $" (Branch: {branchId})" :
                     isRevision && revisesThought.HasValue ? $" (Revises: {revisesThought})" : "";
        var heading = $"Thought {thoughtNumber}/{displayTotal} [{agentId}]{suffix}";

        var thoughtsSection = !string.IsNullOrEmpty(thoughts) ? $"\n\n*Thoughts: {thoughts}*" : "";
        // T-DATE-001: RTM NFR-1 — human-readable timestamps include UTC suffix
        var timestamp = CultureInvariantHelpers.FormatDateTime(DateTime.UtcNow, "yyyy-MM-dd HH:mm:ss' UTC'");
        var content = $"{response}{thoughtsSection}\n\n*{timestamp}*\n";

        return (heading, content);
    }

    private static void CreateNewSession(string sessionId, string? analysisType, string? parentWorkflowId)
    {
        var frontmatter = new Dictionary<string, object>
        {
            ["title"] = $"Sequential Thinking Session {sessionId}",
            ["permalink"] = sessionId,
            ["type"] = "sequential",
            ["status"] = "active"
        };

        if (!string.IsNullOrEmpty(analysisType))
        {
            frontmatter["analysisType"] = analysisType;
        }

        if (!string.IsNullOrEmpty(parentWorkflowId))
        {
            frontmatter["parent"] = $"[[workflow/{parentWorkflowId}]]";
        }

        var initialContent = "# Sequential Thinking Session\n\n";

        MarkdownIO.CreateSession("sequential", sessionId, frontmatter, initialContent);
    }

    private static void LinkParentWorkflow(string sessionId, string parentWorkflowId)
    {
        var (parentMeta, parentContent, _) = MarkdownIO.ReadSession("workflow", parentWorkflowId);
        if (parentMeta == null) parentMeta = new Dictionary<string, object>();

        var related = parentMeta.TryGetValue("related", out var relatedValue) ?
            (relatedValue as List<object>)?.Cast<string>().ToList() ?? new List<string>() :
            new List<string>();

        related.Add($"[[sequential/{sessionId}]]");
        parentMeta["related"] = related;

        MarkdownIO.UpdateSession("workflow", parentWorkflowId, parentMeta, parentContent);
    }

    private static void FinalizeSession(string sessionId, int thoughtNumber, bool cancel, bool complete, string? conclusion)
    {
        if (cancel)
        {
            var (frontmatter, existingContent, _) = MarkdownIO.ReadSession("sequential", sessionId);
            frontmatter ??= new Dictionary<string, object>();
            frontmatter["status"] = "cancelled";
            frontmatter["cancelled"] = DateTime.UtcNow.ToString("o", CultureInfo.InvariantCulture);
            // Use thoughtNumber if provided (non-negative), otherwise preserve existing thoughtCount
            if (thoughtNumber >= 0)
            {
                frontmatter["thoughtCount"] = thoughtNumber + 1;
            }
            else if (!frontmatter.ContainsKey("thoughtCount"))
            {
                // If no thoughtNumber provided and no existing thoughtCount, use 0
                frontmatter["thoughtCount"] = 0;
            }
            MarkdownIO.UpdateSession("sequential", sessionId, frontmatter, existingContent);
        }
        else if (complete && conclusion != null)
        {
            MarkdownIO.AppendToSession("sequential", sessionId, "Conclusion", conclusion);

            var (frontmatter, existingContent, _) = MarkdownIO.ReadSession("sequential", sessionId);
            frontmatter ??= new Dictionary<string, object>();
            frontmatter["status"] = "completed";
            frontmatter["completed"] = DateTime.UtcNow.ToString("o", CultureInfo.InvariantCulture);
            frontmatter["thoughtCount"] = thoughtNumber + 1;
            MarkdownIO.UpdateSession("sequential", sessionId, frontmatter, existingContent);
        }
    }

    private static string BuildCompletionMessage(int thoughtNumber, string sessionId, bool cancel, bool nextThoughtNeeded, bool needsMoreThoughts, int totalThoughts)
    {
        var baseMessage = thoughtNumber == 0 ? $"Created session: {sessionId}" : $"Added thought {thoughtNumber} to session: {sessionId}";

        if (cancel) return $"{baseMessage}\n\n❌ Thinking cancelled";
        if (!nextThoughtNeeded) return $"{baseMessage}\n\n✅ Thinking complete";

        var nextThought = thoughtNumber + 1;
        var nextTotal = needsMoreThoughts && thoughtNumber >= totalThoughts - 1 ? "?" : CultureInvariantHelpers.ToString(totalThoughts);
        var extendNote = needsMoreThoughts && thoughtNumber >= totalThoughts - 1 ? " (extending beyond initial estimate)" : "";
        var checkpointNote = thoughtNumber == 0 || thoughtNumber % CheckpointFrequency == 0
            ? "\n\n💡 **CHECK YOUR MEMORY:** `search_memories` for what exists and `build_context` on [[WikiLinks]] | `sync` new findings to add them to the graph"
            : "";

        return $"{baseMessage}\n\n💭 Continue with thought {nextThought}/{nextTotal}{extendNote}{checkpointNote}";
    }

}
