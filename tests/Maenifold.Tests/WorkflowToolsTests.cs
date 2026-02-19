// T-COV-001.4: RTM FR-17.6
// Tests for WorkflowTools public API — session creation, step advancement,
// status transitions (active→completed/cancelled), serial queuing, conclusion with WikiLinks.
// These tests exercise the WorkflowTools.Workflow() entry point (not the internal
// WorkflowOperations class directly), targeting uncovered paths in the 58.9% baseline.

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text.Json;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Integration tests for WorkflowTools.Workflow() public API.
/// Tests session creation, step advancement, status transitions, serial queuing,
/// and conclusion with WikiLinks through the public entry point.
/// Uses real workflow JSON files and real session files (NO MOCKS) per [[testing-philosophy]].
/// </summary>
[NonParallelizable]
public class WorkflowToolsTests
{
    private static readonly JsonSerializerOptions JsonOptions = new() { WriteIndented = true };

    // Unique workflow names that won't conflict with other test suites (wtt = WorkflowToolsTests)
    private static readonly string[] TestWorkflowNames = new[]
    {
        "wtt-alpha", "wtt-beta", "wtt-gamma"
    };

    // Reusable static arrays to satisfy CA1861 (avoid constant array arguments)
    private static readonly string[] QueueAlphaBeta = new[] { "wtt-alpha", "wtt-beta" };
    private static readonly string[] QueueGammaBeta = new[] { "wtt-gamma", "wtt-beta" };
    private static readonly string[] QueueGammaGamma = new[] { "wtt-gamma", "wtt-gamma" };
    private static readonly string[] QueueAlphaBetaGamma = new[] { "wtt-alpha", "wtt-beta", "wtt-gamma" };
    private static readonly string[] AppendBetaGamma = new[] { "wtt-beta", "wtt-gamma" };

    private string _testWorkflowsPath = string.Empty;
    private string _testMemoryPath = string.Empty;
    private List<string> _createdSessions = new();

    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();
        _testMemoryPath = Config.MemoryPath;
        _testWorkflowsPath = WorkflowCommon.WorkflowsPath;

        // Ensure workflow directory exists
        Directory.CreateDirectory(_testWorkflowsPath);

        // Ensure thinking/workflow session directory exists
        Directory.CreateDirectory(Path.Combine(_testMemoryPath, "thinking", "workflow"));

