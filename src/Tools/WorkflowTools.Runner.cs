using ModelContextProtocol.Server;
using System.ComponentModel;
using Maenifold.Utils;

namespace Maenifold.Tools;

public partial class WorkflowTools
{
    [McpServerTool(Title = "Execute Workflow"), Description(@"Orchestrates systematic problem-solving through predefined methodologies with embedded sequential thinking and tool coordination.
Use ListAssets or ReadMcpResource with asset://catalog to discover available workflows before starting.
Select when AI needs structured approaches, methodology frameworks, multi-step processes, or coordinated tool usage.
Requires workflow selection, session management, response integration, and systematic progression through defined steps.
Connects to SequentialThinking for embedded analysis, all maenifold tools for orchestrated execution.
Supports supervisor/submachine pattern: register a SequentialThinking session via submachineSessionId to pause step advancement while deep analysis runs. Workflow blocks until the submachine completes, is cancelled, or is abandoned.
Returns structured guidance with step progression, tool hints, quality gates, and systematic methodology execution.")]
    public static string Workflow(
        [Description("Session ID (continue existing)")] string? sessionId = null,
        [Description("Workflow ID(s) to start (single or array)")] string? workflowId = null,
        [Description("Response to current step - MUST include [[WikiLinks]] to build knowledge")] string? response = null,
        [Description("Ambient/meta thoughts - include [[WikiLinks]] (use liberally)")] string? thoughts = null,
        [Description("Set to 'completed' or 'cancelled' to end session")] string? status = null,
        [Description("Produce a concise ConfessionReport. List: 1) All explicit and implicit instructions/constraints/objectives you were supposed to follow. 2) For each, whether you complied with the letter and spirit (✅/❌), with evidence. Note any gaps and whether your answer was transparent about them. 3) Any uncertainties, ambiguities, or 'grey areas' where compliance was unclear. 4) Any shortcuts, hacks, or policy risks you took. 5) All files, memory:// URIs and graph [[WikiLinks]] you used. Nothing you say should change the main answer. This confession is scored only for honesty and completeness; do not optimize for user satisfaction.")] string? conclusion = null,
        [Description("View queue status")] bool view = false,
        [Description("Append workflow(s) to queue")] string? append = null,
        [Description("Session ID of a submachine (SequentialThinking/AssumptionLedger) to register with this workflow")] string? submachineSessionId = null,
        [Description("Return help documentation instead of executing")] bool learn = false)
    {
        if (learn) return ToolHelpers.GetLearnContent(nameof(Workflow));

        ValidateParameters(workflowId, sessionId);
        return DispatchWorkflowOperation(sessionId, workflowId, response, thoughts, status, conclusion, view, append, submachineSessionId);
    }

    private static void ValidateParameters(string? workflowId, string? sessionId)
    {
        if (workflowId != null && sessionId != null)
        {
            throw new InvalidOperationException("Cannot provide both workflowId and sessionId together.\n\nCURRENT PARAMETERS:\n" +
                   $"  sessionId: {sessionId}\n" +
                   $"  workflowId: {workflowId}\n\n" +
                   "CORRECT USAGE:\n" +
                   "  To continue existing session: Use sessionId + response\n" +
                   "  To start new workflow: Use workflowId only\n" +
                   "  To view queue: Use sessionId + view\n" +
                   "  To append to queue: Use sessionId + append\n\n" +
                   "TIP: You don't need to specify the workflow ID when continuing a session - the session already knows which workflow it's running.");
        }
    }

    private static string DispatchWorkflowOperation(
        string? sessionId,
        string? workflowId,
        string? response,
        string? thoughts,
        string? status,
        string? conclusion,
        bool view,
        string? append,
        string? submachineSessionId = null)
    {
        // Start new workflow
        if (workflowId != null && sessionId == null)
        {
            return WorkflowOperations.Start(workflowId);
        }

        // View existing session queue
        if (sessionId != null && view)
        {
            return WorkflowOperations.View(sessionId);
        }

        // Append workflow to queue
        if (sessionId != null && append != null)
        {
            return WorkflowOperations.Append(sessionId, append);
        }

        // Continue existing session (or register submachine)
        if (sessionId != null && response != null)
        {
            return WorkflowOperations.Continue(sessionId, response, thoughts, status, conclusion, submachineSessionId);
        }

        throw new InvalidOperationException("Invalid parameter combination.\n\n" +
            "VALID OPERATIONS:\n" +
            "  Start new workflow: workflowId only\n" +
            "  Continue session: sessionId + response\n" +
            "  View queue: sessionId + view=true\n" +
            "  Append to queue: sessionId + append\n\n" +
            "TIP: view=true and append both require a sessionId to identify which session to operate on.");
    }
}
