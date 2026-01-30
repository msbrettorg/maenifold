using Maenifold.Utils;
using NUnit.Framework;
using Markdig.Syntax;

namespace Maenifold.Tests;

public class MarkdownReaderTests
{
    [Test]
    public void ExtractWikiLinks_SkipsCodeBlocks()
    {
        var content = @"Real [[authentication]] here.

```bash
# This [[fake-credential]] and $variable should NOT be extracted
echo ""test""
```

Another [[database-connection]] outside.";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("authentication"));
        Assert.That(concepts, Does.Contain("database-connection"));
        Assert.That(concepts, Does.Not.Contain("fake-credential"));
        Assert.That(concepts.Count, Is.EqualTo(2));
    }

    [Test]
    public void ExtractWikiLinks_SkipsInlineCode()
    {
        var content = @"This is a [[machine-learning]] and `[[inline-code]]` in inline code.";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("machine-learning"));
        Assert.That(concepts, Does.Not.Contain("inline-code"));
        Assert.That(concepts.Count, Is.EqualTo(1));
    }

    [Test]
    public void ExtractWikiLinks_ExtractsFromHeadings()
    {
        var content = @"# Heading with [[neural-network]]

Body with [[deep-learning]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("neural-network"));
        Assert.That(concepts, Does.Contain("deep-learning"));
        Assert.That(concepts.Count, Is.EqualTo(2));
    }

    [Test]
    public void ExtractWikiLinks_ExtractsFromLists()
    {
        var content = @"- Item with [[api-endpoint]]
- Another item with [[database-schema]]

Text with [[cache-strategy]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("api-endpoint"));
        Assert.That(concepts, Does.Contain("database-schema"));
        Assert.That(concepts, Does.Contain("cache-strategy"));
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
        var content = @"Valid [[dependency-injection]] here.

```csharp
var x = ""[[code-snippet]]"";
```

Another [[unit-testing]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("dependency-injection"));
        Assert.That(concepts, Does.Contain("unit-testing"));
        Assert.That(concepts, Does.Not.Contain("code-snippet"));
        Assert.That(concepts.Count, Is.EqualTo(2));
    }

    [Test]
    public void ExtractWikiLinks_SkipsIndentedCodeBlocks()
    {
        var content = @"Normal [[refactoring]].

    This is indented code block
    [[indented-code]]

Back to normal [[code-review]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("refactoring"));
        Assert.That(concepts, Does.Contain("code-review"));
        Assert.That(concepts, Does.Not.Contain("indented-code"));
        Assert.That(concepts.Count, Is.EqualTo(2));
    }

    [Test]
    public void CountConceptOccurrences_SkipsCodeBlocks()
    {
        var content = @"First [[logging]].

```bash
[[logging]] in code
```

Second [[logging]].";

        var count = MarkdownReader.CountConceptOccurrences(content, "logging");

        Assert.That(count, Is.EqualTo(2));
    }

    [Test]
    public void CountConceptOccurrences_SkipsInlineCode()
    {
        var content = @"First [[microservices]] and `[[microservices]]` in code and third [[microservices]].";

        var count = MarkdownReader.CountConceptOccurrences(content, "microservices");

        Assert.That(count, Is.EqualTo(2));
    }

    [Test]
    public void CountConceptOccurrences_HandlesNormalization()
    {
        var content = @"[[API Design]] and [[api-design]] are the same.";

        var count = MarkdownReader.CountConceptOccurrences(content, "api-design");

        Assert.That(count, Is.EqualTo(2));
    }
}
