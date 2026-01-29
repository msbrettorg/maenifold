using Maenifold.Tools;
using NUnit.Framework;
using System;

namespace Maenifold.Tests;

public class ConceptGraphTests
{
    private static readonly string[] TestTags = new[] { "ai", "ml", "test" };

    [SetUp]
    public void SetUp()
    {
        // Ensure database is initialized before any Sync operations
        GraphDatabase.InitializeDatabase();
    }

    [Test]
    public void ConceptGraphSmokeTest()
    {
        Console.WriteLine("Testing Ma Core Concept Graph\n");

        // Test WriteMemory
        Console.WriteLine("1. Writing test memory...");
        var result = MemoryTools.WriteMemory(
            "AI Concepts Test",
            "This document explores [[Artificial Intelligence]] and its relationship to [[Machine Learning]] and [[Deep Learning]].\n\n" +
            "[[Deep Learning]] is a subset of [[Machine Learning]], which itself is a subset of [[Artificial Intelligence]].",
            folder: "test",
            tags: TestTags
        );
        Console.WriteLine(result);

        // Test Sync
        Console.WriteLine("\n2. Syncing concepts to graph database...");
        var syncResult = GraphTools.Sync();
        Console.WriteLine(syncResult);

        // Test BuildContext
        Console.WriteLine("\n3. Building context for 'Machine Learning'...");
        var contextResult = GraphTools.BuildContext("Machine Learning", 2);
        Console.WriteLine(contextResult);

        Console.WriteLine("\nTest complete!");
    }

    [Test]
    public void BuildContextWithoutContentTest()
    {
        Console.WriteLine("\nTesting BuildContext with includeContent=false\n");

        // Test WriteMemory
        Console.WriteLine("1. Writing test memory...");
        var result = MemoryTools.WriteMemory(
            "Machine Learning Concepts for Content Test",
            "This document discusses [[Machine Learning]] and its applications in modern systems.",
            folder: "test",
            tags: TestTags
        );
        Console.WriteLine(result);

        // Test Sync
        Console.WriteLine("\n2. Syncing concepts to graph database...");
        var syncResult = GraphTools.Sync();
        Console.WriteLine(syncResult);

        // Test BuildContext WITHOUT content
        Console.WriteLine("\n3. Building context WITHOUT content preview...");
        var contextResult = GraphTools.BuildContext("Machine Learning", depth: 2, maxEntities: 20, includeContent: false);
        Console.WriteLine(contextResult);

        // Verify content is NOT included
        Assert.That(contextResult.ToString(), Does.Not.Contain("Content preview"),
            "Output should not contain content previews when includeContent=false");

        Console.WriteLine("\nTest passed - no content included!");
    }

    [Test]
    public void BuildContextWithContentTest()
    {
        Console.WriteLine("\nTesting BuildContext with includeContent=true\n");

        // Create MULTIPLE files with SHARED concepts to create relationships
        Console.WriteLine("1. Writing test memories with shared concepts...");

        // File 1: Deep Learning + Machine Learning + Neural Networks
        var result1 = MemoryTools.WriteMemory(
            "Deep Learning Guide",
            "Comprehensive guide to [[Deep Learning]] using [[Neural Networks]]. [[Deep Learning]] is part of [[Machine Learning]].",
            folder: "test",
            tags: TestTags
        );

        // File 2: Machine Learning + Deep Learning + AI
        var result2 = MemoryTools.WriteMemory(
            "Machine Learning Overview",
            "Overview of [[Machine Learning]] and [[Deep Learning]] in [[AI]]. [[Machine Learning]] uses [[Neural Networks]].",
            folder: "test",
            tags: TestTags
        );

        // File 3: Neural Networks + Deep Learning
        var result3 = MemoryTools.WriteMemory(
            "Neural Networks Tutorial",
            "Tutorial on [[Neural Networks]] for [[Deep Learning]]. [[Neural Networks]] power modern [[AI]].",
            folder: "test",
            tags: TestTags
        );

        Console.WriteLine(result1);

        // Test Sync to create relationships
        Console.WriteLine("\n2. Syncing concepts to graph database...");
        var syncResult = GraphTools.Sync();
        Console.WriteLine(syncResult);

        // Test BuildContext WITH content
        Console.WriteLine("\n3. Building context WITH content preview...");
        var contextResult = GraphTools.BuildContext("Deep Learning", depth: 2, maxEntities: 20, includeContent: true);
        Console.WriteLine(contextResult);

        // Verify content IS included when relationships exist
        Assert.That(contextResult.DirectRelations.Count, Is.GreaterThan(0),
            "Should have direct relations from shared concepts");
        Assert.That(contextResult.ToString(), Does.Contain("Content preview"),
            "Output should contain content previews when includeContent=true AND relationships exist");

        Console.WriteLine("\nTest passed - content included with proper formatting!");
    }

