using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Tests for WikiLink regex pattern to ensure malformed patterns are rejected.
/// Fixes bug where [[[triple-attack]]] incorrectly extracted "[triple-attack".
/// </summary>
public class WikiLinkRegexTests
{
    [Test]
    public void ExtractWikiLinks_RejectsTripleBracketPattern()
    {
        // Arrange: Content with malformed [[[triple-attack]]]
        var content = "This has a malformed [[[triple-attack]]] pattern.";

        // Act: Extract WikiLinks
        var concepts = MarkdownReader.ExtractWikiLinks(content);

        // Assert: Should NOT extract any concept (triple brackets are invalid)
        Assert.That(concepts, Is.Empty, "Triple bracket pattern should not extract any concepts");
    }

    [Test]
    public void ExtractWikiLinks_RejectsQuadrupleBracketPattern()
    {
        // Arrange: Content with malformed [[[[quadruple]]]] pattern
        var content = "This has [[[[quadruple]]]] brackets.";

        // Act: Extract WikiLinks
        var concepts = MarkdownReader.ExtractWikiLinks(content);

        // Assert: Should NOT extract concepts with brackets in them
        Assert.That(concepts, Is.Empty.Or.All.Not.Contains("["));
        Assert.That(concepts, Is.Empty.Or.All.Not.Contains("]"));
    }

    [Test]
    public void ExtractWikiLinks_ExtractsOnlyValidConcepts()
    {
        // Arrange: Mixed valid and invalid patterns
        var content = @"
Valid: [[neural-network]] and [[machine-learning]]
Malformed: [[[invalid]]] and [[[[also-invalid]]]]
More valid: [[data-science]]
";

        // Act: Extract WikiLinks
        var concepts = MarkdownReader.ExtractWikiLinks(content);

        // Assert: Should extract only the valid concepts
        Assert.That(concepts, Does.Contain("neural-network"));
        Assert.That(concepts, Does.Contain("machine-learning"));
        Assert.That(concepts, Does.Contain("data-science"));

        // Should NOT contain any brackets in the extracted concepts
        foreach (var concept in concepts)
        {
            Assert.That(concept, Does.Not.Contain("["), $"Concept '{concept}' should not contain '['");
            Assert.That(concept, Does.Not.Contain("]"), $"Concept '{concept}' should not contain ']'");
        }
    }

    [Test]
    public void ExtractWikiLinks_HandlesNestedBracketsCorrectly()
    {
        // Arrange: Content with nested brackets in various forms
        var content = @"
Normal: [[algorithm]]
Triple: [[[nested]]]
Prefix: text [[[before]]] after
Suffix: before [[[[after]]]] text
";

        // Act: Extract WikiLinks
        var concepts = MarkdownReader.ExtractWikiLinks(content);

        // Assert: Should only extract "algorithm" (the valid one)
        Assert.That(concepts, Does.Contain("algorithm"));

        // Verify no concept contains brackets
        foreach (var concept in concepts)
        {
            Assert.That(concept, Does.Not.Contain("["));
            Assert.That(concept, Does.Not.Contain("]"));
        }
    }

    [Test]
    public void CountConceptOccurrences_IgnoresMalformedBrackets()
    {
        // Arrange: Content with both valid and malformed references
        var content = @"
Valid: [[regex-pattern]]
Invalid: [[[regex-pattern]]]
Valid again: [[regex-pattern]]
";

        // Act: Count occurrences of "regex-pattern"
        var count = MarkdownReader.CountConceptOccurrences(content, "regex-pattern");

        // Assert: Should count only the two valid occurrences
        Assert.That(count, Is.EqualTo(2), "Should count only valid [[WikiLink]] patterns");
    }

    [Test]
    public void ExtractWikiLinks_AllowsHyphensUnderscoresInConcepts()
    {
        // Arrange: Valid concepts with allowed characters
        var content = @"
Hyphenated: [[machine-learning]]
Underscored: [[neural_network]]
Mixed: [[deep-learning_model]]
Numbers: [[gpt-4]]
";

        // Act: Extract WikiLinks
        var concepts = MarkdownReader.ExtractWikiLinks(content);

        // Assert: All valid patterns should be extracted (normalized: underscores become hyphens)
        Assert.That(concepts, Does.Contain("machine-learning"));
        Assert.That(concepts, Does.Contain("neural-network")); // normalized from neural_network
        Assert.That(concepts, Does.Contain("deep-learning-model")); // normalized from deep-learning_model
        Assert.That(concepts, Does.Contain("gpt-4"));
    }
}
