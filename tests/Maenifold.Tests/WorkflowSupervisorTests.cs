// T-HSM-001.3: RTM FR-10.1, FR-10.3, FR-10.4 (remediation: HIGH-001, MEDIUM-001)
// Integration tests for Workflow supervisor state machine:
// - Workflow enters waiting state on submachine registration
// - Workflow blocks while submachine is active
// - Workflow unblocks when submachine reaches terminal state (completed/cancelled)
// - Backward compatibility: old sessions without 'phase' field behave as "running"
// - Frontmatter persistence: phase/activeSubmachineType/activeSubmachineSessionId written correctly
// Uses real filesystem and real session files (NO MOCKS) per [[testing-philosophy]].

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
/// Integration tests for the Workflow supervisor state machine (T-HSM-001).
/// Verifies FR-10.1 (supervisor manages submachine lifecycle), FR-10.3 (supervisor state
/// persisted in frontmatter), and FR-10.4 (step not advanced while waiting).
/// Uses real sequential thinking sessions as submachines per [[hierarchical-state-machines]].
/// </summary>
[NonParallelizable]
public class WorkflowSupervisorTests
{
    private static readonly JsonSerializerOptions JsonOptions = new() { WriteIndented = true };
    private static readonly string[] TestWorkflowNames = new[] { "wsup-alpha", "wsup-beta" };

    private string _testWorkflowsPath = string.Empty;
    private string _testMemoryPath = string.Empty;
    private List<string> _createdWorkflowSessions = new();
    private List<string> _createdThinkingSessions = new();

    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();
        _testMemoryPath = Config.MemoryPath;
        _testWorkflowsPath = WorkflowCommon.WorkflowsPath;

        Directory.CreateDirectory(_testWorkflowsPath);
        Directory.CreateDirectory(Path.Combine(_testMemoryPath, "thinking", "workflow"));
        Directory.CreateDirectory(Path.Combine(_testMemoryPath, "thinking", "sequential"));

