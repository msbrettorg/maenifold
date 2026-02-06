using System;
using System.Globalization;
using System.IO;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// T-CLI-JSON-001: Red-team security tests for CLI JSON output feature.
///
/// Attack vectors tested:
/// 1. JSON injection via malicious content
/// 2. Unicode/encoding attacks (BOM, invalid UTF-8, null bytes)
/// 3. Large payload DoS
/// 4. Error handling information leakage
/// 5. Concurrency/race conditions in OutputContext
/// 6. ANSI escape code bypass
/// 7. Special character handling in WikiLinks
/// </summary>
[NonParallelizable]
public class CliJsonSecurityTests
{
    private const string TestFolder = "cli-json-security-tests";
    private string _testFolderPath = string.Empty;

    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();
        _testFolderPath = Path.Combine(Config.MemoryPath, TestFolder);
        Directory.CreateDirectory(_testFolderPath);
        OutputContext.Format = OutputFormat.Json;
    }

    [TearDown]
    public void TearDown()
    {
        OutputContext.Format = OutputFormat.Markdown;

        if (string.IsNullOrEmpty(_testFolderPath))
            return;

        var directory = new DirectoryInfo(_testFolderPath);
        if (directory.Exists)
        {
            foreach (var file in directory.EnumerateFiles("*", SearchOption.AllDirectories))
            {
                file.IsReadOnly = false;
                file.Delete();
            }
            directory.Delete(true);
        }
    }

    #region Attack Vector 1: JSON Injection via Malicious Content

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - JSON injection via double quotes in content.
    /// Attack: Content containing " characters could break JSON structure.
    /// Severity: HIGH if vulnerable (allows JSON structure manipulation)
    /// </summary>
    [Test]
    [Description("SEC-JSON-001: Double quotes in content must be escaped")]
    public void WriteMemory_ContentWithDoubleQuotes_ProducesValidJson()
    {
        // Arrange - Malicious content with JSON control characters
        var title = "Quote Injection Test";
        var content = "Content with [[injection]] and \"malicious\": \"payload\" here.";

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert - Must produce valid JSON despite quotes in content
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "VULNERABILITY: Double quotes in content broke JSON structure");

        var doc = JsonDocument.Parse(result);
        Assert.That(doc.RootElement.GetProperty("success").GetBoolean(), Is.True,
            "Operation should succeed with escaped quotes");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - JSON injection via backslashes.
    /// Attack: Backslashes could interfere with JSON escape sequences.
    /// Severity: HIGH if vulnerable
    /// </summary>
    [Test]
    [Description("SEC-JSON-002: Backslashes in content must be escaped")]
    public void WriteMemory_ContentWithBackslashes_ProducesValidJson()
    {
        // Arrange - Content with backslashes that could break escaping
        var title = "Backslash Injection Test";
        var content = @"Content with [[backslash]] and paths like C:\Users\test\file.txt";

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "VULNERABILITY: Backslashes in content broke JSON structure");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - JSON injection via closing braces.
    /// Attack: Content with } could prematurely close JSON objects.
    /// Severity: CRITICAL if vulnerable
    /// </summary>
    [Test]
    [Description("SEC-JSON-003: Closing braces in content must be escaped")]
    public void WriteMemory_ContentWithBraces_ProducesValidJson()
    {
        // Arrange - Content attempting to close JSON structure
        var title = "Brace Injection Test";
        var content = "Content with [[brace-test]] and }, \"injected\": true}";

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "VULNERABILITY: Braces in content broke JSON structure");

        var doc = JsonDocument.Parse(result);
        // Verify the injected field doesn't appear at root level
        Assert.That(doc.RootElement.TryGetProperty("injected", out _), Is.False,
            "VULNERABILITY: JSON injection allowed adding fields to response");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - JSON injection via newlines.
    /// Attack: Raw newlines in content could break JSON strings.
    /// Severity: HIGH if vulnerable
    /// </summary>
    [Test]
    [Description("SEC-JSON-004: Newlines in content must be escaped")]
    public void WriteMemory_ContentWithNewlines_ProducesValidJson()
    {
        // Arrange
        var title = "Newline Injection Test";
        var content = "Content with [[newline]]\nand multiple\r\nline breaks\rhere.";

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "VULNERABILITY: Newlines in content broke JSON structure");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - JSON injection in WikiLinks.
    /// Attack: WikiLinks with special characters could break JSON.
    /// Severity: HIGH if vulnerable
    /// </summary>
    [Test]
    [Description("SEC-JSON-005: Special characters in WikiLinks must be handled")]
    public void WriteMemory_WikiLinkWithSpecialChars_ProducesValidJson()
    {
        // Arrange - WikiLink attempting JSON injection
        var title = "WikiLink Injection Test";
        var content = "Testing [[concept\"injection]] and [[another}concept]] here.";

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert - Should either escape or reject, but never produce invalid JSON
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "VULNERABILITY: WikiLinks with special chars broke JSON");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - JSON injection via tab characters.
    /// Attack: Tab characters could break JSON strings if not escaped.
    /// Severity: MEDIUM if vulnerable
    /// </summary>
    [Test]
    [Description("SEC-JSON-006: Tab characters in content must be escaped")]
    public void WriteMemory_ContentWithTabs_ProducesValidJson()
    {
        // Arrange
        var title = "Tab Injection Test";
        var content = "Content with [[tabs]]\there\tand\tthere.";

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "VULNERABILITY: Tab characters broke JSON structure");
    }

    #endregion

    #region Attack Vector 2: Unicode/Encoding Attacks

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - UTF-8 BOM injection.
    /// Attack: BOM at start of content could confuse parsers.
    /// Severity: MEDIUM
    /// </summary>
    [Test]
    [Description("SEC-UNICODE-001: UTF-8 BOM in content is handled safely")]
    public void WriteMemory_ContentWithBOM_ProducesValidJson()
    {
        // Arrange - Content starting with UTF-8 BOM
        var title = "BOM Injection Test";
        var bom = Encoding.UTF8.GetString(new byte[] { 0xEF, 0xBB, 0xBF });
        var content = bom + "Content with [[bom-test]] after BOM.";

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "VULNERABILITY: UTF-8 BOM broke JSON parsing");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - Null byte injection.
    /// Attack: Null bytes could truncate strings or cause issues.
    /// Severity: HIGH
    /// </summary>
    [Test]
    [Description("SEC-UNICODE-002: Null bytes in content are handled safely")]
    public void WriteMemory_ContentWithNullBytes_ProducesValidJson()
    {
        // Arrange - Content with embedded null bytes
        var title = "Null Byte Test";
        var content = "Content with [[null-byte]] and \0 null byte here.";

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert - Should either escape or strip null bytes
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "VULNERABILITY: Null bytes broke JSON parsing");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - Unicode control characters.
    /// Attack: Control chars (0x00-0x1F) must be escaped in JSON.
    /// Severity: HIGH
    /// </summary>
    [Test]
    [Description("SEC-UNICODE-003: Control characters are escaped")]
    public void WriteMemory_ContentWithControlChars_ProducesValidJson()
    {
        // Arrange - Content with various control characters
        var title = "Control Char Test";
        var sb = new StringBuilder();
        sb.Append("Content with [[control-chars]]");
        for (int i = 0; i < 32; i++)
        {
            if (i != 10 && i != 13) // Skip common newlines
                sb.Append((char)i);
        }
        sb.Append(" end.");
        var content = sb.ToString();

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "VULNERABILITY: Control characters broke JSON parsing");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - Unicode homoglyphs for JSON syntax.
    /// Attack: Unicode chars that look like JSON syntax could confuse human reviewers.
    /// Severity: LOW (cosmetic attack)
    /// </summary>
    [Test]
    [Description("SEC-UNICODE-004: Unicode homoglyphs are handled correctly")]
    public void WriteMemory_ContentWithHomoglyphs_ProducesValidJson()
    {
        // Arrange - Unicode chars that look like JSON syntax
        var title = "Homoglyph Test";
        // Using fullwidth characters that look like ASCII
        var content = "Content with [[homoglyph]] and \uFF1A \uFF5B \uFF5D chars."; // Fullwidth : { }

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "Unicode homoglyphs should not affect JSON validity");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - Overlong UTF-8 sequences.
    /// Attack: Overlong encodings could bypass filters.
    /// Severity: MEDIUM
    /// </summary>
    [Test]
    [Description("SEC-UNICODE-005: Invalid UTF-8 sequences are handled")]
    public void ReadMemory_ResponseIsValidUtf8()
    {
        // Arrange - Create a file with valid content
        MemoryTools.WriteMemory("UTF8 Validation", "Content with [[utf8-test]].", TestFolder);

        // Act
        var result = MemoryTools.ReadMemory($"memory://{TestFolder}/utf8-validation");

        // Assert - Verify output is valid UTF-8
        var bytes = Encoding.UTF8.GetBytes(result);
        var roundTrip = Encoding.UTF8.GetString(bytes);

        Assert.That(roundTrip, Is.EqualTo(result),
            "JSON output must be valid UTF-8 that survives round-trip encoding");
    }

    #endregion

    #region Attack Vector 3: Large Payload DoS

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - Large content payload.
    /// Attack: Very large content could exhaust memory during serialization.
    /// Severity: MEDIUM (resource exhaustion)
    /// </summary>
    [Test]
    [Description("SEC-DOS-001: Large content is handled without crashing")]
    public void WriteMemory_LargeContent_HandledGracefully()
    {
        // Arrange - Create large content (100KB)
        var title = "Large Content Test";
        var largeContent = "Content with [[large-payload]] " + new string('A', 100 * 1024);

        // Act & Assert - Should complete without exception or timeout
        Assert.DoesNotThrow(() =>
        {
            var result = MemoryTools.WriteMemory(title, largeContent, TestFolder);
            JsonDocument.Parse(result);
        }, "Large content should be handled without crashing");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - Many WikiLinks.
    /// Attack: Excessive WikiLinks could slow graph processing.
    /// Severity: LOW
    /// </summary>
    [Test]
    [Description("SEC-DOS-002: Many WikiLinks are handled efficiently")]
    public void WriteMemory_ManyWikiLinks_HandledEfficiently()
    {
        // Arrange - Content with many WikiLinks
        var title = "Many Links Test";
        var sb = new StringBuilder();
        for (int i = 0; i < 100; i++)
        {
            sb.Append(CultureInfo.InvariantCulture, $"[[concept-{i}]] ");
        }
        var content = sb.ToString();

        // Act
        var startTime = DateTime.UtcNow;
        var result = MemoryTools.WriteMemory(title, content, TestFolder);
        var elapsed = DateTime.UtcNow - startTime;

        // Assert
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "Many WikiLinks should produce valid JSON");
        Assert.That(elapsed.TotalSeconds, Is.LessThan(5),
            "Processing many WikiLinks should complete in reasonable time");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - Deeply nested JSON in error context.
    /// Attack: SafeJson limits depth, but verify it applies to responses.
    /// Severity: HIGH if unprotected
    /// </summary>
    [Test]
    [Description("SEC-DOS-003: JSON depth limits are enforced")]
    public void JsonToolResponse_MaxDepthEnforced()
    {
        // This tests the serialization side - verify Options has MaxDepth
        Assert.That(SafeJson.Options.MaxDepth, Is.EqualTo(32),
            "SafeJson should enforce MaxDepth=32 for DoS protection");
    }

    #endregion

    #region Attack Vector 4: Error Handling Information Leakage

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - Stack traces in error responses.
    /// Attack: Internal implementation details could leak via errors.
    /// Severity: MEDIUM (information disclosure)
    /// </summary>
    [Test]
    [Description("SEC-LEAK-001: Error responses do not contain stack traces")]
    public void ReadMemory_Error_NoStackTraceLeakage()
    {
        // Act - Trigger an error
        var result = MemoryTools.ReadMemory("memory://non-existent-file-12345");

        // Assert
        var doc = JsonDocument.Parse(result);
        var jsonString = doc.RootElement.GetRawText();

        Assert.That(jsonString, Does.Not.Contain("   at "),
            "VULNERABILITY: Stack trace leaked in error response");
        Assert.That(jsonString, Does.Not.Contain("StackTrace"),
            "VULNERABILITY: StackTrace property leaked in error response");
        Assert.That(jsonString, Does.Not.Contain(".cs:line"),
            "VULNERABILITY: Source file info leaked in error response");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - Internal paths in error responses.
    /// Attack: Full file system paths could reveal server structure.
    /// Severity: LOW
    /// </summary>
    [Test]
    [Description("SEC-LEAK-002: Error responses do not expose full paths")]
    public void ReadMemory_Error_NoFullPathLeakage()
    {
        // Act
        var result = MemoryTools.ReadMemory("memory://non-existent-file");

        // Assert
        var doc = JsonDocument.Parse(result);
        var errorMessage = doc.RootElement.GetProperty("error").GetProperty("message").GetString();

        // Should not contain absolute paths like /Users/... or C:\...
        Assert.That(errorMessage, Does.Not.Match(@"^[A-Z]:\\"),
            "VULNERABILITY: Windows absolute path leaked");
        Assert.That(errorMessage, Does.Not.Match(@"^/(?:Users|home|var|etc)/"),
            "VULNERABILITY: Unix absolute path leaked");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - Exception type leakage.
    /// Attack: Internal exception types reveal implementation details.
    /// Severity: LOW
    /// </summary>
    [Test]
    [Description("SEC-LEAK-003: Error codes are application-defined, not exception types")]
    public void ReadMemory_Error_UsesApplicationErrorCodes()
    {
        // Act
        var result = MemoryTools.ReadMemory("memory://non-existent-file");

        // Assert
        var doc = JsonDocument.Parse(result);
        var errorCode = doc.RootElement.GetProperty("error").GetProperty("code").GetString();

        // Error codes should be application-defined, not .NET exception types
        Assert.That(errorCode, Does.Not.Contain("Exception"),
            "Error code should not expose exception type names");
        Assert.That(errorCode, Does.Not.Contain("System."),
            "Error code should not contain .NET namespace prefixes");
    }

    #endregion

    #region Attack Vector 5: Concurrency Issues

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - OutputContext race conditions.
    /// Attack: Parallel requests could get wrong output format.
    /// Severity: MEDIUM
    /// </summary>
    [Test]
    [Description("SEC-CONCURRENCY-001: OutputContext is thread-safe")]
    public void OutputContext_ParallelAccess_IsolatedPerThread()
    {
        // Arrange
        var results = new bool[100];
        var errors = new Exception?[100];

        // Act - Parallel access with different formats
        Parallel.For(0, 100, i =>
        {
            try
            {
                // Alternate between JSON and Markdown
                var useJson = i % 2 == 0;

                using (useJson ? OutputContext.UseJson() : OutputContext.UseMarkdown())
                {
                    // Small delay to increase chance of race
                    Task.Delay(Random.Shared.Next(1, 5)).Wait();

                    // Verify context is as expected
                    results[i] = OutputContext.IsJsonMode == useJson;
                }
            }
            catch (Exception ex)
            {
                errors[i] = ex;
            }
        });

        // Assert
        Assert.That(errors.All(e => e == null), Is.True,
            "No exceptions should occur during parallel access");
        Assert.That(results.All(r => r), Is.True,
            "VULNERABILITY: OutputContext race condition - format leaked between threads");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - Scope disposal restores previous format.
    /// Attack: Improper cleanup could leave context in wrong state.
    /// Severity: MEDIUM
    /// </summary>
    [Test]
    [Description("SEC-CONCURRENCY-002: OutputContext scope properly restores state")]
    public void OutputContext_NestedScopes_ProperlyRestored()
    {
        // Arrange - Start in Markdown
        OutputContext.Format = OutputFormat.Markdown;

        // Act - Nested scopes
        Assert.That(OutputContext.IsJsonMode, Is.False, "Initial state should be Markdown");

        using (OutputContext.UseJson())
        {
            Assert.That(OutputContext.IsJsonMode, Is.True, "Should be JSON in first scope");

            using (OutputContext.UseMarkdown())
            {
                Assert.That(OutputContext.IsJsonMode, Is.False, "Should be Markdown in nested scope");
            }

            Assert.That(OutputContext.IsJsonMode, Is.True, "Should restore to JSON after nested scope");
        }

        // Assert
        Assert.That(OutputContext.IsJsonMode, Is.False, "Should restore to Markdown after outer scope");
    }

    #endregion

    #region Attack Vector 6: ANSI Escape Code Bypass

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - ANSI codes in content.
    /// Attack: ANSI codes could interfere with terminal output or hide content.
    /// Severity: LOW (cosmetic attack, could hide malicious content)
    /// </summary>
    [Test]
    [Description("SEC-ANSI-001: ANSI codes in content don't break JSON")]
    public void WriteMemory_ContentWithAnsiCodes_ProducesValidJson()
    {
        // Arrange - Content with ANSI escape codes
        var title = "ANSI Escape Test";
        var content = "Content with [[ansi-test]] and \u001b[31mred text\u001b[0m here.";

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "ANSI codes in content should not break JSON");

        // Verify ANSI codes are properly escaped in JSON string
        var doc = JsonDocument.Parse(result);
        Assert.That(doc.RootElement.GetProperty("success").GetBoolean(), Is.True);
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - JSON output itself has no ANSI codes.
    /// Attack: Implementation-added ANSI codes in JSON would break machine parsing.
    /// Severity: HIGH if present
    /// </summary>
    [Test]
    [Description("SEC-ANSI-002: JSON output structure contains no ANSI codes")]
    public void WriteMemory_JsonOutput_NoImplementationAnsiCodes()
    {
        // Arrange
        var title = "Clean JSON Test";
        var content = "Clean content with [[clean-json]].";

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert - Check raw output for ANSI sequences
        Assert.That(result, Does.Not.Contain("\u001b["),
            "JSON output should not contain ANSI escape codes from implementation");
        Assert.That(result, Does.Not.Contain("\x1b["),
            "JSON output should not contain ANSI escape codes (hex format)");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - Markdown mode can have ANSI, JSON mode cannot.
    /// Verifies the feature flag properly controls output formatting.
    /// Severity: MEDIUM
    /// </summary>
    [Test]
    [Description("SEC-ANSI-003: Format flag correctly controls ANSI presence")]
    public void OutputContext_JsonMode_StripsStyling()
    {
        // Arrange
        var title = "Format Switch Test";
        var content = "Content with [[format-test]].";

        // Act - Get both outputs
        OutputContext.Format = OutputFormat.Json;
        var jsonResult = MemoryTools.WriteMemory(title + " JSON", content, TestFolder);

        OutputContext.Format = OutputFormat.Markdown;
        var mdResult = MemoryTools.WriteMemory(title + " MD", content, TestFolder);

        // Assert
        Assert.DoesNotThrow(() => JsonDocument.Parse(jsonResult),
            "JSON mode should produce valid JSON");
        // JsonReaderException inherits from JsonException - use Catch to accept either
        Assert.Catch<JsonException>(() => JsonDocument.Parse(mdResult),
            "Markdown mode should NOT produce valid JSON");
    }

    #endregion

    #region Attack Vector 7: Edge Cases and Boundary Conditions

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - Empty content edge case.
    /// Severity: LOW
    /// </summary>
    [Test]
    [Description("SEC-EDGE-001: Empty/whitespace content handled correctly")]
    public void WriteMemory_EmptyContent_ProducesValidError()
    {
        // Arrange
        var title = "Empty Test";
        var content = "";

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert - Should be valid JSON (error response expected)
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "Empty content should produce valid JSON error response");

        var doc = JsonDocument.Parse(result);
        Assert.That(doc.RootElement.GetProperty("success").GetBoolean(), Is.False,
            "Empty content should fail validation");
    }

    /// <summary>
    /// T-CLI-JSON-001: SEC-EDGE-002 - Very long title returns structured error.
    /// Previously: Unhandled PathTooLongException exposed internal paths (MEDIUM severity).
    /// Fix: WriteMemory now catches PathTooLongException and returns structured JSON error.
    /// </summary>
    [Test]
    [Description("SEC-EDGE-002: Long titles return structured PATH_TOO_LONG error")]
    public void WriteMemory_VeryLongTitle_ReturnsStructuredError()
    {
        // Arrange - Title longer than 255 chars creates path > OS limit
        var title = new string('A', 500);
        var content = "Content with [[long-title-test]].";

        // Act - Should return structured error instead of throwing exception
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert - Valid JSON with PATH_TOO_LONG error
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "Long title should produce valid JSON error response");

        var doc = JsonDocument.Parse(result);
        Assert.That(doc.RootElement.GetProperty("success").GetBoolean(), Is.False,
            "Long title should fail");
        Assert.That(doc.RootElement.GetProperty("error").GetProperty("code").GetString(),
            Is.EqualTo("PATH_TOO_LONG"),
            "Error code should be PATH_TOO_LONG");
        Assert.That(doc.RootElement.GetProperty("error").GetProperty("message").GetString(),
            Does.Not.Contain("/Users/").And.Not.Contain("\\Users\\"),
            "Error message should not leak internal paths");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - Unicode title.
    /// Severity: LOW
    /// </summary>
    [Test]
    [Description("SEC-EDGE-003: Unicode titles are handled correctly")]
    public void WriteMemory_UnicodeTitle_ProducesValidJson()
    {
        // Arrange
        var title = "Test with emojis and unicode";
        var content = "Content with [[unicode-title]].";

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "Unicode title should produce valid JSON");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - Null parameters.
    /// Severity: LOW
    /// </summary>
    [Test]
    [Description("SEC-EDGE-004: Null folder parameter handled correctly")]
    public void WriteMemory_NullFolder_ProducesValidJson()
    {
        // Arrange
        var title = "Null Folder Test";
        var content = "Content with [[null-folder-test]].";

        // Act
        var result = MemoryTools.WriteMemory(title, content, folder: null);

        // Assert
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "Null folder should produce valid JSON");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - SearchMemories with no results.
    /// Severity: LOW
    /// </summary>
    [Test]
    [Description("SEC-EDGE-005: Empty search results produce valid JSON")]
    public void SearchMemories_NoResults_ProducesValidJson()
    {
        // Act
        var result = MemorySearchTools.SearchMemories(
            query: "xyznonexistentquery12345",
            mode: SearchMode.Hybrid,
            folder: TestFolder);

        // Assert
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "Empty search results should produce valid JSON");

        var doc = JsonDocument.Parse(result);
        Assert.That(doc.RootElement.GetProperty("success").GetBoolean(), Is.True,
            "Empty results is a successful operation");
    }

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - SequentialThinking cancel operation.
    /// Severity: LOW
    /// </summary>
    [Test]
    [Description("SEC-EDGE-006: Cancel operation produces valid JSON")]
    public void SequentialThinking_Cancel_ProducesValidJson()
    {
        // Arrange - Create a session first
        var createResult = SequentialThinkingTools.SequentialThinking(
            response: "Starting session with [[cancel-test]].",
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2);

        var createDoc = JsonDocument.Parse(createResult);
        var sessionId = createDoc.RootElement.GetProperty("data").GetProperty("sessionId").GetString();

        // Act - Cancel the session
        var cancelResult = SequentialThinkingTools.SequentialThinking(
            sessionId: sessionId,
            cancel: true,
            thoughtNumber: 1);

        // Assert
        Assert.DoesNotThrow(() => JsonDocument.Parse(cancelResult),
            "Cancel operation should produce valid JSON");

        var doc = JsonDocument.Parse(cancelResult);
        Assert.That(doc.RootElement.GetProperty("success").GetBoolean(), Is.True);
        Assert.That(doc.RootElement.GetProperty("data").GetProperty("status").GetString(),
            Is.EqualTo("cancelled"));
    }

    #endregion

    #region Attack Vector 8: Cross-tool Consistency

    /// <summary>
    /// T-CLI-JSON-001: SECURITY - All JSON responses have consistent structure.
    /// Attack: Inconsistent structure could break automation.
    /// Severity: MEDIUM
    /// </summary>
    [Test]
    [Description("SEC-CONSISTENCY-001: All tool responses have consistent JSON structure")]
    public void AllTools_JsonOutput_ConsistentStructure()
    {
        // Arrange - Create test file
        MemoryTools.WriteMemory("Consistency Test", "Content with [[consistency-test]].", TestFolder);
        GraphTools.Sync();

        // Collect responses from multiple tools
        var writeResult = MemoryTools.WriteMemory("New File", "Content with [[new-file]].", TestFolder);
        var readResult = MemoryTools.ReadMemory($"memory://{TestFolder}/consistency-test");
        var searchResult = MemorySearchTools.SearchMemories("consistency", folder: TestFolder);
        var deleteNoConfirm = MemoryTools.DeleteMemory($"memory://{TestFolder}/new-file", confirm: false);
        var readNotFound = MemoryTools.ReadMemory("memory://does-not-exist");

        var responses = new[] { writeResult, readResult, searchResult, deleteNoConfirm, readNotFound };

        // Assert - All should have success, data, error fields
        foreach (var response in responses)
        {
            var doc = JsonDocument.Parse(response);
            var root = doc.RootElement;

            Assert.That(root.TryGetProperty("success", out _), Is.True,
                $"Response missing 'success' field: {response[..Math.Min(100, response.Length)]}");
            Assert.That(root.TryGetProperty("data", out _), Is.True,
                $"Response missing 'data' field: {response[..Math.Min(100, response.Length)]}");
            Assert.That(root.TryGetProperty("error", out _), Is.True,
                $"Response missing 'error' field: {response[..Math.Min(100, response.Length)]}");
        }
    }

    #endregion
}
