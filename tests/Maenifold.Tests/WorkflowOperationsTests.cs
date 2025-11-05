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
/// Baseline tests for WorkflowOperations (via WorkflowTools public API).
/// Tests Append() and Continue() methods through public Workflow() entry point.
/// Creates isolated test workflow sessions - NEVER touches production workflows.
/// Uses real MarkdownIO and JSON workflow files (NO MOCKS) per [[testing-philosophy]].
/// </summary>
public class WorkflowOperationsTests
{
    private const string TestFolder = "workflow-operations-tests";
    private static readonly JsonSerializerOptions JsonOptions = new() { WriteIndented = true };
    private static readonly string[] TestWorkflowNames = new[] { "test-workflow-1", "test-workflow-2", "test-workflow-3" };
    private static readonly string[] AppendWorkflows = new[] { "test-workflow-2", "test-workflow-3" };
    private static readonly string[] MixedArrayWorkflows = new[] { "test-workflow-2", "nonexistent-workflow-xyz" };
    private string _testFolderPath = string.Empty;
    private string _testWorkflowsPath = string.Empty;
    private string _testMemoryPath = string.Empty;
    private List<string> _createdSessions = new();

    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();
        _testFolderPath = Path.Combine(Config.MemoryPath, TestFolder);
        _testMemoryPath = Config.MemoryPath;
        _testWorkflowsPath = WorkflowCommon.WorkflowsPath;

        // Create test folder structure
        Directory.CreateDirectory(_testFolderPath);
        Directory.CreateDirectory(Path.Combine(_testMemoryPath, "thinking", "workflow"));

        // Ensure all parent directories exist for workflows path
        var workflowsParentPath = Path.GetDirectoryName(_testWorkflowsPath);
        if (!string.IsNullOrEmpty(workflowsParentPath))
        {
            Directory.CreateDirectory(workflowsParentPath);
        }
        Directory.CreateDirectory(_testWorkflowsPath);

