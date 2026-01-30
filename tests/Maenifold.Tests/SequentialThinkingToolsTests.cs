using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Baseline tests for SequentialThinkingTools capturing current behavior BEFORE refactoring.
/// Uses real MarkdownIO and file I/O (NO MOCKS) per [[testing-philosophy]].
/// </summary>
public class SequentialThinkingToolsTests
{
    private string _testMemoryPath = null!;
    private string _testDatabasePath = null!;

    [SetUp]
    public void SetUp()
    {
        _testMemoryPath = Config.TestMemoryPath;
        _testDatabasePath = Config.TestDatabasePath;

        // Ensure test directory exists
        Directory.CreateDirectory(_testMemoryPath);
        Directory.CreateDirectory(Path.Combine(_testMemoryPath, "thinking", "sequential"));
        Directory.CreateDirectory(Path.Combine(_testMemoryPath, "thinking", "workflow"));

        // Override Config paths for tests using environment variables
        Environment.SetEnvironmentVariable("MAENIFOLD_ROOT", Path.GetDirectoryName(_testMemoryPath));
    }

    [TearDown]
    public void TearDown()
    {
        // Clean up test directory
        var testOutputsDir = Path.GetDirectoryName(_testMemoryPath);
        if (testOutputsDir != null && Directory.Exists(testOutputsDir))
        {
            Directory.Delete(testOutputsDir, true);
        }

        // Restore environment
        Environment.SetEnvironmentVariable("MAENIFOLD_ROOT", null);
    }

    /// <summary>
    /// Test 1: New Session Creation with Valid Input
    /// Verifies that creating a new session with concepts present generates proper session file.
    /// </summary>
    [Test]
    public void NewSessionWithConceptsCreatesSessionFile()
    {
        // Arrange
        var response = "This is a test thought with [[sequential-thinking]] and [[session-management]] tags.";

        // Act
        var result = SequentialThinkingTools.SequentialThinking(
            response: response,
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 3
        );

        // Assert - verify success response
        Assert.That(result, Does.Contain("Created session:"));
        Assert.That(result, Does.Contain("Continue with thought 1/3"));

        // Verify session file was created with real file I/O
        var sessionId = ExtractSessionId(result);

        Assert.That(MarkdownIO.SessionExists("sequential", sessionId), Is.True,
            "Session file should exist in real file system");

        // Verify session content structure
        var (frontmatter, content, _) = MarkdownIO.ReadSession("sequential", sessionId);
        Assert.That(frontmatter, Is.Not.Null, "Frontmatter should exist");
        Assert.That(frontmatter?["title"], Does.Contain("Sequential Thinking Session"));
        Assert.That(frontmatter?["type"], Is.EqualTo("sequential"));
        Assert.That(frontmatter?["status"], Is.EqualTo("active"));
        Assert.That(frontmatter?["permalink"], Is.EqualTo(sessionId));

        // Verify content contains thought heading
        Assert.That(content, Does.Contain("Thought 0/3"));
        Assert.That(content, Does.Contain("test thought with [[sequential-thinking]]"));
    }

    /// <summary>
    /// Test 2: Concept Validation - Pass Case
    /// Verifies that response WITH WikiLinks passes validation.
    /// </summary>
    [Test]
    public void NewSessionWithConceptsInResponsePassesValidation()
    {
        // Arrange
        var response = "Analyzing [[machine-learning]] algorithms with [[neural-networks]].";

        // Act
        var result = SequentialThinkingTools.SequentialThinking(
            response: response,
            nextThoughtNeeded: false,
            thoughtNumber: 0,
            totalThoughts: 1,
            conclusion: "Concluded that [[deep-learning]] is important."
        );

        // Assert - should succeed with completion message
        Assert.That(result, Does.Contain("Created session:"));
        Assert.That(result, Does.Contain("Thinking complete"));
        Assert.That(result, Does.Not.StartWith("ERROR:"));
    }