        // Create minimal test workflow JSON files
        CreateTestWorkflow("wtt-alpha", 3);
        CreateTestWorkflow("wtt-beta", 2);
        CreateTestWorkflow("wtt-gamma", 1);
    }

    [TearDown]
    public void TearDown()
    {
        // Delete all test sessions created during tests
        foreach (var sessionId in _createdSessions)
        {
            // Session files are nested under thinking/workflow/YYYY/MM/DD/
            // Use SessionExists check and delete via the resolved path
            var sessionPath = MarkdownWriter.GetSessionPath("workflow", sessionId);
            if (File.Exists(sessionPath))
                File.Delete(sessionPath);
        }

        // Delete test workflow files
        foreach (var workflowName in TestWorkflowNames)
        {
            var wfPath = Path.Combine(_testWorkflowsPath, $"{workflowName}.json");
            if (File.Exists(wfPath))
                File.Delete(wfPath);
        }
    }

    // ============ Helper Methods ============

    /// <summary>
    /// Creates a minimal test workflow JSON with the specified number of steps.
    /// </summary>
    private void CreateTestWorkflow(string name, int stepCount)
    {
        var steps = Enumerable.Range(1, stepCount).Select(i => new
        {
            id = $"step-{i}",
            name = $"Step {i}",
            description = $"Test step {i} description for [[workflow]] coverage"
        }).ToArray();

        var workflow = new
        {
            id = name,
            name = $"Test Workflow {name}",
            description = $"Test workflow for [[WorkflowTools]] coverage",
            steps
        };

        var json = JsonSerializer.Serialize(workflow, JsonOptions);
        File.WriteAllText(Path.Combine(_testWorkflowsPath, $"{name}.json"), json);
    }

    /// <summary>
    /// Extracts the session ID from a Workflow() result string and registers it for cleanup.
    /// The result contains "Created session: workflow-TIMESTAMP" on the first line.
    /// </summary>
    private string ExtractAndRegisterSessionId(string result)
    {
        // Result format: "Created session: workflow-<timestamp>\n\nStep 1/N\n\n..."
        var lines = result.Split('\n', StringSplitOptions.RemoveEmptyEntries);
        var sessionLine = lines.FirstOrDefault(l => l.StartsWith("Created session:", StringComparison.Ordinal));
        Assert.That(sessionLine, Is.Not.Null, $"Could not find 'Created session:' in result:\n{result}");

        var sessionId = sessionLine!.Replace("Created session:", "").Trim();
        _createdSessions.Add(sessionId);
        return sessionId;
    }

    /// <summary>
    /// Extracts the queue string array from the deserialized frontmatter queue object,
    /// handling both object[] and List&lt;object&gt; YAML deserialization variants.
    /// </summary>
    private static string[] ExtractQueue(object queueObj)
    {
        if (queueObj is string[] strArray)
            return strArray;
        if (queueObj is object[] objArray)
            return objArray.Select(x => x?.ToString() ?? "").ToArray();
        if (queueObj is List<object> list)
            return list.Select(x => x?.ToString() ?? "").ToArray();
        return Array.Empty<string>();
    }

    // ============ Learn Mode ============

    /// <summary>
    /// Test 1: learn=true short-circuits all validation and execution.
    /// Returns help content or "not found" message — never creates a session.
    /// </summary>
    [Test]
    public void LearnModeShortCircuitsExecutionWithoutCreatingSession()
    {
        // T-COV-001.4: RTM FR-17.6
        // Act - Call with learn=true; no workflowId or sessionId needed
        var result = WorkflowTools.Workflow(learn: true);

        // Assert - Returns some content (help or "not found") and never creates a session
        Assert.That(result, Is.Not.Null.And.Not.Empty, "Should return some content");
        Assert.That(result, Does.Not.Contain("Created session:"), "Learn mode should not create a session");
        Assert.That(result, Does.Not.Contain("VALID OPERATIONS"), "Learn mode should not trigger operation validation");
        Assert.That(result, Does.Not.Contain("Cannot provide both"), "Learn mode should not trigger parameter validation");
    }

    // ============ Session Creation (Workflow() with workflowId) ============

    /// <summary>
    /// Test 2: Workflow() with a valid workflowId creates a new session and returns step 1 instructions.
    /// This is the primary session creation path.
    /// </summary>
    [Test]
    public void WorkflowWithWorkflowIdCreatesNewSession()
    {
        // T-COV-001.4: RTM FR-17.6
        // Act
        var result = WorkflowTools.Workflow(workflowId: "wtt-alpha");

        // Assert - Result contains session ID and first step
        Assert.That(result, Does.Contain("Created session:"), "Should announce session creation");
        Assert.That(result, Does.Contain("Step 1/3"), "Should display step 1 of 3");
        Assert.That(result, Does.Not.Contain("ERROR"), "Should not error on valid workflow");

        // Register for cleanup
        ExtractAndRegisterSessionId(result);
    }

    /// <summary>
    /// Test 3: Workflow() creates a session file on disk that is readable via MarkdownIO.
    /// Verifies that MarkdownIO.CreateSession was called with correct content.
    /// </summary>
    [Test]
    public void WorkflowCreatedSessionIsReadableViaMarkdownIO()
    {
        // T-COV-001.4: RTM FR-17.6
        // Act
        var result = WorkflowTools.Workflow(workflowId: "wtt-beta");
        var sessionId = ExtractAndRegisterSessionId(result);

        // Assert - Session exists and has correct frontmatter
        Assert.That(MarkdownIO.SessionExists("workflow", sessionId), Is.True,
            "Session should exist after creation");

        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        Assert.That(frontmatter, Is.Not.Null, "Session should have parseable frontmatter");
        Assert.That(frontmatter!["status"], Is.EqualTo("active"), "New session should be active");
        Assert.That(frontmatter!["type"], Is.EqualTo("workflow"), "Type should be 'workflow'");
        Assert.That(frontmatter!["currentStep"].ToString(), Is.EqualTo("0"), "Initial step should be 0");
        Assert.That(frontmatter!["currentWorkflow"].ToString(), Is.EqualTo("0"), "Initial workflow index should be 0");
    }

    /// <summary>
    /// Test 4: Workflow() with a nonexistent workflowId returns an error message.
    /// Verifies validation of the workflow file existence.
    /// </summary>
    [Test]
    public void WorkflowWithNonexistentWorkflowIdReturnsError()
    {
        // T-COV-001.4: RTM FR-17.6
        // Act
        var result = WorkflowTools.Workflow(workflowId: "does-not-exist-xyz");

        // Assert
        Assert.That(result, Does.Contain("ERROR"), "Should return error for missing workflow");
        Assert.That(result, Does.Contain("does-not-exist-xyz"), "Error should name the missing workflow");
        Assert.That(result, Does.Contain("not found"), "Error should say 'not found'");
    }

    // ============ Parameter Validation (both workflowId AND sessionId) ============

    /// <summary>
    /// Test 5: Workflow() with both workflowId AND sessionId throws InvalidOperationException.
    /// Verifies ValidateParameters enforcement.
    /// </summary>
    [Test]
    public void WorkflowWithBothWorkflowIdAndSessionIdThrows()
    {
        // T-COV-001.4: RTM FR-17.6
        // Act & Assert
        var ex = Assert.Throws<InvalidOperationException>(() =>
            WorkflowTools.Workflow(workflowId: "wtt-alpha", sessionId: "workflow-123456789"));

        Assert.That(ex.Message, Does.Contain("Cannot provide both workflowId and sessionId"),
            "Error should explain the constraint");
        Assert.That(ex.Message, Does.Contain("sessionId"), "Error should mention sessionId");
        Assert.That(ex.Message, Does.Contain("workflowId"), "Error should mention workflowId");
    }

    // ============ Step Advancement (Workflow() with sessionId + response) ============

    /// <summary>
    /// Test 6: Workflow() with sessionId + response advances from step 1 to step 2.
    /// Verifies the basic continue path through the public API.
    /// </summary>
    [Test]
    public void WorkflowAdvancesFromStep1ToStep2()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange - Create a new session
        var createResult = WorkflowTools.Workflow(workflowId: "wtt-alpha");
        var sessionId = ExtractAndRegisterSessionId(createResult);

        // Act - Advance with a response containing WikiLinks
        var result = WorkflowTools.Workflow(
            sessionId: sessionId,
            response: "Completed step 1 with [[workflow]] and [[session-management]].");

        // Assert
        Assert.That(result, Does.Contain("Step 2/3"), "Should display step 2 of 3");
        Assert.That(result, Does.Not.Contain("ERROR"), "Should not error");

        // Verify session state
        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        Assert.That(frontmatter, Is.Not.Null);
        var currentStep = Convert.ToInt32(frontmatter!["currentStep"], CultureInfo.InvariantCulture);
        Assert.That(currentStep, Is.EqualTo(1), "currentStep should be 1 after first advance");
    }

    /// <summary>
    /// Test 7: Workflow() with response that has no WikiLinks returns validation error.
    /// Verifies the WikiLink requirement is enforced through the public API.
    /// </summary>
    [Test]
    public void WorkflowResponseWithoutWikiLinksReturnsError()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange
        var createResult = WorkflowTools.Workflow(workflowId: "wtt-alpha");
        var sessionId = ExtractAndRegisterSessionId(createResult);

        // Act - Response with no WikiLinks
        var result = WorkflowTools.Workflow(
            sessionId: sessionId,
            response: "Completed step 1 without any double-bracket concepts.");

        // Assert
        Assert.That(result, Does.Contain("ERROR"), "Should require WikiLinks");
        Assert.That(result, Does.Contain("[[WikiLink]]"), "Error should show expected format");
    }

    // ============ Status Transition: active → completed ============

    /// <summary>
    /// Test 8: Status transition active → completed using status="completed" + conclusion with WikiLinks.
    /// Verifies the completion path through the public API.
    /// </summary>
    [Test]
    public void StatusTransitionActiveToCompleted()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange
        var createResult = WorkflowTools.Workflow(workflowId: "wtt-alpha");
        var sessionId = ExtractAndRegisterSessionId(createResult);

        // Act - Complete with status="completed" and a conclusion with WikiLinks
        var result = WorkflowTools.Workflow(
            sessionId: sessionId,
            response: "Final step with [[session-management]].",
            status: "completed",
            conclusion: "Session concluded with [[workflow]] insights and [[maenifold]] learnings.");

        // Assert
        Assert.That(result, Does.Contain("completed"), "Should confirm completion");
        Assert.That(result, Does.Not.Contain("ERROR"), "Should not error on valid completion");

        // Verify session frontmatter is updated
        var (frontmatter, content, _) = MarkdownIO.ReadSession("workflow", sessionId);
        Assert.That(frontmatter, Is.Not.Null);
        Assert.That(frontmatter!["status"], Is.EqualTo("completed"), "Status should be 'completed'");
        Assert.That(frontmatter!["completed"], Is.Not.Null, "Should have completion timestamp");
        Assert.That(content, Does.Contain("Conclusion"), "Content should include Conclusion section");
        Assert.That(content, Does.Contain("[[maenifold]]"), "Conclusion WikiLinks should be stored");
    }

    /// <summary>
    /// Test 9: Completing with status="completed" but no conclusion returns an error.
    /// Verifies that conclusion is required for completion.
    /// </summary>
    [Test]
    public void CompletionWithoutConclusionReturnsError()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange
        var createResult = WorkflowTools.Workflow(workflowId: "wtt-alpha");
        var sessionId = ExtractAndRegisterSessionId(createResult);

        // Act - Attempt to complete without a conclusion
        var result = WorkflowTools.Workflow(
            sessionId: sessionId,
            response: "Done with [[workflow]].",
            status: "completed");

        // Assert
        Assert.That(result, Does.Contain("ERROR"), "Should error when conclusion is missing");
        Assert.That(result, Does.Contain("Conclusion required"), "Error should mention conclusion requirement");
    }

    /// <summary>
    /// Test 10: Completing with a conclusion that has no WikiLinks returns an error.
    /// Verifies the WikiLink requirement for conclusion.
    /// </summary>
    [Test]
    public void CompletionWithConclusionLackingWikiLinksReturnsError()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange
        var createResult = WorkflowTools.Workflow(workflowId: "wtt-alpha");
        var sessionId = ExtractAndRegisterSessionId(createResult);

        // Act - Attempt to complete with a conclusion that lacks WikiLinks
        var result = WorkflowTools.Workflow(
            sessionId: sessionId,
            response: "Done with [[workflow]].",
            status: "completed",
            conclusion: "This is a conclusion without any WikiLinks.");

        // Assert
        Assert.That(result, Does.Contain("ERROR"), "Should error when conclusion lacks WikiLinks");
        Assert.That(result, Does.Contain("[[WikiLinks]]"), "Error should mention WikiLinks format");
    }

    // ============ Status Transition: active → cancelled ============

    /// <summary>
    /// Test 11: Status transition active → cancelled using status="cancelled".
    /// Verifies the cancellation path through the public API.
    /// </summary>
    [Test]
    public void StatusTransitionActiveToCancelled()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange
        var createResult = WorkflowTools.Workflow(workflowId: "wtt-alpha");
        var sessionId = ExtractAndRegisterSessionId(createResult);

        // Act
        var result = WorkflowTools.Workflow(
            sessionId: sessionId,
            response: "Cancelling this session with [[cancellation-reason]].",
            status: "cancelled");

        // Assert
        Assert.That(result, Does.Contain("cancelled"), "Should confirm cancellation");
        Assert.That(result, Does.Not.Contain("ERROR"), "Should not error on valid cancellation");

        // Verify session frontmatter is updated
        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        Assert.That(frontmatter, Is.Not.Null);
        Assert.That(frontmatter!["status"], Is.EqualTo("cancelled"), "Status should be 'cancelled'");
        Assert.That(frontmatter!["cancelled"], Is.Not.Null, "Should have cancellation timestamp");
    }

    // ============ Serial Workflow Queuing ============

    /// <summary>
    /// Test 12: Workflow() with a JSON array of workflowIds starts with the first workflow
    /// and sets up the full queue for serial execution.
    /// </summary>
    [Test]
    public void SerialQueueStartsWithFirstWorkflowInArray()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange - JSON array of workflow IDs
        var workflowIds = JsonSerializer.Serialize(QueueAlphaBeta);

        // Act
        var result = WorkflowTools.Workflow(workflowId: workflowIds);
        var sessionId = ExtractAndRegisterSessionId(result);

        // Assert - First step of first workflow (wtt-alpha has 3 steps)
        Assert.That(result, Does.Contain("Step 1/3"), "Should start with first step of first workflow");
        Assert.That(result, Does.Contain("Created session:"), "Should create a session");

        // Verify queue is set correctly in frontmatter
        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        Assert.That(frontmatter, Is.Not.Null);

        var queue = ExtractQueue(frontmatter!["queue"]);
        Assert.That(queue.Length, Is.EqualTo(2), "Queue should contain both workflows");
        Assert.That(queue[0], Is.EqualTo("wtt-alpha"), "First in queue is wtt-alpha");
        Assert.That(queue[1], Is.EqualTo("wtt-beta"), "Second in queue is wtt-beta");
    }

    /// <summary>
    /// Test 13: Serial queuing — after completing all steps of the first workflow,
    /// the session automatically advances to the next workflow in the queue.
    /// </summary>
    [Test]
    public void SerialQueueAdvancesToNextWorkflowAfterFirstCompletes()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange - Queue with a 1-step workflow followed by a 2-step workflow
        var workflowIds = JsonSerializer.Serialize(QueueGammaBeta);
        var createResult = WorkflowTools.Workflow(workflowId: workflowIds);
        var sessionId = ExtractAndRegisterSessionId(createResult);

        // wtt-gamma has 1 step; completing it should transition to wtt-beta
        var response = "Step done with [[workflow-transition]] and [[serial-queuing]].";

        // Act - Complete the single step of wtt-gamma
        var result = WorkflowTools.Workflow(sessionId: sessionId, response: response);

        // Assert - Should advance to first step of wtt-beta (2 steps)
        Assert.That(result, Does.Contain("Step 1/2"), "Should advance to step 1 of the next workflow");
        Assert.That(result, Does.Not.Contain("ERROR"), "Should not error on workflow transition");

        // Verify session state
        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        Assert.That(frontmatter, Is.Not.Null);
        var currentWorkflow = Convert.ToInt32(frontmatter!["currentWorkflow"], CultureInfo.InvariantCulture);
        var currentStep = Convert.ToInt32(frontmatter!["currentStep"], CultureInfo.InvariantCulture);
        Assert.That(currentWorkflow, Is.EqualTo(1), "currentWorkflow should be 1 (second workflow)");
        Assert.That(currentStep, Is.EqualTo(0), "currentStep should reset to 0");
    }

    /// <summary>
    /// Test 14: When all workflows in the serial queue are exhausted and a conclusion is provided,
    /// the session completes successfully.
    /// </summary>
    [Test]
    public void SerialQueueCompletesWhenAllWorkflowsDoneWithConclusion()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange - Two single-step workflows in queue
        var workflowIds = JsonSerializer.Serialize(QueueGammaGamma);
        var createResult = WorkflowTools.Workflow(workflowId: workflowIds);
        var sessionId = ExtractAndRegisterSessionId(createResult);

        var response = "Step done with [[workflow-queue]] completion.";
        var conclusion = "All workflows completed — [[serial-queuing]] verified with [[workflow]] mechanics.";

        // Act - Complete first workflow (wtt-gamma step 1)
        WorkflowTools.Workflow(sessionId: sessionId, response: response);

        // Complete second workflow — queue is now exhausted, provide conclusion
        var exhaustedResult = WorkflowTools.Workflow(
            sessionId: sessionId,
            response: response,
            conclusion: conclusion);

        // Assert
        Assert.That(exhaustedResult, Does.Contain("All workflows"), "Should indicate all workflows done");
        Assert.That(exhaustedResult, Does.Not.Contain("ERROR"), "Should not error with valid conclusion");

        // Verify session is completed
        var (frontmatter, content, _) = MarkdownIO.ReadSession("workflow", sessionId);
        Assert.That(frontmatter, Is.Not.Null);
        Assert.That(frontmatter!["status"], Is.EqualTo("completed"), "Should be completed");
        Assert.That(content, Does.Contain("[[serial-queuing]]"), "Conclusion WikiLinks should be in content");
    }

    /// <summary>
    /// Test 15: When all workflows in the queue are exhausted without a conclusion,
    /// the system prompts for a conclusion parameter rather than auto-completing.
    /// </summary>
    [Test]
    public void SerialQueueExhaustedWithoutConclusionPromptsForConclusion()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange
        var createResult = WorkflowTools.Workflow(workflowId: "wtt-gamma");
        var sessionId = ExtractAndRegisterSessionId(createResult);

        var response = "Done with [[workflow-queue]] exhaustion test.";

        // Act - Complete the only workflow step without a conclusion
        var result = WorkflowTools.Workflow(sessionId: sessionId, response: response);

        // Assert - Should ask for conclusion
        Assert.That(result, Does.Contain("ERROR"), "Should error when queue exhausted without conclusion");
        Assert.That(result, Does.Contain("All workflows complete"), "Should mention queue completion");
        Assert.That(result, Does.Contain("conclusion parameter"), "Should request conclusion");
    }

    // ============ View Queue ============

    /// <summary>
    /// Test 16: Workflow() with view=true and sessionId shows queue status.
    /// Verifies view dispatches correctly through the public API.
    /// </summary>
    [Test]
    public void ViewQueueShowsCurrentPositionAndNextStep()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange - Create session with multiple workflows
        var workflowIds = JsonSerializer.Serialize(QueueAlphaBetaGamma);
        var createResult = WorkflowTools.Workflow(workflowId: workflowIds);
        var sessionId = ExtractAndRegisterSessionId(createResult);

        // Act
        var result = WorkflowTools.Workflow(sessionId: sessionId, view: true);

        // Assert
        Assert.That(result, Does.Not.Contain("ERROR"), "Should not error");
        Assert.That(result, Does.Contain("Queue:"), "Should show queue header");
        Assert.That(result, Does.Contain("wtt-alpha"), "Should list first workflow");
        Assert.That(result, Does.Contain("wtt-beta"), "Should list second workflow");
        Assert.That(result, Does.Contain("wtt-gamma"), "Should list third workflow");
        Assert.That(result, Does.Contain("Position:"), "Should show position info");
        Assert.That(result, Does.Contain("step 1/3"), "Should show step position in wtt-alpha");
    }

    /// <summary>
    /// Test 17: View reflects updated position after step advancement.
    /// Verifies that view reads current persisted state.
    /// </summary>
    [Test]
    public void ViewReflectsUpdatedPositionAfterStepAdvancement()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange
        var createResult = WorkflowTools.Workflow(workflowId: "wtt-alpha");
        var sessionId = ExtractAndRegisterSessionId(createResult);

        // Advance one step
        WorkflowTools.Workflow(
            sessionId: sessionId,
            response: "Step 1 done with [[step-tracking]].");

        // Act - View after advancement
        var result = WorkflowTools.Workflow(sessionId: sessionId, view: true);

        // Assert
        Assert.That(result, Does.Contain("step 2/3"), "Should show updated step position");
        Assert.That(result, Does.Contain("Next:"), "Should show the next step info");
    }

    // ============ Append to Queue ============

    /// <summary>
    /// Test 18: Workflow() with append and sessionId adds a workflow to the queue.
    /// Verifies the append path through the public API.
    /// </summary>
    [Test]
    public void AppendAddsWorkflowToQueue()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange
        var createResult = WorkflowTools.Workflow(workflowId: "wtt-alpha");
        var sessionId = ExtractAndRegisterSessionId(createResult);

        // Act
        var result = WorkflowTools.Workflow(sessionId: sessionId, append: "wtt-beta");

        // Assert
        Assert.That(result, Does.Contain("Added 1 workflow(s) to queue"), "Should confirm addition");
        Assert.That(result, Does.Contain("wtt-beta"), "Should mention appended workflow");
        Assert.That(result, Does.Not.Contain("ERROR"), "Should not error");

        // Verify queue is updated
        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        Assert.That(frontmatter, Is.Not.Null);
        var queue = ExtractQueue(frontmatter!["queue"]);
        Assert.That(queue.Length, Is.EqualTo(2), "Queue should now have 2 workflows");
        Assert.That(queue[1], Is.EqualTo("wtt-beta"), "Second workflow should be wtt-beta");
    }

    /// <summary>
    /// Test 19: Workflow() with append and a JSON array adds multiple workflows to the queue.
    /// Verifies the batch append path through the public API.
    /// </summary>
    [Test]
    public void AppendAddsMultipleWorkflowsAsJsonArray()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange
        var createResult = WorkflowTools.Workflow(workflowId: "wtt-alpha");
        var sessionId = ExtractAndRegisterSessionId(createResult);

        var appendArray = JsonSerializer.Serialize(AppendBetaGamma);

        // Act
        var result = WorkflowTools.Workflow(sessionId: sessionId, append: appendArray);

        // Assert
        Assert.That(result, Does.Contain("Added 2 workflow(s) to queue"), "Should confirm 2 additions");
        Assert.That(result, Does.Contain("wtt-beta"), "Should mention first appended workflow");
        Assert.That(result, Does.Contain("wtt-gamma"), "Should mention second appended workflow");

        // Verify queue
        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        var queue = ExtractQueue(frontmatter!["queue"]);
        Assert.That(queue.Length, Is.EqualTo(3), "Queue should have 3 workflows total");
    }

    // ============ Conclusion with WikiLinks ============

    /// <summary>
    /// Test 20: Conclusion parameter is stored in session content with WikiLinks preserved.
    /// Verifies that conclusion content is appended and WikiLinks are intact.
    /// </summary>
    [Test]
    public void ConclusionWithWikiLinksIsStoredInSessionContent()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange
        var createResult = WorkflowTools.Workflow(workflowId: "wtt-alpha");
        var sessionId = ExtractAndRegisterSessionId(createResult);

        var conclusionText = "Completed with insights about [[sequential-thinking]] " +
                             "and [[workflow]] mechanics. Key findings: [[maenifold]] " +
                             "enables [[session-management]] at scale.";

        // Act - Complete with a rich conclusion
        WorkflowTools.Workflow(
            sessionId: sessionId,
            response: "Step 1 done with [[workflow]].",
            status: "completed",
            conclusion: conclusionText);

        // Assert - Verify conclusion content is in the session file
        var (_, content, _) = MarkdownIO.ReadSession("workflow", sessionId);
        Assert.That(content, Does.Contain("Conclusion"), "Should have Conclusion heading");
        Assert.That(content, Does.Contain("[[sequential-thinking]]"), "Should preserve WikiLink 1");
        Assert.That(content, Does.Contain("[[workflow]]"), "Should preserve WikiLink 2");
        Assert.That(content, Does.Contain("[[maenifold]]"), "Should preserve WikiLink 3");
        Assert.That(content, Does.Contain("[[session-management]]"), "Should preserve WikiLink 4");
    }

    // ============ Invalid Parameter Combinations ============

    /// <summary>
    /// Test 21: Calling Workflow() with no parameters at all throws InvalidOperationException.
    /// Verifies that the catch-all error in DispatchWorkflowOperation is reachable.
    /// </summary>
    [Test]
    public void WorkflowWithNoParametersThrowsInvalidOperationException()
    {
        // T-COV-001.4: RTM FR-17.6
        // Act & Assert
        var ex = Assert.Throws<InvalidOperationException>(() =>
            WorkflowTools.Workflow());

        Assert.That(ex.Message, Does.Contain("VALID OPERATIONS"), "Error should list valid operations");
        Assert.That(ex.Message, Does.Contain("workflowId"), "Should mention workflowId");
        Assert.That(ex.Message, Does.Contain("sessionId"), "Should mention sessionId");
    }

    /// <summary>
    /// Test 22: Calling Workflow() with only sessionId (no response, view, or append)
    /// throws InvalidOperationException with helpful message.
    /// </summary>
    [Test]
    public void WorkflowWithOnlySessionIdThrowsInvalidOperationException()
    {
        // T-COV-001.4: RTM FR-17.6
        // Arrange - Create a real session to use a valid sessionId
        var createResult = WorkflowTools.Workflow(workflowId: "wtt-alpha");
        var sessionId = ExtractAndRegisterSessionId(createResult);

        // Act & Assert - sessionId alone (no response, no view, no append) is invalid
        var ex = Assert.Throws<InvalidOperationException>(() =>
            WorkflowTools.Workflow(sessionId: sessionId));

        Assert.That(ex.Message, Does.Contain("VALID OPERATIONS"), "Error should list valid operations");
    }
}
