using Maenifold.Utils;
using NUnit.Framework;
using Markdig.Syntax;

namespace Maenifold.Tests;

public class MarkdownReaderTests
{
    [Test]
    public void ExtractWikiLinks_SkipsCodeBlocks()
    {
        var content = @"Real [[concept]] here.

```bash
# This [[fake-concept]] and $variable should NOT be extracted
echo ""test""
```

Another [[real-concept]] outside.";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("concept"));
        Assert.That(concepts, Does.Contain("real-concept"));
        Assert.That(concepts, Does.Not.Contain("fake-concept"));
        Assert.That(concepts.Count, Is.EqualTo(2));
    }

    [Test]
    public void ExtractWikiLinks_SkipsInlineCode()
    {
        var content = @"This is a [[real-concept]] and `[[not-a-concept]]` in inline code.";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("real-concept"));
        Assert.That(concepts, Does.Not.Contain("not-a-concept"));
        Assert.That(concepts.Count, Is.EqualTo(1));
    }

    [Test]
    public void ExtractWikiLinks_ExtractsFromHeadings()
    {
        var content = @"# Heading with [[concept-in-heading]]

Body with [[concept-in-body]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("concept-in-heading"));
        Assert.That(concepts, Does.Contain("concept-in-body"));
        Assert.That(concepts.Count, Is.EqualTo(2));
    }

    [Test]
    public void ExtractWikiLinks_ExtractsFromLists()
    {
        var content = @"- Item with [[concept-one]]
- Another item with [[concept-two]]

Text with [[concept-three]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("concept-one"));
        Assert.That(concepts, Does.Contain("concept-two"));
        Assert.That(concepts, Does.Contain("concept-three"));
        Assert.That(concepts.Count, Is.EqualTo(3));
    }

    [Test]
    public void ExtractWikiLinks_NormalizesConcepts()
    {
        var content = @"[[Machine Learning]] and [[machine-learning]] are the same.";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts.Count, Is.EqualTo(1));
        Assert.That(concepts, Does.Contain("machine-learning"));
    }

    [Test]
    public void ExtractWikiLinks_SkipsFencedCodeBlockWithLanguage()
    {
        var content = @"Valid [[concept]] here.

```csharp
var x = ""[[fake-concept]]"";
```

Another [[valid-concept]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("concept"));
        Assert.That(concepts, Does.Contain("valid-concept"));
        Assert.That(concepts, Does.Not.Contain("fake-concept"));
        Assert.That(concepts.Count, Is.EqualTo(2));
    }

    [Test]
    public void ExtractWikiLinks_SkipsIndentedCodeBlocks()
    {
        var content = @"Normal [[concept]].

    This is indented code block
    [[should-not-extract]]

Back to normal [[another-concept]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("concept"));
        Assert.That(concepts, Does.Contain("another-concept"));
        Assert.That(concepts, Does.Not.Contain("should-not-extract"));
        Assert.That(concepts.Count, Is.EqualTo(2));
    }

    [Test]
    public void CountConceptOccurrences_SkipsCodeBlocks()
    {
        var content = @"First [[test-concept]].

```bash
[[test-concept]] in code
```

Second [[test-concept]].";

        var count = MarkdownReader.CountConceptOccurrences(content, "test-concept");

        Assert.That(count, Is.EqualTo(2));
    }

    [Test]
    public void CountConceptOccurrences_SkipsInlineCode()
    {
        var content = @"First [[test-concept]] and `[[test-concept]]` in code and third [[test-concept]].";

        var count = MarkdownReader.CountConceptOccurrences(content, "test-concept");

        Assert.That(count, Is.EqualTo(2));
    }

    [Test]
    public void CountConceptOccurrences_HandlesNormalization()
    {
        var content = @"[[Test Concept]] and [[test-concept]] are the same.";

        var count = MarkdownReader.CountConceptOccurrences(content, "test-concept");

        Assert.That(count, Is.EqualTo(2));
    }
}