    /// <summary>
    /// Test 3: Concept Validation - Fail Case
    /// Verifies that response WITHOUT WikiLinks fails validation.
    /// </summary>
    [Test]
    public void NewSessionWithoutConceptsFailsValidation()
    {
        // Arrange
        var response = "This thought has no concepts at all.";

        // Act
        var result = SequentialThinkingTools.SequentialThinking(
            response: response,
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2
        );

        // Assert
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("Must include [[WikiLinks]]"));
    }

    /// <summary>
    /// Test 4: Session Continuation
    /// Verifies that continuing an existing session appends thought and preserves structure.
    /// </summary>
    [Test]
    public void ContinueSessionWithValidSessionIdAppendsTought()
    {
        // Arrange - create initial session
        var firstResponse = "First thought with [[data-analysis]].";
        var createResult = SequentialThinkingTools.SequentialThinking(
            response: firstResponse,
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2
        );

        var sessionId = ExtractSessionId(createResult);

        // Act - continue with second thought
        var secondResponse = "Second thought with [[pattern-recognition]].";
        var continueResult = SequentialThinkingTools.SequentialThinking(
            response: secondResponse,
            nextThoughtNeeded: false,
            thoughtNumber: 1,
            totalThoughts: 2,
            sessionId: sessionId,
            conclusion: "Final synthesis with [[concept-synthesis]]."
        );

        // Assert
        Assert.That(continueResult, Does.Contain("Added thought 1 to session:"));
        Assert.That(continueResult, Does.Contain("Thinking complete"));

        // Verify both thoughts are in session file
        var (frontmatter, content, _) = MarkdownIO.ReadSession("sequential", sessionId);
        Assert.That(content, Does.Contain("Thought 0/2"));
        Assert.That(content, Does.Contain("Thought 1/2"));
        Assert.That(content, Does.Contain("First thought with [[data-analysis]]"));
        Assert.That(content, Does.Contain("Second thought with [[pattern-recognition]]"));
        Assert.That(frontmatter?["status"], Is.EqualTo("completed"));
        Assert.That(frontmatter?["thoughtCount"].ToString(), Is.EqualTo("2"));
    }

    /// <summary>
    /// Test 5: Parent Workflow Linking
    /// Verifies that sessions can link to parent workflow and bidirectional relationship is created.
    /// </summary>
    [Test]
    public void NewSessionWithParentWorkflowIdCreatesBidirectionalLinks()
    {
        // Arrange - create parent workflow session first
        var parentSessionId = $"workflow-{DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()}";
        var parentFrontmatter = new Dictionary<string, object>
        {
            ["title"] = $"Workflow Session {parentSessionId}",
            ["permalink"] = parentSessionId,
            ["type"] = "workflow",
            ["status"] = "active"
        };
        MarkdownIO.CreateSession("workflow", parentSessionId, parentFrontmatter, "# Workflow\n\nTest workflow.");

        // Act - create sequential session with parent reference
        var response = "Thinking within workflow with [[analysis]].";
        var result = SequentialThinkingTools.SequentialThinking(
            response: response,
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2,
            parentWorkflowId: parentSessionId
        );

        // Assert - child session created successfully
        Assert.That(result, Does.Contain("Created session:"));
        var sessionId = ExtractSessionId(result);

        // Verify child session has parent reference
        var (childFrontmatter, _, _) = MarkdownIO.ReadSession("sequential", sessionId);
        Assert.That(childFrontmatter?["parent"], Does.Contain(parentSessionId));

        // Verify parent session was updated with child reference
        var (parentMeta, parentContent, _) = MarkdownIO.ReadSession("workflow", parentSessionId);
        Assert.That(parentMeta, Is.Not.Null);
        Assert.That(parentMeta!.ContainsKey("related"), Is.True, "Parent should have related field");
        var relatedList = parentMeta!["related"] as List<object>;
        Assert.That(relatedList, Is.Not.Null);
        Assert.That(relatedList!.OfType<string>().Any(r => r.Contains(sessionId)), Is.True,
            "Parent should reference child session");
    }

    /// <summary>
    /// Test 6: Session Cancellation
    /// Verifies that cancel=true updates session status to cancelled.
    /// </summary>
    [Test]
    public void SessionWithCancelUpdatesStatusToCancelled()
    {
        // Arrange - create session
        var response = "Initial thought with [[session-lifecycle]].";
        var createResult = SequentialThinkingTools.SequentialThinking(
            response: response,
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 5
        );

        var sessionId = ExtractSessionId(createResult);

        // Act - cancel session
        var cancelResult = SequentialThinkingTools.SequentialThinking(
            response: "Cancelling thoughts with [[workflow-cancellation]].",
            nextThoughtNeeded: false,
            thoughtNumber: 1,
            totalThoughts: 5,
            sessionId: sessionId,
            cancel: true
        );

        // Assert
        Assert.That(cancelResult, Does.Contain("Thinking cancelled"));
        Assert.That(cancelResult, Does.Contain("Added thought 1 to session:"));

        // Verify session status is cancelled
        var (frontmatter, _, _) = MarkdownIO.ReadSession("sequential", sessionId);
        Assert.That(frontmatter?["status"], Is.EqualTo("cancelled"));
        Assert.That(frontmatter?.ContainsKey("cancelled"), Is.True);
        Assert.That(frontmatter?["thoughtCount"].ToString(), Is.EqualTo("2"));
    }

    /// <summary>
    /// Test 6b: Session Cancellation Without Response (NEW - Tests the fix for optional response parameter)
    /// Verifies that cancel=true works without requiring response parameter.
    /// This is the key test for the fix - cancel should only require sessionId and cancel=true.
    /// </summary>
    [Test]
    public void SessionWithCancelWithoutResponseUpdatesStatusToCancelled()
    {
        // Arrange - create session
        var response = "Initial thought with [[cancel-workflow]].";
        var createResult = SequentialThinkingTools.SequentialThinking(
            response: response,
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 5
        );

        var sessionId = ExtractSessionId(createResult);

        // Act - cancel session WITHOUT providing response parameter (key test!)
        var cancelResult = SequentialThinkingTools.SequentialThinking(
            sessionId: sessionId,
            cancel: true
        );

        // Assert
        Assert.That(cancelResult, Does.Contain("Thinking cancelled"));
        Assert.That(cancelResult, Does.Not.StartWith("ERROR:"));

        // Verify session status is cancelled
        var (frontmatter, _, _) = MarkdownIO.ReadSession("sequential", sessionId);
        Assert.That(frontmatter?["status"], Is.EqualTo("cancelled"));
        Assert.That(frontmatter?.ContainsKey("cancelled"), Is.True);
        // When cancelling without thoughtNumber, thoughtCount is preserved (or 0 if not set)
        Assert.That(frontmatter?.ContainsKey("thoughtCount"), Is.True);
    }

    /// <summary>
    /// Test 7: Session Completion with Conclusion
    /// Verifies that session completes with conclusion and status is marked completed.
    /// </summary>
    [Test]
    public void SessionWithConclusionCompletesSession()
    {
        // Arrange
        var createResult = SequentialThinkingTools.SequentialThinking(
            response: "Thought 1 with [[requirements-analysis]].",
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2
        );

        var sessionId = ExtractSessionId(createResult);

        // Act - complete with conclusion
        var completeResult = SequentialThinkingTools.SequentialThinking(
            response: "Thought 2 with [[solution-design]].",
            nextThoughtNeeded: false,
            thoughtNumber: 1,
            totalThoughts: 2,
            sessionId: sessionId,
            conclusion: "This analysis demonstrates [[synthesis]] of concepts."
        );

        // Assert
        Assert.That(completeResult, Does.Contain("Thinking complete"));

        // Verify session is completed
        var (frontmatter, content, _) = MarkdownIO.ReadSession("sequential", sessionId);
        Assert.That(frontmatter?["status"], Is.EqualTo("completed"));
        Assert.That(frontmatter?.ContainsKey("completed"), Is.True);
        Assert.That(frontmatter?["thoughtCount"].ToString(), Is.EqualTo("2"));
        Assert.That(content, Does.Contain("Conclusion"));
        Assert.That(content, Does.Contain("[[synthesis]]"));
    }

    /// <summary>
    /// Test 8: Revision Mode
    /// Verifies that isRevision=true with revisesThought creates revision heading.
    /// </summary>
    [Test]
    public void SessionWithRevisionCreatesRevisionHeading()
    {
        // Arrange - create initial session
        var createResult = SequentialThinkingTools.SequentialThinking(
            response: "Original thought with [[initial-idea]].",
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2
        );

        var sessionId = ExtractSessionId(createResult);

        // Act - create revision
        var revisionResult = SequentialThinkingTools.SequentialThinking(
            response: "Revised thought with [[refined-approach]].",
            nextThoughtNeeded: true,
            thoughtNumber: 1,
            totalThoughts: 2,
            sessionId: sessionId,
            isRevision: true,
            revisesThought: 0
        );

        // Assert
        Assert.That(revisionResult, Does.Contain("Added thought 1 to session:"));

        // Verify revision heading in content
        var (_, content, _) = MarkdownIO.ReadSession("sequential", sessionId);
        Assert.That(content, Does.Contain("(Revises: 0)"));
    }

    /// <summary>
    /// Test 9: Branch Creation
    /// Verifies that branchFromThought with branchId creates branch correctly.
    /// </summary>
    [Test]
    public void SessionWithBranchCreatesBranchHeading()
    {
        // Arrange - create session with multiple thoughts
        var createResult = SequentialThinkingTools.SequentialThinking(
            response: "Main branch thought with [[branching-strategy]].",
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2
        );

        var sessionId = ExtractSessionId(createResult);

        // Continue main branch
        SequentialThinkingTools.SequentialThinking(
            response: "Main branch thought 2 with [[revision-control]].",
            nextThoughtNeeded: false,
            thoughtNumber: 1,
            totalThoughts: 2,
            sessionId: sessionId,
            conclusion: "Main branch conclusion with [[thought-divergence]]."
        );

        // Act - create branch from thought 0
        var branchResult = SequentialThinkingTools.SequentialThinking(
            response: "Branched thought with [[alternative-concept]].",
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2,
            sessionId: sessionId,
            branchFromThought: 0,
            branchId: "alternative-analysis"
        );

        // Assert
        Assert.That(branchResult, Does.Contain("Created session:"));

        // Verify branch heading in content
        var (_, content, _) = MarkdownIO.ReadSession("sequential", sessionId);
        Assert.That(content, Does.Contain("(Branch: alternative-analysis)"));
    }

    /// <summary>
    /// Test 10: Needs More Thoughts
    /// Verifies that needsMoreThoughts=true expands totalThoughts estimate.
    /// </summary>
    [Test]
    public void SessionWithNeedsMoreThoughtsExpandsTotalThoughts()
    {
        // Arrange
        var createResult = SequentialThinkingTools.SequentialThinking(
            response: "First thought with [[api-design]].",
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2
        );

        var sessionId = ExtractSessionId(createResult);

        // Act - continue and expand thoughts
        var expandResult = SequentialThinkingTools.SequentialThinking(
            response: "Second thought with [[implementation-details]].",
            nextThoughtNeeded: true,
            thoughtNumber: 1,
            totalThoughts: 2,
            sessionId: sessionId,
            needsMoreThoughts: true
        );

        // Assert
        Assert.That(expandResult, Does.Contain("Continue with thought 2/?"));
        Assert.That(expandResult, Does.Contain("extending beyond initial estimate"));
    }

    /// <summary>
    /// Test 11: Error Handling - Invalid Parent Workflow
    /// Verifies that nonexistent parent workflow ID is rejected.
    /// </summary>
    [Test]
    public void NewSessionWithInvalidParentWorkflowReturnsError()
    {
        // Arrange
        var invalidParentId = "nonexistent-workflow-12345";

        // Act
        var result = SequentialThinkingTools.SequentialThinking(
            response: "Thinking with [[workflow-validation]].",
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2,
            parentWorkflowId: invalidParentId
        );

        // Assert
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("not found"));
    }

    /// <summary>
    /// Test 12: Error Handling - Parent Workflow on Non-First Thought
    /// Verifies that parentWorkflowId on thought > 1 is rejected.
    /// </summary>
    [Test]
    public void ContinueSessionWithParentWorkflowOnThought2ReturnsError()
    {
        // Arrange - create initial session
        var createResult = SequentialThinkingTools.SequentialThinking(
            response: "First thought with [[parent-workflow-linking]].",
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2
        );

        var sessionId = ExtractSessionId(createResult);

        // Create parent workflow
        var parentSessionId = $"workflow-{DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()}";
        var parentFrontmatter = new Dictionary<string, object>
        {
            ["title"] = $"Workflow Session {parentSessionId}",
            ["permalink"] = parentSessionId,
            ["type"] = "workflow",
            ["status"] = "active"
        };
        MarkdownIO.CreateSession("workflow", parentSessionId, parentFrontmatter, "# Workflow\n\nTest.");

        // Act - try to set parent on thought 1
        var result = SequentialThinkingTools.SequentialThinking(
            response: "Second thought with [[error-handling]].",
            nextThoughtNeeded: true,
            thoughtNumber: 1,
            totalThoughts: 2,
            sessionId: sessionId,
            parentWorkflowId: parentSessionId
        );

        // Assert
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("Parent workflow can only be set on first thought"));
    }

    /// <summary>
    /// Test 13: Full Session Lifecycle
    /// Integration test: create -> continue -> complete with all features working together.
    /// </summary>
    [Test]
    public void CompleteSessionLifecycleCreateContinueCompleteSucceeds()
    {
        // Arrange & Act Step 1: Create session
        var createResult = SequentialThinkingTools.SequentialThinking(
            response: "Analyzing problem space with [[architecture]] and [[design-patterns]].",
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 3,
            analysisType: "architecture"
        );

        Assert.That(createResult, Does.Contain("Created session:"));
        var sessionId = ExtractSessionId(createResult);

        // Act Step 2: Continue with thought 1
        var continueResult2 = SequentialThinkingTools.SequentialThinking(
            response: "Implementing solution with [[implementation]] details and [[testing]].",
            nextThoughtNeeded: true,
            thoughtNumber: 1,
            totalThoughts: 3,
            sessionId: sessionId,
            thoughts: "Considering [[performance]] implications."
        );

        Assert.That(continueResult2, Does.Contain("Added thought 1 to session:"));

        // Act Step 3: Continue with thought 2 and complete
        var completeResult = SequentialThinkingTools.SequentialThinking(
            response: "Reviewing and refining with [[quality-assurance]].",
            nextThoughtNeeded: false,
            thoughtNumber: 2,
            totalThoughts: 3,
            sessionId: sessionId,
            conclusion: "Concluded that [[modular-design]] with [[robust-testing]] is the best approach."
        );

        Assert.That(completeResult, Does.Contain("Thinking complete"));

        // Assert - verify complete session structure
        var (frontmatter, content, _) = MarkdownIO.ReadSession("sequential", sessionId);

        // Verify frontmatter
        Assert.That(frontmatter?["status"], Is.EqualTo("completed"));
        Assert.That(frontmatter?["type"], Is.EqualTo("sequential"));
        Assert.That(frontmatter?["analysisType"], Is.EqualTo("architecture"));
        Assert.That(frontmatter?.ContainsKey("completed"), Is.True);
        Assert.That(frontmatter?["thoughtCount"].ToString(), Is.EqualTo("3"));

        // Verify content contains all thoughts
        Assert.That(content, Does.Contain("Thought 0/3"));
        Assert.That(content, Does.Contain("Thought 1/3"));
        Assert.That(content, Does.Contain("Thought 2/3"));
        Assert.That(content, Does.Contain("Analyzing problem space"));
        Assert.That(content, Does.Contain("Implementing solution"));
        Assert.That(content, Does.Contain("Reviewing and refining"));
        Assert.That(content, Does.Contain("Conclusion"));
        Assert.That(content, Does.Contain("[[modular-design]]"));
    }

    /// <summary>
    /// Test 14: Concept Extraction in Response
    /// Verifies that concepts are properly extracted from response and thoughts.
    /// </summary>
    [Test]
    public void NewSessionExtractsConceptsCorrectly()
    {
        // Arrange
        var response = "Exploring [[machine-learning]], [[neural-networks]], and [[deep-learning]].";
        var thoughts = "Related to [[AI]], [[algorithms]], and [[data-science]].";

        // Act
        var result = SequentialThinkingTools.SequentialThinking(
            response: response,
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 1,
            thoughts: thoughts
        );

        // Assert
        Assert.That(result, Does.Contain("Created session:"));
        var sessionId = ExtractSessionId(result);

        // Verify session was created (concepts validation passed)
        Assert.That(MarkdownIO.SessionExists("sequential", sessionId), Is.True);

        // Verify content includes both response and thoughts
        var (_, content, _) = MarkdownIO.ReadSession("sequential", sessionId);
        Assert.That(content, Does.Contain("machine-learning"));
        Assert.That(content, Does.Contain("Thoughts:"));
        Assert.That(content, Does.Contain("AI"));
    }

    /// <summary>
    /// Test 15: Session Continuation Validation
    /// Verifies error handling when continuing nonexistent session.
    /// </summary>
    [Test]
    public void ContinueSessionWithNonexistentSessionIdReturnsError()
    {
        // Arrange
        var nonexistentSessionId = "session-9999999999999";

        // Act
        var result = SequentialThinkingTools.SequentialThinking(
            response: "Thought with [[validation-testing]].",
            nextThoughtNeeded: true,
            thoughtNumber: 1,
            totalThoughts: 3,
            sessionId: nonexistentSessionId
        );

        // Assert
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("missing"));
    }

    /// <summary>
    /// Helper method to extract session ID from response message.
    /// </summary>
    /// <summary>
    /// Test 19: Branch ID Validation for Multi-Agent Safety
    /// Verifies that branchFromThought without branchId is rejected to prevent multi-agent conflicts.
    /// </summary>
    [Test]
    public void SessionWithBranchFromThoughtWithoutBranchIdReturnsError()
    {
        // Arrange - create initial session
        var createResult = SequentialThinkingTools.SequentialThinking(
            response: "Main thought with [[multi-agent-coordination]].",
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2
        );

        var sessionId = ExtractSessionId(createResult);

        // Act - attempt to branch WITHOUT branchId (should fail)
        var result = SequentialThinkingTools.SequentialThinking(
            response: "Branched thought with [[alternative-approach]].",
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2,
            sessionId: sessionId,
            branchFromThought: 0,
            branchId: null  // Missing branchId for multi-agent coordination
        );

        // Assert - should return error about missing branchId
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("branchId required"));
        Assert.That(result, Does.Contain("multi-agent coordination"));
    }

    private static string ExtractSessionId(string responseMessage)
    {
        // Format: "Created session: session-1234567890" or "Added thought X to session: session-1234567890"
        var lines = responseMessage.Split('\n');
        foreach (var line in lines)
        {
            if (line.Contains("Created session:") || line.Contains("to session:"))
            {
                var parts = line.Split(':');
                if (parts.Length > 1)
                {
                    return parts[^1].Trim();
                }
            }
        }
        throw new InvalidOperationException($"Could not extract session ID from: {responseMessage}");
    }
}
