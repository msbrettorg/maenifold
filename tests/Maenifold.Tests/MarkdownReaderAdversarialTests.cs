using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Adversarial security testing for markdown-aware concept extraction.
/// These tests target edge cases, malicious inputs, and potential exploits.
/// </summary>
public class MarkdownReaderAdversarialTests
{
    #region Nested Code Block Attacks

    [Test]
    public void ExtractWikiLinks_NestedCodeBlocks_OuterOnly()
    {
        // CommonMark alignment test: Malformed nested backticks
        // Per CommonMark spec 4.5, fence closes at FIRST matching ``` line
        //
        // Line 4: ``` (opens fence)
        // Line 6: ``` (CLOSES fence - first matching fence)
        // Line 7: "Inner attempt..." is PARAGRAPH text (not in code)
        // Line 8-9: New fence pair
        //
        // Markdig correctly parses line 7 as paragraph → [[should-skip-inner]] WILL be extracted
        var content = @"
Valid [[markdown-parsing]].

```
Outer block with [[should-skip-outer]]
```
Inner attempt with [[should-skip-inner]]
```
```

Valid [[code-fence]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        // Correct expectations per CommonMark spec:
        Assert.That(concepts, Does.Contain("markdown-parsing"));
        Assert.That(concepts, Does.Contain("code-fence"));
        Assert.That(concepts, Does.Not.Contain("should-skip-outer"));
        // FIXED: should-skip-inner is in paragraph text, so it WILL be extracted
        Assert.That(concepts, Does.Contain("should-skip-inner"));
        Assert.That(concepts.Count, Is.EqualTo(3));
    }

    [Test]
    public void ExtractWikiLinks_TripleBacktickInsideTripleBacktick()
    {
        // CommonMark alignment test: Triple backticks inside code fence
        // Per CommonMark spec 4.5, fence closes at FIRST matching ``` line
        //
        // Line 4: ``` (opens fence)
        // Line 8: ``` (CLOSES fence - consumes "```bash\n[[nested-attack]]" as literal code)
        // Line 9: "More code" is PARAGRAPH
        // Line 10: ``` (opens NEW fence that NEVER closes)
        // Line 12: "Another [[valid-concept]]." is INSIDE unclosed fence
        //
        // Markdig correctly parses line 12 as inside unclosed fence → NOT extracted
        var content = @"
Valid [[security-testing]].

```
Code block
```bash
[[nested-attack]]
```
More code
```

Another [[valid-concept]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("security-testing"));
        // FIXED: valid-concept is inside unclosed code fence, so it will NOT be extracted
        Assert.That(concepts, Does.Not.Contain("valid-concept"));
        Assert.That(concepts, Does.Not.Contain("nested-attack"));
        Assert.That(concepts.Count, Is.EqualTo(1));
    }

    #endregion

    #region Escaped Character Attacks

    [Test]
    public void ExtractWikiLinks_EscapedBackticks()
    {
        // Attack: Using escaped backticks to break code inline detection
        var content = @"Normal [[regex-pattern]] and \`[[escaped-attack]]\` text.";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        // Escaped backticks are treated as literals, not code delimiters
        // So [[escaped-attack]] should be extracted since it's not in actual code
        Assert.That(concepts, Does.Contain("regex-pattern"));
        Assert.That(concepts, Does.Contain("escaped-attack"));
    }

    [Test]
    public void ExtractWikiLinks_BackslashBeforeBacktick()
    {
        // Attack: Backslash escaping attempts
        var content = @"Normal [[string-escaping]] and \`code\` with [[potential-escape]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        // Markdig should handle escaped backticks properly
        Assert.That(concepts, Does.Contain("string-escaping"));
        Assert.That(concepts, Does.Contain("potential-escape"));
    }

    #endregion

    #region Mixed Code Context Attacks

    [Test]
    public void ExtractWikiLinks_InlineCodeFollowedByCodeBlock()
    {
        // Attack: Mixing inline and block code to confuse parser
        var content = @"
Text with `[[inline-skip]]` and [[real-concept]].

```
[[block-skip]]
```

Final [[another-real]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("real-concept"));
        Assert.That(concepts, Does.Contain("another-real"));
        Assert.That(concepts, Does.Not.Contain("inline-skip"));
        Assert.That(concepts, Does.Not.Contain("block-skip"));
        Assert.That(concepts.Count, Is.EqualTo(2));
    }

    [Test]
    public void ExtractWikiLinks_MultipleInlineCodesInSameParagraph()
    {
        // Attack: Multiple inline codes with concepts to test range replacement logic
        var content = @"Text `[[skip1]]` more `[[skip2]]` and `[[skip3]]` but [[real]] here.";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("real"));
        Assert.That(concepts, Does.Not.Contain("skip1"));
        Assert.That(concepts, Does.Not.Contain("skip2"));
        Assert.That(concepts, Does.Not.Contain("skip3"));
        Assert.That(concepts.Count, Is.EqualTo(1));
    }