        // Create minimal test workflow files in real workflows directory
        CreateTestWorkflow("test-workflow-1", 3);
        CreateTestWorkflow("test-workflow-2", 2);
        CreateTestWorkflow("test-workflow-3", 2);
    }

    [TearDown]
    public void TearDown()
    {
        if (string.IsNullOrEmpty(_testFolderPath))
            return;

        // Delete all test sessions created during tests
        foreach (var sessionId in _createdSessions)
        {
            var sessionPath = Path.Combine(_testMemoryPath, "thinking", "workflow", $"{sessionId}.md");
            if (File.Exists(sessionPath))
            {
                File.Delete(sessionPath);
            }
        }

        // Delete test workflow files
        foreach (var workflowName in TestWorkflowNames)
        {
            var wfPath = Path.Combine(_testWorkflowsPath, $"{workflowName}.json");
            if (File.Exists(wfPath))
            {
                File.Delete(wfPath);
            }
        }

        // Delete test folder
        var directory = new DirectoryInfo(_testFolderPath);
        if (directory.Exists)
        {
            directory.Delete(true);
        }
    }

    /// <summary>
    /// Creates a minimal test workflow JSON file with specified number of steps.
    /// Format: name, steps array with id, name, description fields.
    /// </summary>
    private void CreateTestWorkflow(string workflowName, int stepCount)
    {
        var steps = new List<JsonElement>();
        for (int i = 0; i < stepCount; i++)
        {
            steps.Add(JsonDocument.Parse($@"{{""id"": ""step-{i+1}"", ""name"": ""Step {i+1}"", ""description"": ""Test step {i+1}""}}").RootElement);
        }

        var workflow = new
        {
            id = workflowName,
            name = workflowName,
            steps = steps
        };

        var path = Path.Combine(_testWorkflowsPath, $"{workflowName}.json");
        File.WriteAllText(path, JsonSerializer.Serialize(workflow, JsonOptions));
    }

    /// <summary>
    /// Helper: Creates a workflow session in isolated test memory folder.
    /// Returns sessionId for cleanup tracking.
    /// </summary>
    private string CreateTestSession(params string[] workflowIds)
    {
        var sessionId = $"test-session-{DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()}";
        _createdSessions.Add(sessionId);

        var frontmatter = new Dictionary<string, object>
        {
            ["title"] = $"Workflow Session {sessionId}",
            ["permalink"] = sessionId,
            ["type"] = "workflow",
            ["status"] = "active",
            ["queue"] = workflowIds.Cast<object>().ToArray(), // Cast to object[] to match production code expectations
            ["currentWorkflow"] = 0,
            ["currentStep"] = 0
        };

        MarkdownIO.CreateSession("workflow", sessionId, frontmatter, "# Workflow Session\n\nTest workflow.");
        return sessionId;
    }

    // ============ Append() Tests (5 tests) ============

    /// <summary>
    /// Test 1: Append valid single workflow to session queue.
    /// Verifies successful queue update and confirmation message.
    /// </summary>
    [Test]
    public void AppendValidWorkflowToQueue()
    {
        // Arrange
        var sessionId = CreateTestSession("test-workflow-1");

        // Act
        var result = WorkflowOperations.Append(sessionId, "test-workflow-2");

        // Assert
        Assert.That(result, Does.Contain("Added 1 workflow(s) to queue"), "Should confirm single workflow added");
        Assert.That(result, Does.Contain("test-workflow-2"), "Should show added workflow in result");
        Assert.That(result, Does.Not.Contain("ERROR"), "Should not error on valid append");

        // Verify session was updated
        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        Assert.That(frontmatter, Is.Not.Null);
        if (frontmatter != null)
        {
            var queueObj = frontmatter["queue"];
            Assert.That(queueObj, Is.Not.Null, "Queue should not be null");

            string[] queue;
            if (queueObj is string[] strArray)
                queue = strArray;
            else if (queueObj is object[] objArray)
                queue = objArray.Select(x => x?.ToString() ?? "").ToArray();
            else if (queueObj is List<object> list)
                queue = list.Select(x => x?.ToString() ?? "").ToArray();
            else
            {
                Assert.Fail($"Unexpected queue type: {queueObj.GetType().Name}");
                queue = Array.Empty<string>();
            }

            Assert.That(queue.Length, Is.EqualTo(2), "Queue should contain 2 workflows");
            Assert.That(queue[1], Is.EqualTo("test-workflow-2"), "Second item should be appended workflow");
        }
    }

    /// <summary>
    /// Test 2: Append with invalid sessionId returns error.
    /// Verifies error handling for nonexistent session.
    /// </summary>
    [Test]
    public void AppendWithInvalidSessionIdReturnsError()
    {
        // Arrange - Use numeric session ID that doesn't exist (valid timestamp format but nonexistent)
        var invalidSessionId = "1000000000000";

        // Act
        var result = WorkflowOperations.Append(invalidSessionId, "test-workflow-1");

        // Assert
        Assert.That(result, Does.Contain("ERROR"), "Should return error message");
        Assert.That(result, Does.Contain(invalidSessionId), "Error should reference session ID");
        Assert.That(result, Does.Contain("doesn't exist"), "Should indicate session doesn't exist");
    }

    /// <summary>
    /// Test 3: Append with malformed workflow name (missing file) returns error.
    /// Verifies validation of workflow existence before queue update.
    /// </summary>
    [Test]
    public void AppendWithMissingWorkflowFileReturnsError()
    {
        // Arrange
        var sessionId = CreateTestSession("test-workflow-1");
        var nonexistentWorkflow = "nonexistent-workflow-xyz";

        // Act
        var result = WorkflowOperations.Append(sessionId, nonexistentWorkflow);

        // Assert
        Assert.That(result, Does.Contain("ERROR"), "Should return error for missing workflow");
        Assert.That(result, Does.Contain(nonexistentWorkflow), "Error should reference missing workflow");
        Assert.That(result, Does.Contain("not found"), "Should indicate workflow not found");

        // Verify session queue was NOT modified
        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (frontmatter != null)
        {
            var queueObj = frontmatter["queue"];
            string[] queue;
            if (queueObj is object[] objArray)
                queue = objArray.Select(x => x?.ToString() ?? "").ToArray();
            else if (queueObj is List<object> list)
                queue = list.Select(x => x?.ToString() ?? "").ToArray();
            else
                queue = Array.Empty<string>();

            Assert.That(queue.Length, Is.EqualTo(1), "Queue should still contain only original workflow");
        }
    }

    /// <summary>
    /// Test 4: Append multiple workflows as JSON array.
    /// Verifies batch append functionality with queue ordering.
    /// </summary>
    [Test]
    public void AppendMultipleWorkflowsAsJsonArray()
    {
        // Arrange
        var sessionId = CreateTestSession("test-workflow-1");
        var jsonArray = JsonSerializer.Serialize(AppendWorkflows);

        // Act
        var result = WorkflowOperations.Append(sessionId, jsonArray);

        // Assert
        Assert.That(result, Does.Contain("Added 2 workflow(s) to queue"), "Should confirm two workflows added");
        Assert.That(result, Does.Contain("test-workflow-2"), "Should show appended workflows in result");
        Assert.That(result, Does.Contain("test-workflow-3"), "Should show appended workflows in result");
        Assert.That(result, Does.Contain("New queue:"), "Should show full queue in result");

        // Verify queue order
        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (frontmatter != null)
        {
            var queueObj = frontmatter["queue"];
            Assert.That(queueObj, Is.Not.Null, "Queue should not be null");

            string[] queue;
            if (queueObj is string[] strArray)
                queue = strArray;
            else if (queueObj is object[] objArray)
                queue = objArray.Select(x => x?.ToString() ?? "").ToArray();
            else if (queueObj is List<object> list)
                queue = list.Select(x => x?.ToString() ?? "").ToArray();
            else
            {
                Assert.Fail($"Unexpected queue type: {queueObj.GetType().Name}");
                queue = Array.Empty<string>();
            }

            Assert.That(queue.Length, Is.EqualTo(3), "Queue should contain 3 workflows");
            Assert.That(queue[0], Is.EqualTo("test-workflow-1"), "First item should be original");
            Assert.That(queue[1], Is.EqualTo("test-workflow-2"), "Second item correct");
            Assert.That(queue[2], Is.EqualTo("test-workflow-3"), "Third item correct");
        }
    }

    /// <summary>
    /// Test 5: Append validates all workflows before updating queue (atomic operation).
    /// Verifies that partial failures don't corrupt queue state.
    /// </summary>
    [Test]
    public void AppendValidatesAllWorkflowsBeforeQueueUpdate()
    {
        // Arrange
        var sessionId = CreateTestSession("test-workflow-1");
        var mixedArray = JsonSerializer.Serialize(MixedArrayWorkflows);

        // Act
        var result = WorkflowOperations.Append(sessionId, mixedArray);

        // Assert
        Assert.That(result, Does.Contain("ERROR"), "Should fail due to missing workflow");
        Assert.That(result, Does.Contain("nonexistent-workflow-xyz"), "Error should name missing workflow");

        // Verify queue was NOT modified (atomic)
        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (frontmatter != null)
        {
            var queueObj = frontmatter["queue"];
            string[] queue;
            if (queueObj is object[] objArray)
                queue = objArray.Select(x => x?.ToString() ?? "").ToArray();
            else if (queueObj is List<object> list)
                queue = list.Select(x => x?.ToString() ?? "").ToArray();
            else
                queue = Array.Empty<string>();

            Assert.That(queue.Length, Is.EqualTo(1), "Queue should be unchanged after partial validation failure");
        }
    }

    // ============ Continue() Tests (10-15 tests) ============

    /// <summary>
    /// Test 6: Continue to next step within same workflow.
    /// Verifies currentStep increment, response recording, and step display.
    /// </summary>
    [Test]
    public void ContinueToNextStepInSameWorkflow()
    {
        // Arrange
        var sessionId = CreateTestSession("test-workflow-1");
        var response = "Completed first step with [[analysis]] and [[findings]].";

        // Act
        var result = WorkflowOperations.Continue(sessionId, response, null, null);

        // Assert
        Assert.That(result, Does.Contain("Step 2/3"), "Should display next step");
        Assert.That(result, Does.Not.Contain("ERROR"), "Should succeed");

        // Verify session state updated
        var (frontmatter, content, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (frontmatter != null)
        {
            var currentStep = Convert.ToInt32(frontmatter["currentStep"], CultureInfo.InvariantCulture);
            Assert.That(currentStep, Is.EqualTo(1), "currentStep should increment to 1");
            Assert.That(content, Does.Contain("Step 1/3 Response"), "Should record response with step heading");
            Assert.That(content, Does.Contain("[[analysis]]"), "Should preserve [[concepts]]");
        }
    }

    /// <summary>
    /// Test 7: Continue to next workflow when current workflow completes.
    /// Verifies workflow queue advancement and step reset.
    /// </summary>
    [Test]
    public void ContinueToNextWorkflowWhenCurrentCompletes()
    {
        // Arrange - Session with 2 steps in first workflow
        var sessionId = CreateTestSession("test-workflow-3", "test-workflow-2");
        var response = "Step 1 done with [[progress]].";

        // Act - Advance to complete first workflow
        var result1 = WorkflowOperations.Continue(sessionId, response, null, null);
        Assert.That(result1, Does.Contain("Step 2/2"), "First response should show next step");

        var response2 = "Step 2 done with [[completion]].";
        var result2 = WorkflowOperations.Continue(sessionId, response2, null, null);

        // Assert - Should advance to next workflow
        Assert.That(result2, Does.Contain("Step 1/2"), "Should advance to first step of next workflow");
        Assert.That(result2, Does.Not.Contain("ERROR"), "Should succeed in transition");

        // Verify session state
        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (frontmatter != null)
        {
            var currentWorkflow = Convert.ToInt32(frontmatter["currentWorkflow"], CultureInfo.InvariantCulture);
            var currentStep = Convert.ToInt32(frontmatter["currentStep"], CultureInfo.InvariantCulture);
            Assert.That(currentWorkflow, Is.EqualTo(1), "currentWorkflow should increment");
            Assert.That(currentStep, Is.EqualTo(0), "currentStep should reset to 0");
        }
    }

    /// <summary>
    /// Test 8: Continue with 'completed' status finalizes session.
    /// Verifies proper conclusion requirement and status update.
    /// </summary>
    [Test]
    public void ContinueWithCompletionStatusFinalizesSession()
    {
        // Arrange
        var sessionId = CreateTestSession("test-workflow-1");
        var response = "Step 1 work with [[findings]].";
        var conclusion = "Session complete with [[summary]] and [[learning]].";

        // Act - First record response
        WorkflowOperations.Continue(sessionId, response, null, null);
        // Then complete
        var result = WorkflowOperations.Continue(sessionId, response, null, "completed", conclusion);

        // Assert
        Assert.That(result, Does.Contain("✅"), "Should show completion emoji");
        Assert.That(result, Does.Contain("completed"), "Should confirm completion");

        // Verify frontmatter updated
        var (frontmatter, content, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (frontmatter != null)
        {
            Assert.That(frontmatter["status"], Is.EqualTo("completed"), "Status should be completed");
            Assert.That(frontmatter?["completed"], Is.Not.Null, "Should have completion timestamp");
            Assert.That(content, Does.Contain("Conclusion"), "Should include conclusion section");
            Assert.That(content, Does.Contain("[[summary]]"), "Should preserve conclusion [[concepts]]");
        }
    }

    /// <summary>
    /// Test 9: Continue with 'cancelled' status terminates session early.
    /// Verifies cancellation handling and status flags.
    /// </summary>
    [Test]
    public void ContinueWithCancellationStatusTerminatesEarly()
    {
        // Arrange
        var sessionId = CreateTestSession("test-workflow-1");
        var response = "Cancelled work with [[reason]].";

        // Act
        var result = WorkflowOperations.Continue(sessionId, response, null, "cancelled");

        // Assert
        Assert.That(result, Does.Contain("❌"), "Should show cancellation emoji");
        Assert.That(result, Does.Contain("cancelled"), "Should confirm cancellation");

        // Verify frontmatter updated
        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (frontmatter != null)
        {
            Assert.That(frontmatter["status"], Is.EqualTo("cancelled"), "Status should be cancelled");
            Assert.That(frontmatter?["cancelled"], Is.Not.Null, "Should have cancellation timestamp");
        }
    }

    /// <summary>
    /// Test 10: Continue without required [[concepts]] returns error.
    /// Verifies knowledge graph integration requirement enforcement.
    /// </summary>
    [Test]
    public void ContinueWithoutConceptsReturnsError()
    {
        // Arrange
        var sessionId = CreateTestSession("test-workflow-1");
        var responseWithoutConcepts = "This response has no double-bracket concepts.";

        // Act
        var result = WorkflowOperations.Continue(sessionId, responseWithoutConcepts, null, null);

        // Assert
        Assert.That(result, Does.Contain("ERROR"), "Should require concepts");
        Assert.That(result, Does.Contain("[[concept]]"), "Error should show concept format");
        Assert.That(result, Does.Contain("knowledge graph"), "Should mention knowledge graph");
    }

    /// <summary>
    /// Test 11: Continue with invalid sessionId returns error or throws exception.
    /// Verifies session existence validation. Invalid session IDs may throw format exceptions.
    /// </summary>
    [Test]
    public void ContinueWithInvalidSessionIdReturnsError()
    {
        // Arrange
        var invalidSessionId = "workflow-1234"; // Valid format but nonexistent
        var response = "Response with [[concept]].";

        // Act
        var result = WorkflowOperations.Continue(invalidSessionId, response, null, null);

        // Assert
        Assert.That(result, Does.Contain("ERROR"), "Should return error");
        Assert.That(result, Does.Contain(invalidSessionId), "Error should reference session ID");
        Assert.That(result, Does.Contain("doesn't exist"), "Should indicate nonexistent session");
    }

    /// <summary>
    /// Test 12: Continue creates proper frontmatter with timestamps.
    /// Verifies metadata preservation during state transitions.
    /// </summary>
    [Test]
    public void ContinueCreatesProperFrontmatterMetadata()
    {
        // Arrange
        var sessionId = CreateTestSession("test-workflow-1");
        var response = "Work with [[progress]].";
        var timestampBefore = DateTime.UtcNow;

        // Act
        WorkflowOperations.Continue(sessionId, response, null, null);

        // Assert
        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (frontmatter != null)
        {
            Assert.That(frontmatter["title"], Is.Not.Null);
            Assert.That(frontmatter["type"], Is.EqualTo("workflow"));
            Assert.That(frontmatter["permalink"], Is.EqualTo(sessionId));
            Assert.That(frontmatter["status"], Is.EqualTo("active"));
            Assert.That(frontmatter["queue"], Is.Not.Null, "Queue should be preserved");
            Assert.That(frontmatter["currentWorkflow"], Is.Not.Null, "Workflow index should exist");
            Assert.That(frontmatter["currentStep"], Is.Not.Null, "Step index should exist");
        }
    }

    /// <summary>
    /// Test 13: Continue preserves thoughts when provided.
    /// Verifies optional thoughts parameter is recorded.
    /// </summary>
    [Test]
    public void ContinuePreservesThoughtsInContent()
    {
        // Arrange
        var sessionId = CreateTestSession("test-workflow-1");
        var response = "Response with [[findings]].";
        var thoughts = "Meta thoughts with [[insight]] and [[reflection]].";

        // Act
        WorkflowOperations.Continue(sessionId, response, thoughts, null);

        // Assert
        var (_, content, _) = MarkdownIO.ReadSession("workflow", sessionId);
        Assert.That(content, Does.Contain(response), "Should contain response");
        Assert.That(content, Does.Contain(thoughts), "Should contain thoughts");
        Assert.That(content, Does.Contain("Thoughts:"), "Should label thoughts section");
        Assert.That(content, Does.Contain("[[insight]]"), "Should preserve thought [[concepts]]");
    }

    /// <summary>
    /// Test 14: Continue requires conclusion when completing all workflows in queue.
    /// Verifies proper error when queue exhausted without conclusion.
    /// </summary>
    [Test]
    public void ContinueRequiresConclustionWhenQueueExhausted()
    {
        // Arrange - Workflow with 2 steps
        var sessionId = CreateTestSession("test-workflow-3");
        var response = "Work with [[done]].";

        // Act - Complete step 1, then step 2
        WorkflowOperations.Continue(sessionId, response, null, null); // Advances to step 2/2
        var result = WorkflowOperations.Continue(sessionId, response, null, null); // Completes all workflows, should error

        // Assert
        Assert.That(result, Does.Contain("ERROR"), "Should error when queue exhausted without conclusion");
        Assert.That(result, Does.Contain("All workflows complete"), "Should mention queue exhaustion");
        Assert.That(result, Does.Contain("conclusion"), "Should require conclusion parameter");
    }

    /// <summary>
    /// Test 15: Continue updates currentStep index correctly.
    /// Verifies state tracking through multiple steps.
    /// </summary>
    [Test]
    public void ContinueUpdatesCurrentStepIndexCorrectly()
    {
        // Arrange - Queue with 2 workflows to test transition
        var sessionId = CreateTestSession("test-workflow-1", "test-workflow-2");

        // Act & Assert - Verify step progression
        var response = "Step work with [[concept]].";

        // Initial state should be step 0
        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (frontmatter != null)
        {
            var currentStep = Convert.ToInt32(frontmatter["currentStep"], CultureInfo.InvariantCulture);
            Assert.That(currentStep, Is.EqualTo(0), "Initial step should be 0");
        }

        // After first continue
        WorkflowOperations.Continue(sessionId, response, null, null);
        var (fm1, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (fm1 != null)
        {
            var currentStep1 = Convert.ToInt32(fm1["currentStep"], CultureInfo.InvariantCulture);
            Assert.That(currentStep1, Is.EqualTo(1), "After continue, step should be 1");
        }

        // After second continue
        WorkflowOperations.Continue(sessionId, response, null, null);
        var (fm2, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (fm2 != null)
        {
            var currentStep2 = Convert.ToInt32(fm2["currentStep"], CultureInfo.InvariantCulture);
            Assert.That(currentStep2, Is.EqualTo(2), "After second continue, step should be 2");
        }

        // On final step of first workflow, should transition to next workflow
        WorkflowOperations.Continue(sessionId, response, null, null);
        var (fm3, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (fm3 != null)
        {
            var currentStep3 = Convert.ToInt32(fm3["currentStep"], CultureInfo.InvariantCulture);
            var currentWorkflow3 = Convert.ToInt32(fm3["currentWorkflow"], CultureInfo.InvariantCulture);
            Assert.That(currentStep3, Is.EqualTo(0), "After workflow completion, step resets");
            Assert.That(currentWorkflow3, Is.EqualTo(1), "Workflow index increments");
        }
    }

    /// <summary>
    /// Test 16: Continue completes all workflows with proper conclusion.
    /// Verifies final state when all workflows in queue are processed.
    /// </summary>
    [Test]
    public void ContinueCompletesAllWorkflowsWithConclusion()
    {
        // Arrange - Two short workflows
        var sessionId = CreateTestSession("test-workflow-3", "test-workflow-3");
        var response = "Step work with [[progress]].";
        var conclusion = "Final synthesis with [[summary]] and [[learning]].";

        // Act - Process all steps
        WorkflowOperations.Continue(sessionId, response, null, null); // First workflow step 1
        WorkflowOperations.Continue(sessionId, response, null, null); // First workflow step 2
        WorkflowOperations.Continue(sessionId, response, null, null); // Second workflow step 1
        WorkflowOperations.Continue(sessionId, response, null, null); // Second workflow step 2
        var result = WorkflowOperations.Continue(sessionId, response, null, null, conclusion); // Completion

        // Assert
        Assert.That(result, Does.Contain("✅"), "Should show completion emoji");
        Assert.That(result, Does.Contain("All workflows"), "Should indicate all workflows complete");

        // Verify final state
        var (frontmatter, content, _) = MarkdownIO.ReadSession("workflow", sessionId);
        if (frontmatter != null)
        {
            Assert.That(frontmatter["status"], Is.EqualTo("completed"), "Should be marked completed");
            Assert.That(content, Does.Contain("Conclusion"), "Should include conclusion");
            Assert.That(content, Does.Contain("[[summary]]"), "Should preserve conclusion [[concepts]]");
        }
    }

    /// <summary>
    /// Test 17: Continue with thoughts but no conclusion for intermediate steps.
    /// Verifies thoughts are optional, only response required at intermediate steps.
    /// </summary>
    [Test]
    public void ContinueWithThoughtsButNoConclusionAtIntermediateStep()
    {
        // Arrange
        var sessionId = CreateTestSession("test-workflow-1");
        var response = "Response with [[findings]].";
        var thoughts = "Optional thinking with [[analysis]].";

        // Act - Should succeed without conclusion at intermediate step
        var result = WorkflowOperations.Continue(sessionId, response, thoughts, null, null);

        // Assert
        Assert.That(result, Does.Not.Contain("ERROR"), "Should succeed with thoughts");
        Assert.That(result, Does.Contain("Step 2/3"), "Should advance to next step");

        var (_, content, _) = MarkdownIO.ReadSession("workflow", sessionId);
        Assert.That(content, Does.Contain(thoughts), "Should preserve thoughts");
    }

    // ============ View() Tests ============

    /// <summary>
    /// Test 18: View displays queue status with current position.
    /// Verifies view shows queue, current workflow/step, and next item.
    /// </summary>
    [Test]
    public void ViewDisplaysQueueStatusWithCurrentPosition()
    {
        // Arrange - Session with multiple workflows
        var sessionId = CreateTestSession("test-workflow-1", "test-workflow-2", "test-workflow-3");

        // Act
        var result = WorkflowOperations.View(sessionId);

        // Assert
        Assert.That(result, Does.Not.Contain("ERROR"), "Should succeed");
        Assert.That(result, Does.Contain("Queue:"), "Should show queue");
        Assert.That(result, Does.Contain("test-workflow-1"), "Should list workflow 1");
        Assert.That(result, Does.Contain("test-workflow-2"), "Should list workflow 2");
        Assert.That(result, Does.Contain("test-workflow-3"), "Should list workflow 3");
        Assert.That(result, Does.Contain("Position:"), "Should show current position");
        Assert.That(result, Does.Contain("workflow 1/3"), "Should show workflow index");
        Assert.That(result, Does.Contain("step 1/3"), "Should show step index");
    }

    /// <summary>
    /// Test 19: View with invalid sessionId returns error.
    /// Verifies error handling for nonexistent session.
    /// </summary>
    [Test]
    public void ViewWithInvalidSessionIdReturnsError()
    {
        // Arrange
        var invalidSessionId = "workflow-nonexistent-12345";

        // Act
        var result = WorkflowOperations.View(invalidSessionId);

        // Assert
        Assert.That(result, Does.Contain("ERROR"), "Should return error");
        Assert.That(result, Does.Contain(invalidSessionId), "Error should reference session ID");
        Assert.That(result, Does.Contain("doesn't exist"), "Should indicate session doesn't exist");
    }

    /// <summary>
    /// Test 20: View after progressing shows updated position.
    /// Verifies view reflects current session state correctly.
    /// </summary>
    [Test]
    public void ViewAfterProgressingShowsUpdatedPosition()
    {
        // Arrange - Create session and progress through steps
        var sessionId = CreateTestSession("test-workflow-1", "test-workflow-2");
        var response = "Step work with [[progress]].";

        // Progress to step 2
        WorkflowOperations.Continue(sessionId, response, null, null);

        // Act
        var result = WorkflowOperations.View(sessionId);

        // Assert
        Assert.That(result, Does.Not.Contain("ERROR"), "Should succeed");
        Assert.That(result, Does.Contain("step 2/3"), "Should show updated step position");
        Assert.That(result, Does.Contain("Next:"), "Should show next step info");
    }

    /// <summary>
    /// Test 21: View via WorkflowTools.Workflow() entry point.
    /// Verifies view parameter works through public API.
    /// </summary>
    [Test]
    public void ViewViaWorkflowToolsEntryPoint()
    {
        // Arrange
        var sessionId = CreateTestSession("test-workflow-1");

        // Act
        var result = WorkflowTools.Workflow(sessionId: sessionId, view: true);

        // Assert
        Assert.That(result, Does.Not.Contain("ERROR"), "Should succeed via public API");
        Assert.That(result, Does.Contain("Queue:"), "Should display queue information");
        Assert.That(result, Does.Contain("Position:"), "Should display position information");
    }

    /// <summary>
    /// Test 22: View without sessionId throws helpful error.
    /// Verifies error message explains sessionId requirement for view.
    /// </summary>
    [Test]
    public void ViewWithoutSessionIdThrowsHelpfulError()
    {
        // Act & Assert
        var ex = Assert.Throws<InvalidOperationException>(() => WorkflowTools.Workflow(view: true));
        Assert.That(ex.Message, Does.Contain("view"), "Error should mention view parameter");
        Assert.That(ex.Message, Does.Contain("sessionId"), "Error should mention sessionId requirement");
        Assert.That(ex.Message, Does.Contain("VALID OPERATIONS"), "Error should show valid operations");
    }
}