        CreateTestWorkflow("wsup-alpha", 3);
        CreateTestWorkflow("wsup-beta", 2);
    }

    [TearDown]
    public void TearDown()
    {
        foreach (var sessionId in _createdWorkflowSessions)
        {
            var sessionPath = MarkdownWriter.GetSessionPath("workflow", sessionId);
            if (File.Exists(sessionPath))
                File.Delete(sessionPath);
        }

        foreach (var sessionId in _createdThinkingSessions)
        {
            var sessionPath = MarkdownWriter.GetSessionPath("sequential", sessionId);
            if (File.Exists(sessionPath))
                File.Delete(sessionPath);
        }

        foreach (var workflowName in TestWorkflowNames)
        {
            var wfPath = Path.Combine(_testWorkflowsPath, $"{workflowName}.json");
            if (File.Exists(wfPath))
                File.Delete(wfPath);
        }
    }

    // ============ Helper Methods ============

    private void CreateTestWorkflow(string name, int stepCount)
    {
        var steps = Enumerable.Range(1, stepCount).Select(i => new
        {
            id = $"step-{i}",
            name = $"Step {i}",
            description = $"Test step {i} for [[hierarchical-state-machines]]"
        }).ToArray();

        var workflow = new
        {
            id = name,
            name = $"Test Supervisor Workflow {name}",
            description = $"Test workflow for [[workflow]] supervisor coverage",
            steps
        };

        File.WriteAllText(
            Path.Combine(_testWorkflowsPath, $"{name}.json"),
            JsonSerializer.Serialize(workflow, JsonOptions));
    }

    private string ExtractAndRegisterWorkflowSessionId(string result)
    {
        var lines = result.Split('\n', StringSplitOptions.RemoveEmptyEntries);
        var sessionLine = lines.FirstOrDefault(l => l.StartsWith("Created session:", StringComparison.Ordinal));
        Assert.That(sessionLine, Is.Not.Null, $"Could not find 'Created session:' in result:\n{result}");
        var sessionId = sessionLine!.Replace("Created session:", "").Trim();
        _createdWorkflowSessions.Add(sessionId);
        return sessionId;
    }

    /// <summary>
    /// Creates a real sequential thinking session file on disk with the given status.
    /// Returns the sessionId (e.g., "session-1234567890123-99999").
    /// </summary>
    private string CreateThinkingSession(string status = "active")
    {
        var timestamp = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        var suffix = new Random().Next(10000, 99999);
        var sessionId = $"session-{timestamp}-{suffix}";

        var frontmatter = new Dictionary<string, object>
        {
            ["title"] = $"Test Thinking Session {sessionId}",
            ["permalink"] = sessionId,
            ["type"] = "sequential",
            ["status"] = status,
            ["thoughtNumber"] = 1,
            ["totalThoughts"] = 3
        };

        var content = "# Test Thinking Session\n\nThought 1: Working on [[hierarchical-state-machines]] implementation.\n";

        MarkdownIO.CreateSession("sequential", sessionId, frontmatter, content);
        _createdThinkingSessions.Add(sessionId);
        return sessionId;
    }

    /// <summary>
    /// Updates the status field of a thinking session's frontmatter on disk.
    /// </summary>
    private static void UpdateThinkingSessionStatus(string sessionId, string newStatus)
    {
        var (frontmatter, content, _) = MarkdownIO.ReadSession("sequential", sessionId);
        Assert.That(frontmatter, Is.Not.Null, "Thinking session must be readable to update status");
        frontmatter!["status"] = newStatus;
        MarkdownIO.UpdateSession("sequential", sessionId, frontmatter, content);
    }

    // ============ FR-10.3: Start() writes supervisor frontmatter fields ============

    /// <summary>
    /// Test 1: Workflow Start() SHALL write phase, activeSubmachineType, and activeSubmachineSessionId
    /// to session frontmatter (FR-10.3).
    /// </summary>
    [Test]
    public void WorkflowStartWritesSupervisorFrontmatterFields()
    {
        // T-HSM-001.1: RTM FR-10.3
        var result = WorkflowTools.Workflow(workflowId: "wsup-alpha");
        var sessionId = ExtractAndRegisterWorkflowSessionId(result);

        var (frontmatter, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        Assert.That(frontmatter, Is.Not.Null, "Session should have frontmatter");
        Assert.That(frontmatter!.ContainsKey("phase"), Is.True, "Should have 'phase' field");
        Assert.That(frontmatter!["phase"]?.ToString(), Is.EqualTo("running"), "Initial phase should be 'running'");
        Assert.That(frontmatter!.ContainsKey("activeSubmachineType"), Is.True, "Should have 'activeSubmachineType' field");
        Assert.That(frontmatter!["activeSubmachineType"]?.ToString(), Is.EqualTo(""), "Initial activeSubmachineType should be empty");
        Assert.That(frontmatter!.ContainsKey("activeSubmachineSessionId"), Is.True, "Should have 'activeSubmachineSessionId' field");
        Assert.That(frontmatter!["activeSubmachineSessionId"]?.ToString(), Is.EqualTo(""), "Initial activeSubmachineSessionId should be empty");
    }

    // ============ FR-10.1 + FR-10.3: Register path — entering waiting state ============

    /// <summary>
    /// Test 2: Providing submachineSessionId during Continue() SHALL write phase=waiting,
    /// activeSubmachineType, and activeSubmachineSessionId to frontmatter and NOT advance step (FR-10.1, FR-10.3, FR-10.4).
    /// </summary>
    [Test]
    public void RegisterSubmachineEntersWaitingStateWithoutAdvancingStep()
    {
        // T-HSM-001.1b: RTM FR-10.1, FR-10.3, FR-10.4
        var workflowResult = WorkflowTools.Workflow(workflowId: "wsup-alpha");
        var workflowSessionId = ExtractAndRegisterWorkflowSessionId(workflowResult);

        var thinkingSessionId = CreateThinkingSession("active");

        // Read currentStep before registration
        var (fmBefore, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        var stepBefore = Convert.ToInt32(fmBefore!["currentStep"], CultureInfo.InvariantCulture);

        // Register the submachine
        var registerResult = WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Starting analysis with [[hierarchical-state-machines]] and [[sequential-thinking]].",
            submachineSessionId: thinkingSessionId);

        // Result should say "Registered submachine"
        Assert.That(registerResult, Does.Contain("Registered submachine"), "Should confirm registration");
        Assert.That(registerResult, Does.Contain(thinkingSessionId), "Should name the registered session");
        Assert.That(registerResult, Does.Contain("waiting"), "Should mention waiting state");

        // Verify frontmatter
        var (fmAfter, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(fmAfter, Is.Not.Null);
        Assert.That(fmAfter!["phase"]?.ToString(), Is.EqualTo("waiting"), "Phase should be 'waiting'");
        Assert.That(fmAfter!["activeSubmachineType"]?.ToString(), Is.EqualTo("sequential"), "Type should be 'sequential'");
        Assert.That(fmAfter!["activeSubmachineSessionId"]?.ToString(), Is.EqualTo(thinkingSessionId), "Should store session ID");

        // currentStep must NOT have advanced (FR-10.4)
        var stepAfter = Convert.ToInt32(fmAfter!["currentStep"], CultureInfo.InvariantCulture);
        Assert.That(stepAfter, Is.EqualTo(stepBefore), "currentStep MUST NOT advance while registering submachine");
    }

    // ============ FR-10.4: Gate path — block while submachine is active ============

    /// <summary>
    /// Test 3: While workflow is waiting on an active submachine, Continue() SHALL return
    /// a blocking message and NOT advance currentStep (FR-10.4).
    /// </summary>
    [Test]
    public void ContinueWhileWaitingOnActiveSubmachineBlocksAndDoesNotAdvanceStep()
    {
        // T-HSM-001.1c: RTM FR-10.4
        var workflowResult = WorkflowTools.Workflow(workflowId: "wsup-alpha");
        var workflowSessionId = ExtractAndRegisterWorkflowSessionId(workflowResult);

        var thinkingSessionId = CreateThinkingSession("active");

        // Register the submachine
        WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Registering submachine with [[state-machine]] analysis.",
            submachineSessionId: thinkingSessionId);

        // Read step after registration (should still be 0)
        var (fmAfterReg, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        var stepAfterReg = Convert.ToInt32(fmAfterReg!["currentStep"], CultureInfo.InvariantCulture);

        // Now try to continue the workflow while submachine is still active
        var blockResult = WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Trying to advance but submachine is still running [[workflow]].");

        // Should return a blocking message
        Assert.That(blockResult, Does.Contain("waiting on submachine"), "Should indicate waiting");
        Assert.That(blockResult, Does.Contain(thinkingSessionId), "Should name the submachine");
        Assert.That(blockResult, Does.Not.Contain("Step 2/"), "Should not advance to next step");

        // currentStep must not have advanced
        var (fmAfterBlock, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        var stepAfterBlock = Convert.ToInt32(fmAfterBlock!["currentStep"], CultureInfo.InvariantCulture);
        Assert.That(stepAfterBlock, Is.EqualTo(stepAfterReg), "Step must not advance while blocked");
    }

    // ============ FR-10.1: Gate path — unblock when submachine completes ============

    /// <summary>
    /// Test 4: When submachine reaches 'completed' terminal state, Continue() SHALL clear
    /// supervisor fields (phase=running, empty type/id) and resume normal step advancement (FR-10.1).
    /// </summary>
    [Test]
    public void ContinueAfterSubmachineCompletedClearsSupervisorFieldsAndResumes()
    {
        // T-HSM-001.1c: RTM FR-10.1, FR-10.3, FR-10.4
        var workflowResult = WorkflowTools.Workflow(workflowId: "wsup-alpha");
        var workflowSessionId = ExtractAndRegisterWorkflowSessionId(workflowResult);

        var thinkingSessionId = CreateThinkingSession("active");

        // Register submachine
        WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Registering [[sequential-thinking]] submachine.",
            submachineSessionId: thinkingSessionId);

        // Mark submachine as completed
        UpdateThinkingSessionStatus(thinkingSessionId, "completed");

        // Now continue the workflow — should unblock and advance
        var resumeResult = WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Resuming with [[hierarchical-state-machines]] after submachine completed.");

        // Should advance (not block)
        Assert.That(resumeResult, Does.Not.Contain("waiting on submachine"), "Should not be waiting anymore");
        Assert.That(resumeResult, Does.Not.Contain("ERROR"), "Should not error");
        Assert.That(resumeResult, Does.Contain("Step 2/"), "Should advance to next step");

        // Verify supervisor fields cleared
        var (fmAfter, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(fmAfter, Is.Not.Null);
        Assert.That(fmAfter!["phase"]?.ToString(), Is.EqualTo("running"), "Phase should be 'running' again");
        Assert.That(fmAfter!["activeSubmachineType"]?.ToString(), Is.EqualTo(""), "activeSubmachineType should be cleared");
        Assert.That(fmAfter!["activeSubmachineSessionId"]?.ToString(), Is.EqualTo(""), "activeSubmachineSessionId should be cleared");
    }

    /// <summary>
    /// Test 5: When submachine reaches 'cancelled' terminal state, Continue() SHALL unblock
    /// and resume normal workflow execution (FR-10.1).
    /// </summary>
    [Test]
    public void ContinueAfterSubmachineCancelledClearsSupervisorFieldsAndResumes()
    {
        // T-HSM-001.1c: RTM FR-10.1
        var workflowResult = WorkflowTools.Workflow(workflowId: "wsup-alpha");
        var workflowSessionId = ExtractAndRegisterWorkflowSessionId(workflowResult);

        var thinkingSessionId = CreateThinkingSession("active");

        // Register submachine
        WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Registering submachine for [[state-machine]] analysis.",
            submachineSessionId: thinkingSessionId);

        // Mark submachine as cancelled
        UpdateThinkingSessionStatus(thinkingSessionId, "cancelled");

        // Continue — should unblock
        var resumeResult = WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Resuming after [[sequential-thinking]] submachine was cancelled.");

        Assert.That(resumeResult, Does.Not.Contain("waiting on submachine"), "Should not be waiting");
        Assert.That(resumeResult, Does.Contain("Step 2/"), "Should advance to next step");

        // Verify cleared
        var (fmAfter, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(fmAfter!["phase"]?.ToString(), Is.EqualTo("running"), "Phase should be 'running'");
    }

    // ============ FR-10.2 + backward compatibility ============

    /// <summary>
    /// Test 6: A workflow session that was created without supervisor fields (simulates old sessions
    /// pre-FR-10.3) SHALL proceed normally as if phase="running" (backward compatibility).
    /// </summary>
    [Test]
    public void BackwardCompatibilityOldSessionWithoutPhaseFieldProceedsNormally()
    {
        // T-HSM-001.1d: RTM FR-10.2
        // Create a session without phase/activeSubmachineType/activeSubmachineSessionId
        var timestamp = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        var sessionId = $"workflow-{timestamp}";
        _createdWorkflowSessions.Add(sessionId);

        var frontmatterLegacy = new Dictionary<string, object>
        {
            ["title"] = $"Legacy Workflow Session {sessionId}",
            ["permalink"] = sessionId,
            ["type"] = "workflow",
            ["status"] = "active",
            ["queue"] = new[] { "wsup-alpha" },
            ["currentWorkflow"] = 0,
            ["currentStep"] = 0
            // NOTE: No 'phase', no 'activeSubmachineType', no 'activeSubmachineSessionId'
        };

        var content = "# Workflow Session\n\nLegacy session content with [[backward-compatibility]] test.\n";
        MarkdownIO.CreateSession("workflow", sessionId, frontmatterLegacy, content);

        // Verify no phase field
        var (fmLegacy, _, _) = MarkdownIO.ReadSession("workflow", sessionId);
        Assert.That(fmLegacy!.ContainsKey("phase"), Is.False, "Legacy session should not have 'phase' field");

        // Continue should work without error (treating missing phase as "running")
        var result = WorkflowTools.Workflow(
            sessionId: sessionId,
            response: "Step 1 done with [[workflow]] and [[backward-compatibility]] verification.");

        Assert.That(result, Does.Not.Contain("ERROR"), "Legacy session should work normally");
        Assert.That(result, Does.Contain("Step 2/"), "Should advance to step 2");
    }

    // ============ Register path: invalid submachine session ID ============

    /// <summary>
    /// Test 7: Providing a submachineSessionId that does not exist SHALL return an error.
    /// </summary>
    [Test]
    public void RegisterNonexistentSubmachineReturnsError()
    {
        // T-HSM-001.3: Security — invalid sessionId
        var workflowResult = WorkflowTools.Workflow(workflowId: "wsup-alpha");
        var workflowSessionId = ExtractAndRegisterWorkflowSessionId(workflowResult);

        var fakeSessionId = "session-9999999999999-00001";

        var result = WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Registering with [[hierarchical-state-machines]] context.",
            submachineSessionId: fakeSessionId);

        Assert.That(result, Does.Contain("ERROR"), "Should error for nonexistent submachine");
        Assert.That(result, Does.Contain(fakeSessionId), "Error should name the bad session ID");

        // Workflow should remain in running state (not enter waiting)
        var (fmAfter, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(fmAfter!.ContainsKey("phase") == false || fmAfter["phase"]?.ToString() != "waiting",
            "Workflow should not be waiting after failed registration");
    }

    // ============ Frontmatter persistence: content note appended ============

    /// <summary>
    /// Test 8: Register path SHALL append a note to the session markdown containing
    /// the submachine type and ID (FR-10.3).
    /// </summary>
    [Test]
    public void RegisterSubmachineAppendsNoteToSessionContent()
    {
        // T-HSM-001.1b: RTM FR-10.3
        var workflowResult = WorkflowTools.Workflow(workflowId: "wsup-alpha");
        var workflowSessionId = ExtractAndRegisterWorkflowSessionId(workflowResult);

        var thinkingSessionId = CreateThinkingSession("active");

        WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Registering submachine with [[workflow]] tracking.",
            submachineSessionId: thinkingSessionId);

        var (_, content, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(content, Does.Contain("Submachine Registered"), "Content should contain registration heading");
        Assert.That(content, Does.Contain(thinkingSessionId), "Content should contain the submachine session ID");
        Assert.That(content, Does.Contain("sequential"), "Content should note the submachine type");
    }

    /// <summary>
    /// Test 9: Unblock (submachine completed) SHALL append a resume note to session content.
    /// </summary>
    [Test]
    public void UnblockAfterCompletionAppendsResumeNoteToContent()
    {
        // T-HSM-001.1c: RTM FR-10.3
        var workflowResult = WorkflowTools.Workflow(workflowId: "wsup-alpha");
        var workflowSessionId = ExtractAndRegisterWorkflowSessionId(workflowResult);

        var thinkingSessionId = CreateThinkingSession("active");
        WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Registering with [[state-machine]] tracking.",
            submachineSessionId: thinkingSessionId);

        UpdateThinkingSessionStatus(thinkingSessionId, "completed");

        WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Resuming after submachine with [[hierarchical-state-machines]] done.");

        var (_, content, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(content, Does.Contain("Submachine Resumed"), "Should append resume heading");
        Assert.That(content, Does.Contain("completed"), "Should note submachine completion status");
    }

    // ============ HIGH-001: Abandoned submachine unblocks workflow ============

    /// <summary>
    /// Test 11 (HIGH-001 Part A): When submachine reaches 'abandoned' terminal state,
    /// Continue() SHALL clear supervisor fields and resume normal workflow execution (FR-10.1).
    /// </summary>
    [Test]
    public void AbandonedSubmachineUnblocksWorkflow()
    {
        // T-HSM-001.3: RTM FR-10.1
        var workflowResult = WorkflowTools.Workflow(workflowId: "wsup-alpha");
        var workflowSessionId = ExtractAndRegisterWorkflowSessionId(workflowResult);

        var thinkingSessionId = CreateThinkingSession("active");

        // Register the submachine
        WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Registering submachine for [[hierarchical-state-machines]] analysis.",
            submachineSessionId: thinkingSessionId);

        // Mark submachine as abandoned (the new terminal state under HIGH-001 fix)
        UpdateThinkingSessionStatus(thinkingSessionId, "abandoned");

        // Continue — should unblock and advance
        var resumeResult = WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Resuming after [[sequential-thinking]] submachine was abandoned.");

        Assert.That(resumeResult, Does.Not.Contain("waiting on submachine"), "Should not be waiting");
        Assert.That(resumeResult, Does.Not.Contain("ERROR"), "Should not error on abandoned submachine");
        Assert.That(resumeResult, Does.Contain("Step 2/"), "Should advance to next step");

        // Verify supervisor fields cleared
        var (fmAfter, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(fmAfter, Is.Not.Null);
        Assert.That(fmAfter!["phase"]?.ToString(), Is.EqualTo("running"), "Phase should be 'running' after abandoned submachine");
        Assert.That(fmAfter!["activeSubmachineType"]?.ToString(), Is.EqualTo(""), "activeSubmachineType should be cleared");
        Assert.That(fmAfter!["activeSubmachineSessionId"]?.ToString(), Is.EqualTo(""), "activeSubmachineSessionId should be cleared");
    }

    // ============ HIGH-001: Missing session file unblocks workflow ============

    /// <summary>
    /// Test 12 (HIGH-001 Part B): When the registered submachine session file has been deleted
    /// from disk, Continue() SHALL self-heal: clear waiting state and resume (not return ERROR).
    /// </summary>
    [Test]
    public void MissingSubmachineFileUnblocksWorkflow()
    {
        // T-HSM-001.3: RTM FR-10.1
        var workflowResult = WorkflowTools.Workflow(workflowId: "wsup-alpha");
        var workflowSessionId = ExtractAndRegisterWorkflowSessionId(workflowResult);

        var thinkingSessionId = CreateThinkingSession("active");

        // Register the submachine
        WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Registering submachine with [[state-machine]] and [[workflow]] tracking.",
            submachineSessionId: thinkingSessionId);

        // Verify we are in waiting state
        var (fmWaiting, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(fmWaiting!["phase"]?.ToString(), Is.EqualTo("waiting"), "Should be in waiting state after register");

        // Delete the submachine session file from disk (simulates expired/purged session)
        var submachineFilePath = MarkdownWriter.GetSessionPath("sequential", thinkingSessionId);
        Assert.That(File.Exists(submachineFilePath), Is.True, "Submachine file must exist before deletion");
        File.Delete(submachineFilePath);
        _createdThinkingSessions.Remove(thinkingSessionId); // Already deleted — skip TearDown cleanup

        // Continue should self-heal, NOT return an ERROR
        var resumeResult = WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Resuming after [[hierarchical-state-machines]] submachine was deleted.");

        Assert.That(resumeResult, Does.Not.Contain("ERROR"), "Should NOT error when submachine file is missing");
        Assert.That(resumeResult, Does.Not.Contain("waiting on submachine"), "Should not remain blocked");
        Assert.That(resumeResult, Does.Contain("Step 2/"), "Should advance to next step");

        // Verify supervisor fields cleared in frontmatter
        var (fmAfter, contentAfter, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(fmAfter, Is.Not.Null);
        Assert.That(fmAfter!["phase"]?.ToString(), Is.EqualTo("running"), "Phase should be 'running' after self-heal");
        Assert.That(fmAfter!["activeSubmachineType"]?.ToString(), Is.EqualTo(""), "activeSubmachineType should be cleared");
        Assert.That(fmAfter!["activeSubmachineSessionId"]?.ToString(), Is.EqualTo(""), "activeSubmachineSessionId should be cleared");

        // Verify the clearing note was appended to session content
        Assert.That(contentAfter, Does.Contain("Submachine Cleared"), "Content should include Submachine Cleared heading");
        Assert.That(contentAfter, Does.Contain(thinkingSessionId), "Cleared note should name the missing session");
        Assert.That(contentAfter, Does.Contain("deleted or expired"), "Cleared note should explain reason");
    }

    // ============ MEDIUM-001: Response persisted during register path ============

    /// <summary>
    /// Test 13 (MEDIUM-001): When registering a submachine, the response content SHALL be
    /// persisted to the session markdown file BEFORE entering waiting state. Step is NOT advanced.
    /// </summary>
    [Test]
    public void ResponsePersistedDuringRegister()
    {
        // T-HSM-001.3: RTM FR-10.3, FR-10.4
        var workflowResult = WorkflowTools.Workflow(workflowId: "wsup-alpha");
        var workflowSessionId = ExtractAndRegisterWorkflowSessionId(workflowResult);

        var thinkingSessionId = CreateThinkingSession("active");

        var distinctResponseText = $"Analysis complete for [[hierarchical-state-machines]] and [[workflow]] at step boundary.";

        // Read currentStep before
        var (fmBefore, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        var stepBefore = Convert.ToInt32(fmBefore!["currentStep"], CultureInfo.InvariantCulture);

        // Register with a response containing WikiLinks
        WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: distinctResponseText,
            submachineSessionId: thinkingSessionId);

        // Read session content after registration
        var (fmAfter, contentAfter, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);

        // Response text MUST appear in session content
        Assert.That(contentAfter, Does.Contain("Analysis complete for"), "Response text must be written to session during register path");
        Assert.That(contentAfter, Does.Contain("[[hierarchical-state-machines]]"), "WikiLinks in response must be preserved");

        // currentStep MUST NOT have advanced (FR-10.4)
        var stepAfter = Convert.ToInt32(fmAfter!["currentStep"], CultureInfo.InvariantCulture);
        Assert.That(stepAfter, Is.EqualTo(stepBefore), "Step must NOT advance during register path");

        // Phase must be waiting
        Assert.That(fmAfter["phase"]?.ToString(), Is.EqualTo("waiting"), "Phase must be 'waiting' after register");
    }

    // ============ WikiLink validation still applies on register path ============

    /// <summary>
    /// Test 10: Register path SHALL still enforce WikiLink validation on the response parameter.
    /// </summary>
    [Test]
    public void RegisterPathEnforcesWikiLinkValidationOnResponse()
    {
        // T-HSM-001.1b: WikiLink validation per constraint
        var workflowResult = WorkflowTools.Workflow(workflowId: "wsup-alpha");
        var workflowSessionId = ExtractAndRegisterWorkflowSessionId(workflowResult);

        var thinkingSessionId = CreateThinkingSession("active");

        // Response without WikiLinks
        var result = WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "No double bracket concepts in this response at all.",
            submachineSessionId: thinkingSessionId);

        Assert.That(result, Does.Contain("ERROR"), "Should error when no WikiLinks in response");
        Assert.That(result, Does.Contain("[[WikiLink]]"), "Error should show expected format");

        // Workflow should NOT be in waiting state
        var (fmAfter, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        var phase = fmAfter!.TryGetValue("phase", out var phaseObj) ? phaseObj?.ToString() : "running";
        Assert.That(phase, Is.Not.EqualTo("waiting"), "Workflow should not enter waiting on failed validation");
    }

    // ============ FR-10.1: Re-registration cycle — multiple sequential submachines ============

    /// <summary>
    /// Test 14: A workflow SHALL support multiple sequential submachine registrations across its lifecycle.
    /// Register A → complete A → resume → register B → complete B → resume (FR-10.1).
    /// </summary>
    [Test]
    public void ReRegistrationCycleWorksAcrossMultipleSubmachines()
    {
        // T-HSM-001.3: RTM FR-10.1
        var workflowResult = WorkflowTools.Workflow(workflowId: "wsup-alpha");
        var workflowSessionId = ExtractAndRegisterWorkflowSessionId(workflowResult);

        // --- Phase 1: Register submachine A ---
        var thinkingSessionIdA = CreateThinkingSession("active");

        var (fmBeforeA, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        var stepBeforeA = Convert.ToInt32(fmBeforeA!["currentStep"], CultureInfo.InvariantCulture);

        WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Starting [[hierarchical-state-machines]] analysis with submachine A.",
            submachineSessionId: thinkingSessionIdA);

        // After registering A: phase=waiting, step not advanced
        var (fmAfterRegA, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(fmAfterRegA!["phase"]?.ToString(), Is.EqualTo("waiting"), "Phase should be 'waiting' after registering A");
        var stepAfterRegA = Convert.ToInt32(fmAfterRegA!["currentStep"], CultureInfo.InvariantCulture);
        Assert.That(stepAfterRegA, Is.EqualTo(stepBeforeA), "Step must NOT advance while registering submachine A");

        // --- Phase 2: Complete A and resume ---
        UpdateThinkingSessionStatus(thinkingSessionIdA, "completed");

        var resumeResultA = WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Resuming [[workflow]] after submachine A completed — advancing step.");

        // After completing A and resuming: phase=running, step advances to 2/3
        Assert.That(resumeResultA, Does.Not.Contain("waiting on submachine"), "Should not be blocked after A completes");
        Assert.That(resumeResultA, Does.Contain("Step 2/"), "Should advance to step 2 after A completes");

        var (fmAfterResumeA, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(fmAfterResumeA!["phase"]?.ToString(), Is.EqualTo("running"), "Phase should be 'running' after A completes");
        Assert.That(fmAfterResumeA!["activeSubmachineSessionId"]?.ToString(), Is.EqualTo(""), "activeSubmachineSessionId should be cleared after A");

        var stepAfterResumeA = Convert.ToInt32(fmAfterResumeA!["currentStep"], CultureInfo.InvariantCulture);

        // --- Phase 3: Register submachine B at step 2 ---
        var thinkingSessionIdB = CreateThinkingSession("active");

        WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Starting [[sequential-thinking]] submachine B on step 2.",
            submachineSessionId: thinkingSessionIdB);

        // After registering B at step 2: phase=waiting, step not advanced (still at step 2 index)
        var (fmAfterRegB, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(fmAfterRegB!["phase"]?.ToString(), Is.EqualTo("waiting"), "Phase should be 'waiting' after registering B");
        Assert.That(fmAfterRegB!["activeSubmachineSessionId"]?.ToString(), Is.EqualTo(thinkingSessionIdB), "activeSubmachineSessionId should be B");
        var stepAfterRegB = Convert.ToInt32(fmAfterRegB!["currentStep"], CultureInfo.InvariantCulture);
        Assert.That(stepAfterRegB, Is.EqualTo(stepAfterResumeA), "Step must NOT advance while registering submachine B");

        // --- Phase 4: Complete B and resume ---
        UpdateThinkingSessionStatus(thinkingSessionIdB, "completed");

        var resumeResultB = WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Resuming [[hierarchical-state-machines]] after submachine B completed — advancing step.");

        // After completing B and resuming: phase=running, step advances to 3/3
        Assert.That(resumeResultB, Does.Not.Contain("waiting on submachine"), "Should not be blocked after B completes");
        Assert.That(resumeResultB, Does.Contain("Step 3/"), "Should advance to step 3 after B completes");

        var (fmAfterResumeB, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(fmAfterResumeB!["phase"]?.ToString(), Is.EqualTo("running"), "Phase should be 'running' after B completes");
        Assert.That(fmAfterResumeB!["activeSubmachineSessionId"]?.ToString(), Is.EqualTo(""), "activeSubmachineSessionId should be cleared after B");
        Assert.That(fmAfterResumeB!["activeSubmachineType"]?.ToString(), Is.EqualTo(""), "activeSubmachineType should be cleared after B");
    }

    // ============ FR-10.4: Double registration while waiting is blocked ============

    /// <summary>
    /// Test 15: Attempting to register a second submachine while already waiting SHALL be blocked
    /// by the gate check, preserving the original submachine registration (FR-10.4).
    /// </summary>
    [Test]
    public void DoubleRegistrationWhileWaitingIsBlocked()
    {
        // T-HSM-001.3: RTM FR-10.4
        var workflowResult = WorkflowTools.Workflow(workflowId: "wsup-alpha");
        var workflowSessionId = ExtractAndRegisterWorkflowSessionId(workflowResult);

        // Register the first submachine
        var firstSessionId = CreateThinkingSession("active");

        WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Registering first [[hierarchical-state-machines]] submachine.",
            submachineSessionId: firstSessionId);

        // After first registration: phase=waiting, activeSubmachineSessionId = first session
        var (fmAfterFirst, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(fmAfterFirst!["phase"]?.ToString(), Is.EqualTo("waiting"), "Phase should be 'waiting' after first registration");
        Assert.That(fmAfterFirst!["activeSubmachineSessionId"]?.ToString(), Is.EqualTo(firstSessionId), "activeSubmachineSessionId should be the first session");

        // Attempt to register a second submachine while still waiting on the first
        var secondSessionId = CreateThinkingSession("active");

        var secondRegResult = WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Attempting to register second [[sequential-thinking]] submachine while waiting.",
            submachineSessionId: secondSessionId);

        // After second registration attempt: response contains "waiting on submachine"
        Assert.That(secondRegResult, Does.Contain("waiting on submachine"), "Second registration should be blocked by gate check");

        // Phase still waiting, activeSubmachineSessionId still = first session (not overwritten)
        var (fmAfterSecond, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(fmAfterSecond!["phase"]?.ToString(), Is.EqualTo("waiting"), "Phase should still be 'waiting'");
        Assert.That(fmAfterSecond!["activeSubmachineSessionId"]?.ToString(), Is.EqualTo(firstSessionId),
            "activeSubmachineSessionId must still be the first session — second registration must NOT overwrite it");
    }

    // ============ FR-10.4: Status parameter while waiting is blocked ============

    /// <summary>
    /// Test 16: Attempting to complete or cancel a workflow while waiting on an active submachine
    /// SHALL be blocked by the gate check (FR-10.4).
    /// </summary>
    [Test]
    public void StatusParameterWhileWaitingIsBlocked()
    {
        // T-HSM-001.3: RTM FR-10.4
        var workflowResult = WorkflowTools.Workflow(workflowId: "wsup-alpha");
        var workflowSessionId = ExtractAndRegisterWorkflowSessionId(workflowResult);

        var thinkingSessionId = CreateThinkingSession("active");

        // Register the submachine to enter waiting state
        WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Registering [[hierarchical-state-machines]] submachine before attempting completion.",
            submachineSessionId: thinkingSessionId);

        // After registration: phase=waiting
        var (fmAfterReg, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        Assert.That(fmAfterReg!["phase"]?.ToString(), Is.EqualTo("waiting"), "Phase should be 'waiting' after registration");

        // Attempt to complete the workflow while waiting on active submachine
        var completeResult = WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Trying to complete [[workflow]] while submachine is active.",
            status: "completed",
            conclusion: "Attempting completion with [[hierarchical-state-machines]] conclusion.");

        Assert.That(completeResult, Does.Contain("waiting on submachine"),
            "Completing while waiting should be blocked by gate check");

        // Attempt to cancel the workflow while waiting on active submachine
        var cancelResult = WorkflowTools.Workflow(
            sessionId: workflowSessionId,
            response: "Trying to cancel [[workflow]] while submachine is active.",
            status: "cancelled");

        Assert.That(cancelResult, Does.Contain("waiting on submachine"),
            "Cancelling while waiting should be blocked by gate check");

        // Workflow status in frontmatter is still "active" (not "completed" or "cancelled")
        var (fmAfterAttempts, _, _) = MarkdownIO.ReadSession("workflow", workflowSessionId);
        var workflowStatus = fmAfterAttempts!["status"]?.ToString();
        Assert.That(workflowStatus, Is.EqualTo("active"),
            "Workflow status must remain 'active' — blocked attempts must not alter it to 'completed' or 'cancelled'");

        // phase is still "waiting"
        Assert.That(fmAfterAttempts!["phase"]?.ToString(), Is.EqualTo("waiting"),
            "Phase must remain 'waiting' after blocked completion/cancellation attempts");
    }
}