    #endregion

    #region HTML Code Element Attacks

    [Test]
    public void ExtractWikiLinks_HtmlCodeElements()
    {
        // Attack: HTML <code> tags (Markdig may parse these)
        var content = @"Normal [[html-parsing]] and <code>[[html-attack]]</code> text.";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        // HTML code elements are not currently filtered - potential vulnerability
        Assert.That(concepts, Does.Contain("html-parsing"));
        // EXPECTATION: This might extract html-attack depending on Markdig's HTML handling
        // This test documents current behavior
    }

    [Test]
    public void ExtractWikiLinks_HtmlPreElements()
    {
        // Attack: HTML <pre> tags
        var content = @"
Normal [[preformatted-text]].

<pre>
[[pre-attack]]
</pre>

Another [[valid]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("preformatted-text"));
        Assert.That(concepts, Does.Contain("valid"));
        // EXPECTATION: HTML pre blocks may not be filtered - documents behavior
    }

    #endregion

    #region Unicode and Special Character Attacks

    [Test]
    public void ExtractWikiLinks_UnicodeEmojis()
    {
        // Attack: Unicode emojis in concept names
        var content = @"Testing [[concept-with-emoji-🚀]] and [[normal-concept]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        // Should normalize but may contain emoji
        Assert.That(concepts, Does.Contain("normal-concept"));
        Assert.That(concepts.Count, Is.GreaterThanOrEqualTo(1));

        // Document what happens with emoji
        var emojiConcept = concepts.FirstOrDefault(c => c.Contains("emoji"));
        if (emojiConcept != null)
        {
            Console.WriteLine($"Emoji concept normalized to: {emojiConcept}");
        }
    }

    [Test]
    public void ExtractWikiLinks_UnicodeAccents()
    {
        // Attack: Accented characters
        var content = @"Testing [[café]] and [[naïve]] concepts.";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        // ToLowerInvariant should handle these
        Assert.That(concepts.Count, Is.EqualTo(2));
        // Accents should be preserved in normalized form
        Assert.That(concepts, Does.Contain("café"));
        Assert.That(concepts, Does.Contain("naïve"));
    }

    [Test]
    public void ExtractWikiLinks_ZeroWidthCharacters()
    {
        // Attack: Zero-width Unicode characters (invisible)
        var content = "Testing [[concept\u200B]] and [[normal]]."; // \u200B = zero-width space

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        // Should extract with zero-width chars included
        Assert.That(concepts, Does.Contain("normal"));
        Assert.That(concepts.Count, Is.GreaterThanOrEqualTo(1));
    }