    [Test]
    public void BuildContextIncludeContentDefaultTest()
    {
        Console.WriteLine("\nTesting BuildContext with default includeContent parameter\n");

        // Test WriteMemory
        Console.WriteLine("1. Writing test memory...");
        var result = MemoryTools.WriteMemory(
            "AI Overview for Default Test",
            "Overview of [[Artificial Intelligence]] in modern systems.",
            folder: "test",
            tags: TestTags
        );
        Console.WriteLine(result);

        // Test Sync
        Console.WriteLine("\n2. Syncing concepts to graph database...");
        var syncResult = GraphTools.Sync();
        Console.WriteLine(syncResult);

        // Test BuildContext with default parameter (should not include content)
        Console.WriteLine("\n3. Building context with default includeContent...");
        var contextResult = GraphTools.BuildContext("Artificial Intelligence");
        Console.WriteLine(contextResult);

        // Default should be false, so no content preview
        Assert.That(contextResult.ToString(), Does.Not.Contain("Content preview"),
            "Default includeContent should be false - no content previews");

        Console.WriteLine("\nTest passed - default behavior maintains backwards compatibility!");
    }

    [Test]
    public void BuildContextExtractsSectionWithConceptTest()
    {
        Console.WriteLine("\nTesting GRAPH-001: BuildContext extracts section containing concept mention\n");

        // Create file with concept in later section
        Console.WriteLine("1. Writing test memory with concept in Security section...");
        var authFile = MemoryTools.WriteMemory(
            "Authentication System Overview",
            "# Authentication System Overview\n\n" +
            "## Introduction\n\n" +
            "This document describes our authentication system architecture.\n\n" +
            "## Security\n\n" +
            "The system uses [[authentication]] via JWT tokens. " +
            "All requests must include valid [[authentication]] headers.\n\n" +
            "## Configuration\n\n" +
            "See config.yml for settings.",
            folder: "test",
            tags: TestTags
        );

        // Create second file that also mentions authentication
        var jwtFile = MemoryTools.WriteMemory(
            "JWT Implementation",
            "# JWT Implementation\n\n" +
            "## Overview\n\n" +
            "JWT tokens are used for [[authentication]]. " +
            "This document covers [[JWT]] implementation details.",
            folder: "test",
            tags: TestTags
        );

        Console.WriteLine(authFile);

        // Sync to create relationships
        Console.WriteLine("\n2. Syncing concepts to graph database...");
        var syncResult = GraphTools.Sync();
        Console.WriteLine(syncResult);

        // Build context for authentication with content
        Console.WriteLine("\n3. Building context for 'authentication' WITH content...");
        var contextResult = GraphTools.BuildContext("authentication", depth: 1, maxEntities: 20, includeContent: true);
        Console.WriteLine(contextResult);

        // Verify we have content previews
        Assert.That(contextResult.DirectRelations.Count, Is.GreaterThan(0),
            "Should have direct relations");

        // Check that the preview contains the Security section, not Introduction
        bool foundSecuritySection = false;
        foreach (var relation in contextResult.DirectRelations)
        {
            foreach (var preview in relation.ContentPreview.Values)
            {
                Console.WriteLine($"\nPreview content:\n{preview}\n");

                // Should contain the Security section content
                if (preview.Contains("## Security", StringComparison.Ordinal) ||
                    preview.Contains("JWT tokens", StringComparison.Ordinal))
                {
                    foundSecuritySection = true;
                }

                // Should NOT start with Introduction if concept is in Security section
                Assert.That(preview, Does.Not.StartWith("## Introduction"),
                    "Preview should not show Introduction section when concept is in Security section");
            }
        }

        Assert.That(foundSecuritySection, Is.True,
            "Preview should contain content from Security section where authentication is mentioned");

        Console.WriteLine("\nTest passed - section extraction working correctly!");
    }