    [Test]
    public void ExtractWikiLinks_RightToLeftCharacters()
    {
        // Attack: RTL override characters
        var content = @"Testing [[concept\u202E]] and [[normal]]."; // \u202E = RTL override

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("normal"));
        Assert.That(concepts.Count, Is.GreaterThanOrEqualTo(1));
    }

    #endregion

    #region Extremely Long Input Attacks (DoS)

    [Test]
    public void ExtractWikiLinks_VeryLongConceptName()
    {
        // Attack: Extremely long concept to test regex backtracking
        var longConcept = new string('a', 10000);
        var content = $"Normal [[performance-testing]] and [[{longConcept}]].";

        // Should not hang or crash
        var sw = System.Diagnostics.Stopwatch.StartNew();
        var concepts = MarkdownReader.ExtractWikiLinks(content);
        sw.Stop();

        Assert.That(sw.ElapsedMilliseconds, Is.LessThan(1000),
            "Regex should not have catastrophic backtracking");
        Assert.That(concepts, Does.Contain("performance-testing"));
        Assert.That(concepts.Count, Is.GreaterThanOrEqualTo(1));
    }

    [Test]
    public void ExtractWikiLinks_ThousandsOfConcepts()
    {
        // Attack: DoS via large number of concepts
        var sb = new System.Text.StringBuilder();
        for (int i = 0; i < 5000; i++)
        {
            sb.AppendLine(System.Globalization.CultureInfo.InvariantCulture, $"Concept [[concept-{i}]] here.");
        }

        var sw = System.Diagnostics.Stopwatch.StartNew();
        var concepts = MarkdownReader.ExtractWikiLinks(sb.ToString());
        sw.Stop();

        Assert.That(sw.ElapsedMilliseconds, Is.LessThan(5000),
            "Should handle large documents efficiently");
        Assert.That(concepts.Count, Is.EqualTo(5000));
    }

    [Test]
    public void ExtractWikiLinks_DeeplyNestedLists()
    {
        // Attack: Deeply nested list structure
        var sb = new System.Text.StringBuilder();
        for (int i = 0; i < 100; i++)
        {
            sb.Append(new string(' ', i * 2));
            sb.AppendLine(System.Globalization.CultureInfo.InvariantCulture, $"- Item with [[concept-{i}]]");
        }

        var sw = System.Diagnostics.Stopwatch.StartNew();
        var concepts = MarkdownReader.ExtractWikiLinks(sb.ToString());
        sw.Stop();

        Assert.That(sw.ElapsedMilliseconds, Is.LessThan(2000));
        Assert.That(concepts.Count, Is.EqualTo(100));
    }

    #endregion

    #region Malformed WikiLink Attacks

    [Test]
    public void ExtractWikiLinks_TripleBrackets()
    {
        // Attack: Triple opening brackets - T-REL-001: Malformed patterns should be rejected
        var content = @"Normal [[bracket-validation]] and [[[triple-attack]]] text.";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("bracket-validation"));
        // T-REL-001: Triple brackets are malformed and should NOT match (lookbehind/lookahead rejects them)
        Assert.That(concepts, Does.Not.Contain("triple-attack"));
    }

    [Test]
    public void ExtractWikiLinks_MissingClosingBracket()
    {
        // Attack: Incomplete WikiLink
        var content = @"Normal [[malformed-input]] and [[missing-bracket text continues.";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("malformed-input"));
        // Should not match incomplete bracket
        Assert.That(concepts, Does.Not.Contain("missing-bracket text continues"));
    }

    [Test]
    public void ExtractWikiLinks_MissingOpeningBracket()
    {
        // Attack: Malformed closing
        var content = @"Normal [[edge-case-handling]] and wrong]] text.";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("edge-case-handling"));
        Assert.That(concepts.Count, Is.EqualTo(1));
    }

    [Test]
    public void ExtractWikiLinks_EmptyBrackets()
    {
        // Attack: Empty WikiLink
        var content = @"Normal [[whitespace-validation]] and [[]] and [[another]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("whitespace-validation"));
        Assert.That(concepts, Does.Contain("another"));
        // Empty brackets normalize to empty string, should not pollute graph
        concepts = concepts.Where(c => !string.IsNullOrWhiteSpace(c)).ToList();
        Assert.That(concepts.Count, Is.EqualTo(2));
    }

    [Test]
    public void ExtractWikiLinks_WhitespaceOnlyBrackets()
    {
        // Attack: Whitespace-only concept
        var content = @"Normal [[input-sanitization]] and [[   ]] and [[another]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("input-sanitization"));
        Assert.That(concepts, Does.Contain("another"));
        // Trim should eliminate whitespace-only concepts
        concepts = concepts.Where(c => !string.IsNullOrWhiteSpace(c)).ToList();
        Assert.That(concepts.Count, Is.EqualTo(2));
    }

    [Test]
    public void ExtractWikiLinks_NewlineInBrackets()
    {
        // Attack: Newline inside WikiLink
        var content = @"Normal [[multiline-parsing]] and [[multi
line
attack]] text.";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("multiline-parsing"));
        // Current regex uses [^\]]+ which matches newlines
        // This documents the behavior - multiline concepts ARE extracted
        var multiline = concepts.FirstOrDefault(c => c.Contains("multi") || c.Contains("line"));
        if (multiline != null)
        {
            Console.WriteLine($"Multiline concept: '{multiline}'");
        }
    }

    #endregion

    #region Code Block Edge Cases

    [Test]
    public void ExtractWikiLinks_DoubleBacktickInlineCode()
    {
        // Attack: Double backtick inline code (valid markdown)
        var content = @"Normal [[inline-code-handling]] and ``[[double-backtick]]`` text.";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("inline-code-handling"));
        // Double backticks should also be treated as code
        Assert.That(concepts, Does.Not.Contain("double-backtick"));
    }

    [Test]
    public void ExtractWikiLinks_TildeCodeBlock()
    {
        // Attack: Tilde-fenced code blocks (valid in some parsers)
        var content = @"
Valid [[fence-syntax]].

~~~
[[tilde-attack]]
~~~

Another [[valid]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("fence-syntax"));
        Assert.That(concepts, Does.Contain("valid"));
        // Markdig may support tilde blocks - verify behavior
    }

    [Test]
    public void ExtractWikiLinks_UnclosedCodeBlock()
    {
        // Attack: Code block without closing fence
        var content = @"
Valid [[unclosed-fence]].

```
[[unclosed-attack]]

Another [[potentially-valid]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("unclosed-fence"));
        // Unclosed code blocks may consume rest of document
        // This test documents the behavior
    }

    [Test]
    public void ExtractWikiLinks_CodeBlockWithExtraBackticks()
    {
        // Attack: Code fence with more than 3 backticks
        var content = @"
Valid [[fence-delimiters]].

`````
[[extra-backtick-attack]]
`````

Another [[valid]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("fence-delimiters"));
        Assert.That(concepts, Does.Contain("valid"));
        Assert.That(concepts, Does.Not.Contain("extra-backtick-attack"));
    }

    #endregion

    #region Normalization Collision Attacks

    [Test]
    public void ExtractWikiLinks_NormalizationCollision()
    {
        // Attack: Different concepts that normalize to same value
        var content = @"[[Test Concept]] and [[test___concept]] and [[TEST//CONCEPT]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        // All should normalize to "test-concept"
        Assert.That(concepts.Count, Is.EqualTo(1));
        Assert.That(concepts, Does.Contain("test-concept"));
    }

    [Test]
    public void ExtractWikiLinks_LeadingTrailingHyphens()
    {
        // Attack: Hyphens that should be trimmed
        var content = @"[[--data-model--]] and [[data-model]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        // Should normalize to single "data-model"
        Assert.That(concepts.Count, Is.EqualTo(1));
        Assert.That(concepts, Does.Contain("data-model"));
    }

    [Test]
    public void ExtractWikiLinks_MultipleAdjacentHyphens()
    {
        // Attack: Multiple hyphens should collapse
        var content = @"[[test-----concept]] and [[test-concept]].";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        // Should normalize to single "test-concept"
        Assert.That(concepts.Count, Is.EqualTo(1));
        Assert.That(concepts, Does.Contain("test-concept"));
    }

    #endregion

    #region Span Manipulation Attacks

    [Test]
    public void ExtractWikiLinks_VeryLongDocument()
    {
        // Attack: Test span boundary handling with huge document
        var sb = new System.Text.StringBuilder();
        sb.AppendLine("Start [[first-concept]].");
        sb.AppendLine(new string('\n', 10000)); // Lots of newlines
        sb.AppendLine("End [[last-concept]].");

        var concepts = MarkdownReader.ExtractWikiLinks(sb.ToString());

        Assert.That(concepts, Does.Contain("first-concept"));
        Assert.That(concepts, Does.Contain("last-concept"));
        Assert.That(concepts.Count, Is.EqualTo(2));
    }

    [Test]
    public void ExtractWikiLinks_OnlyCodeBlocks()
    {
        // Attack: Document with only code blocks
        var content = @"
```
[[attack1]]
```

```
[[attack2]]
```";

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts.Count, Is.EqualTo(0));
    }

    #endregion

    #region Injection Attacks via File Content

    [Test]
    public void CountConceptOccurrences_NullByteInjection()
    {
        // Attack: Null byte injection
        var content = "[[null-byte-handling]]\0[[attack]]";

        // Should not crash
        var count = MarkdownReader.CountConceptOccurrences(content, "null-byte-handling");
        Assert.That(count, Is.GreaterThanOrEqualTo(1));
    }

    [Test]
    public void ExtractWikiLinks_ControlCharacters()
    {
        // Attack: Various control characters
        var content = "[[control-characters\u0001]] and [[normal]]."; // \u0001 = SOH

        var concepts = MarkdownReader.ExtractWikiLinks(content);

        Assert.That(concepts, Does.Contain("normal"));
        Assert.That(concepts.Count, Is.GreaterThanOrEqualTo(1));
    }

    #endregion
}