    [Test]
    public void ExtractSectionWithConceptEdgeCasesTest()
    {
        Console.WriteLine("\nTesting ExtractSectionWithConcept edge cases\n");

        // Test 1: Concept in H2 section (should return that section)
        var content1 = "# Title\n## Intro\nSome intro.\n## Security\nUsing [[authentication]] here.\n## End\nThe end.";
        var result1 = GraphTools.ExtractSectionWithConcept(content1, "authentication", maxLength: 500);
        Console.WriteLine($"Test 1 (concept in H2):\n{result1}\n");
        Assert.That(result1, Does.Contain("## Security"), "Should extract Security section");
        Assert.That(result1, Does.Contain("[[authentication]]"), "Should include the concept mention");

        // Test 2: Concept appears multiple times (should use first occurrence)
        var content2 = "# Title\n## Intro\nMentions [[auth]].\n## Details\nMore [[auth]] info.";
        var result2 = GraphTools.ExtractSectionWithConcept(content2, "auth", maxLength: 500);
        Console.WriteLine($"Test 2 (multiple mentions):\n{result2}\n");
        Assert.That(result2, Does.Contain("## Intro"), "Should extract first section with mention");

        // Test 3: Concept only in H1 section (no H2 headers)
        var content3 = "# Title\nThis uses [[jwt]] tokens.\nMore content here.";
        var result3 = GraphTools.ExtractSectionWithConcept(content3, "jwt", maxLength: 500);
        Console.WriteLine($"Test 3 (H1 only):\n{result3}\n");
        Assert.That(result3, Does.Contain("[[jwt]]"), "Should extract H1 section");

        // Test 4: Concept not found (should fallback to CreateSmartPreview)
        var content4 = "# Title\n## Section\nNo concept here.";
        var result4 = GraphTools.ExtractSectionWithConcept(content4, "nonexistent", maxLength: 500);
        Console.WriteLine($"Test 4 (concept not found):\n{result4}\n");
        Assert.That(result4, Does.Contain("Title"), "Should fallback to smart preview from start");

        // Test 5: Plain text match (not WikiLink)
        var content5 = "# Title\n## Usage\nPlain authentication without brackets.";
        var result5 = GraphTools.ExtractSectionWithConcept(content5, "authentication", maxLength: 500);
        Console.WriteLine($"Test 5 (plain text):\n{result5}\n");
        Assert.That(result5, Does.Contain("## Usage"), "Should match plain text");
        Assert.That(result5, Does.Contain("authentication"), "Should include plain text mention");

        // Test 6: Very long section (should truncate)
        var longContent = "# Title\n## Long\nUsing [[test]]. " + new string('x', 1000);
        var result6 = GraphTools.ExtractSectionWithConcept(longContent, "test", maxLength: 100);
        Console.WriteLine($"Test 6 (truncation):\nLength: {result6.Length}\n{result6.Substring(0, Math.Min(100, result6.Length))}\n");
        Assert.That(result6.Length, Is.LessThanOrEqualTo(200), "Should truncate long sections");

        // Test 7: Empty or whitespace content
        var result7 = GraphTools.ExtractSectionWithConcept("", "test", maxLength: 500);
        Assert.That(result7, Is.Empty, "Empty content should return empty string");

        var result8 = GraphTools.ExtractSectionWithConcept("   \n  \n", "test", maxLength: 500);
        Console.WriteLine($"Test 8 (whitespace only):\nLength: {result8.Length}");

        Console.WriteLine("\nAll edge case tests passed!");
    }
}
