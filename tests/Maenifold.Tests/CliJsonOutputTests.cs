using System;
using System.IO;
using System.Text;
using System.Text.Json;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// TDD tests for CLI JSON output feature (FR-8.x).
/// Tests MUST FAIL initially - implementation follows tests.
///
/// T-CLI-JSON-001: RTM FR-8.1, FR-8.2, FR-8.3, FR-8.4, FR-8.5
///
/// These tests validate that when --json flag is provided:
/// - All MCP tools return valid JSON (FR-8.2)
/// - JSON includes success, data, error fields (FR-8.3)
/// - Error responses have code and message (FR-8.4)
/// - Omitting --json returns markdown for backward compatibility (FR-8.5)
/// </summary>
[NonParallelizable]
public class CliJsonOutputTests
{
    private const string TestFolder = "cli-json-tests";
    private string _testFolderPath = string.Empty;

    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();
        _testFolderPath = Path.Combine(Config.MemoryPath, TestFolder);
        Directory.CreateDirectory(_testFolderPath);
        // T-CLI-JSON-001: Enable JSON output mode for tests
        OutputContext.Format = OutputFormat.Json;
    }

    [TearDown]
    public void TearDown()
    {
        // T-CLI-JSON-001: Reset to default markdown mode after tests
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

    #region TC-CLI-JSON-001: JSON flag produces valid JSON output

    /// <summary>
    /// TC-CLI-JSON-001: RTM FR-8.1, FR-8.2
    /// Verify that tool output with JSON mode enabled produces valid parseable JSON.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-001: JSON flag produces valid JSON output for WriteMemory")]
    public void WriteMemory_WithJsonFlag_ProducesValidJson()
    {
        // Arrange
        var title = "JSON Output Test";
        var content = "Test content with [[json-output]] concept.";

        // Act - Call tool (implementation will need to support JSON mode)
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert - Output should be valid JSON when JSON mode is implemented
        // This test will FAIL until FR-8.1 is implemented
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "WriteMemory output should be valid JSON when --json flag is used");
    }

    /// <summary>
    /// TC-CLI-JSON-001: RTM FR-8.1, FR-8.2
    /// Verify that SequentialThinking output is valid JSON.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-001: JSON flag produces valid JSON output for SequentialThinking")]
    public void SequentialThinking_WithJsonFlag_ProducesValidJson()
    {
        // Arrange
        var response = "Test thought with [[sequential-thinking]] concept.";

        // Act
        var result = SequentialThinkingTools.SequentialThinking(
            response: response,
            nextThoughtNeeded: false,
            thoughtNumber: 0,
            totalThoughts: 1,
            conclusion: "Test conclusion with [[test-concept]].");

        // Assert - This will FAIL until JSON output is implemented
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "SequentialThinking output should be valid JSON when --json flag is used");
    }

    /// <summary>
    /// TC-CLI-JSON-001: RTM FR-8.1, FR-8.2
    /// Verify that SearchMemories output is valid JSON.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-001: JSON flag produces valid JSON output for SearchMemories")]
    public void SearchMemories_WithJsonFlag_ProducesValidJson()
    {
        // Arrange - Create a memory file to search
        MemoryTools.WriteMemory("Search Test", "Content about [[testing]] concepts.", TestFolder);
        GraphTools.Sync();

        // Act
        var result = MemorySearchTools.SearchMemories(
            query: "testing",
            mode: SearchMode.Hybrid,
            pageSize: 10,
            page: 1,
            folder: TestFolder);

        // Assert - This will FAIL until JSON output is implemented
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "SearchMemories output should be valid JSON when --json flag is used");
    }

    /// <summary>
    /// TC-CLI-JSON-001: RTM FR-8.1, FR-8.2
    /// Verify that ReadMemory output is valid JSON.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-001: JSON flag produces valid JSON output for ReadMemory")]
    public void ReadMemory_WithJsonFlag_ProducesValidJson()
    {
        // Arrange - Create a memory file
        MemoryTools.WriteMemory("Read Test", "Content with [[read-memory]] concept.", TestFolder);

        // Act
        var result = MemoryTools.ReadMemory($"memory://{TestFolder}/read-test");

        // Assert - This will FAIL until JSON output is implemented
        Assert.DoesNotThrow(() => JsonDocument.Parse(result),
            "ReadMemory output should be valid JSON when --json flag is used");
    }

    /// <summary>
    /// TC-CLI-JSON-001: RTM FR-8.1, FR-8.2
    /// Verify that BuildContext output is valid JSON.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-001: JSON flag produces valid JSON output for BuildContext")]
    public void BuildContext_WithJsonFlag_ProducesValidJson()
    {
        // Arrange - Create memory with concepts
        MemoryTools.WriteMemory("Context Test", "Content about [[graph-context]] and [[relationships]].", TestFolder);
        GraphTools.Sync();

        // Act - BuildContext returns BuildContextResult which serializes to JSON
        var result = GraphTools.BuildContext("graph-context", depth: 1, maxEntities: 10);
        var jsonResult = JsonSerializer.Serialize(result);

        // Assert - BuildContext returns a structured object, verify it serializes properly
        Assert.DoesNotThrow(() => JsonDocument.Parse(jsonResult),
            "BuildContext result should serialize to valid JSON");
    }

    #endregion

    #region TC-CLI-JSON-002: JSON includes required fields (success, data, error)

    /// <summary>
    /// TC-CLI-JSON-002: RTM FR-8.3
    /// Verify that successful JSON response includes success=true, data object, and error=null.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-002: JSON includes required fields for successful WriteMemory")]
    public void WriteMemory_JsonSuccess_HasRequiredFields()
    {
        // Arrange
        var title = "Required Fields Test";
        var content = "Content with [[required-fields]] validation.";

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert - Verify JSON structure has required fields
        // This will FAIL until FR-8.3 is implemented
        var doc = JsonDocument.Parse(result);
        var root = doc.RootElement;

        Assert.That(root.TryGetProperty("success", out var successProp), Is.True,
            "JSON response must include 'success' field");
        Assert.That(successProp.GetBoolean(), Is.True,
            "Success field should be true for successful operation");

        Assert.That(root.TryGetProperty("data", out var dataProp), Is.True,
            "JSON response must include 'data' field");
        Assert.That(dataProp.ValueKind, Is.Not.EqualTo(JsonValueKind.Null),
            "Data field should not be null for successful operation");

        Assert.That(root.TryGetProperty("error", out var errorProp), Is.True,
            "JSON response must include 'error' field");
        Assert.That(errorProp.ValueKind, Is.EqualTo(JsonValueKind.Null),
            "Error field should be null for successful operation");
    }

    /// <summary>
    /// TC-CLI-JSON-002: RTM FR-8.3
    /// Verify that SearchMemories JSON response includes required fields.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-002: JSON includes required fields for SearchMemories")]
    public void SearchMemories_JsonSuccess_HasRequiredFields()
    {
        // Arrange
        MemoryTools.WriteMemory("Fields Test", "Content for [[field-validation]] test.", TestFolder);
        GraphTools.Sync();

        // Act
        var result = MemorySearchTools.SearchMemories(
            query: "field-validation",
            mode: SearchMode.Hybrid,
            folder: TestFolder);

        // Assert - This will FAIL until FR-8.3 is implemented
        var doc = JsonDocument.Parse(result);
        var root = doc.RootElement;

        Assert.That(root.TryGetProperty("success", out _), Is.True,
            "JSON response must include 'success' field");
        Assert.That(root.TryGetProperty("data", out _), Is.True,
            "JSON response must include 'data' field");
        Assert.That(root.TryGetProperty("error", out _), Is.True,
            "JSON response must include 'error' field");
    }

    /// <summary>
    /// TC-CLI-JSON-002: RTM FR-8.3
    /// Verify that SequentialThinking JSON response includes required fields.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-002: JSON includes required fields for SequentialThinking")]
    public void SequentialThinking_JsonSuccess_HasRequiredFields()
    {
        // Arrange & Act
        var result = SequentialThinkingTools.SequentialThinking(
            response: "Thought with [[required-fields-test]].",
            nextThoughtNeeded: false,
            thoughtNumber: 0,
            totalThoughts: 1,
            conclusion: "Conclusion with [[test-complete]].");

        // Assert - This will FAIL until FR-8.3 is implemented
        var doc = JsonDocument.Parse(result);
        var root = doc.RootElement;

        Assert.That(root.TryGetProperty("success", out var successProp), Is.True,
            "JSON response must include 'success' field");
        Assert.That(root.TryGetProperty("data", out var dataProp), Is.True,
            "JSON response must include 'data' field");
        Assert.That(root.TryGetProperty("error", out _), Is.True,
            "JSON response must include 'error' field");

        // Verify data contains session-specific fields
        Assert.That(dataProp.TryGetProperty("sessionId", out _), Is.True,
            "SequentialThinking data should include sessionId");
    }

    #endregion

    #region TC-CLI-JSON-003: Error responses have structured error object

    /// <summary>
    /// TC-CLI-JSON-003: RTM FR-8.4
    /// Verify that error responses include success=false and structured error object.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-003: Error responses have structured error object")]
    public void ReadMemory_NotFound_HasStructuredError()
    {
        // Arrange - Use non-existent file
        var nonExistentUri = "memory://non-existent-file-12345";

        // Act
        var result = MemoryTools.ReadMemory(nonExistentUri);

        // Assert - This will FAIL until FR-8.4 is implemented
        var doc = JsonDocument.Parse(result);
        var root = doc.RootElement;

        Assert.That(root.TryGetProperty("success", out var successProp), Is.True,
            "Error JSON response must include 'success' field");
        Assert.That(successProp.GetBoolean(), Is.False,
            "Success should be false for error response");

        Assert.That(root.TryGetProperty("data", out var dataProp), Is.True,
            "Error JSON response must include 'data' field");
        Assert.That(dataProp.ValueKind, Is.EqualTo(JsonValueKind.Null),
            "Data should be null for error response");

        Assert.That(root.TryGetProperty("error", out var errorProp), Is.True,
            "Error JSON response must include 'error' field");
        Assert.That(errorProp.ValueKind, Is.Not.EqualTo(JsonValueKind.Null),
            "Error field should not be null for error response");

        // Verify error object structure
        Assert.That(errorProp.TryGetProperty("code", out var codeProp), Is.True,
            "Error object must include 'code' field");
        Assert.That(codeProp.GetString(), Is.Not.Empty,
            "Error code should not be empty");

        Assert.That(errorProp.TryGetProperty("message", out var messageProp), Is.True,
            "Error object must include 'message' field");
        Assert.That(messageProp.GetString(), Is.Not.Empty,
            "Error message should not be empty");
    }

    /// <summary>
    /// TC-CLI-JSON-003: RTM FR-8.4
    /// Verify that SequentialThinking validation error has structured error object.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-003: SequentialThinking validation error has structured error")]
    public void SequentialThinking_ValidationError_HasStructuredError()
    {
        // Arrange - Response without WikiLinks should fail validation
        var responseWithoutLinks = "This response has no wiki links at all.";

        // Act
        var result = SequentialThinkingTools.SequentialThinking(
            response: responseWithoutLinks,
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 2);

        // Assert - This will FAIL until FR-8.4 is implemented
        var doc = JsonDocument.Parse(result);
        var root = doc.RootElement;

        Assert.That(root.TryGetProperty("success", out var successProp), Is.True,
            "Error JSON response must include 'success' field");
        Assert.That(successProp.GetBoolean(), Is.False,
            "Success should be false for validation error");

        Assert.That(root.TryGetProperty("error", out var errorProp), Is.True,
            "Error JSON response must include 'error' field");
        Assert.That(errorProp.TryGetProperty("code", out _), Is.True,
            "Error object must include 'code' field");
        Assert.That(errorProp.TryGetProperty("message", out var messageProp), Is.True,
            "Error object must include 'message' field");
        Assert.That(messageProp.GetString(), Does.Contain("WikiLink"),
            "Error message should indicate WikiLink validation failure");
    }

    /// <summary>
    /// TC-CLI-JSON-003: RTM FR-8.4
    /// Verify that DeleteMemory with confirm=false returns error.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-003: DeleteMemory without confirmation has structured error")]
    public void DeleteMemory_WithoutConfirmation_HasStructuredError()
    {
        // Arrange - Create a file then try to delete without confirmation
        MemoryTools.WriteMemory("Delete Test", "Content with [[delete-test]] concept.", TestFolder);

        // Act - Delete without confirm=true should fail
        var result = MemoryTools.DeleteMemory($"memory://{TestFolder}/delete-test", confirm: false);

        // Assert - This will FAIL until FR-8.4 is implemented
        var doc = JsonDocument.Parse(result);
        var root = doc.RootElement;

        Assert.That(root.TryGetProperty("success", out var successProp), Is.True,
            "Response must include 'success' field");
        Assert.That(successProp.GetBoolean(), Is.False,
            "Success should be false when confirmation required");

        Assert.That(root.TryGetProperty("error", out var errorProp), Is.True,
            "Response must include 'error' field");
        Assert.That(errorProp.TryGetProperty("code", out _), Is.True,
            "Error must have 'code' field");
        Assert.That(errorProp.TryGetProperty("message", out _), Is.True,
            "Error must have 'message' field");
    }

    #endregion

    #region TC-CLI-JSON-004: Omitting flag returns markdown (backward compat)

    /// <summary>
    /// TC-CLI-JSON-004: RTM FR-8.5
    /// Verify that without --json flag, output remains markdown for backward compatibility.
    /// Note: This test validates CURRENT behavior should be preserved.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-004: Omitting --json flag returns markdown output")]
    public void WriteMemory_WithoutJsonFlag_ReturnsMarkdown()
    {
        // Arrange - Switch to Markdown mode for backward compat test
        OutputContext.Format = OutputFormat.Markdown;
        var title = "Markdown Output Test";
        var content = "Content with [[markdown-output]] concept.";

        // Act - Current behavior without JSON flag
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert - Current output should NOT be JSON (it's markdown)
        // This validates backward compatibility is maintained
        Assert.That(result, Does.StartWith("Created memory FILE:"),
            "Without --json flag, WriteMemory should return markdown-style output");

        // Verify it's NOT valid JSON (current behavior)
        Assert.Catch<JsonException>(() => JsonDocument.Parse(result),
            "Without --json flag, output should NOT be valid JSON (backward compat)");
    }

    /// <summary>
    /// TC-CLI-JSON-004: RTM FR-8.5
    /// Verify that SearchMemories without JSON flag returns markdown.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-004: SearchMemories without --json returns markdown")]
    public void SearchMemories_WithoutJsonFlag_ReturnsMarkdown()
    {
        // Arrange - First create file in JSON mode (to ensure it exists)
        MemoryTools.WriteMemory("Markdown Search", "Content with [[markdown-search]].", TestFolder);
        GraphTools.Sync();

        // Switch to Markdown mode for backward compat test
        OutputContext.Format = OutputFormat.Markdown;

        // Act
        var result = MemorySearchTools.SearchMemories(
            query: "markdown",
            mode: SearchMode.Hybrid,
            folder: TestFolder);

        // Assert - Current output contains markdown formatting
        Assert.That(result, Does.Contain("Search"),
            "Without --json flag, SearchMemories should return human-readable output");

        // Verify it's NOT valid JSON
        Assert.Catch<JsonException>(() => JsonDocument.Parse(result),
            "Without --json flag, output should NOT be valid JSON");
    }

    #endregion

    #region TC-CLI-JSON-005: No ANSI escape codes in JSON output

    /// <summary>
    /// TC-CLI-JSON-005: RTM FR-8.2
    /// Verify that JSON output does not contain ANSI escape codes.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-005: No ANSI escape codes in JSON output")]
    public void WriteMemory_JsonOutput_NoAnsiEscapeCodes()
    {
        // Arrange
        var title = "ANSI Test";
        var content = "Content with [[ansi-test]] concept.";

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert - When JSON mode is implemented, output should have no ANSI codes
        // This will FAIL initially until JSON output is implemented
        var doc = JsonDocument.Parse(result);
        var jsonString = doc.RootElement.GetRawText();

        // Check for common ANSI escape sequences
        Assert.That(jsonString, Does.Not.Contain("\u001b["),
            "JSON output should not contain ANSI escape codes");
        Assert.That(jsonString, Does.Not.Contain("\x1b["),
            "JSON output should not contain ANSI escape codes (hex format)");
        Assert.That(jsonString, Does.Not.Match(@"\[[\d;]+m"),
            "JSON output should not contain ANSI color codes");
    }

    /// <summary>
    /// TC-CLI-JSON-005: RTM FR-8.2
    /// Verify that error responses have no ANSI escape codes.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-005: Error responses have no ANSI escape codes")]
    public void ReadMemory_Error_NoAnsiEscapeCodes()
    {
        // Act - Read non-existent file to trigger error
        var result = MemoryTools.ReadMemory("memory://non-existent-ansi-test");

        // Assert - This will FAIL until JSON output is implemented
        var doc = JsonDocument.Parse(result);
        var jsonString = doc.RootElement.GetRawText();

        Assert.That(jsonString, Does.Not.Contain("\u001b["),
            "JSON error output should not contain ANSI escape codes");
        Assert.That(jsonString, Does.Not.Contain("\x1b["),
            "JSON error output should not contain ANSI escape codes");
    }

    #endregion

    #region TC-CLI-JSON-006: JSON is valid UTF-8

    /// <summary>
    /// TC-CLI-JSON-006: RTM FR-8.2
    /// Verify that JSON output is valid UTF-8 encoding.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-006: JSON output is valid UTF-8")]
    public void WriteMemory_JsonOutput_IsValidUtf8()
    {
        // Arrange - Content with various Unicode characters
        var title = "UTF-8 Test";
        var content = "Content with [[unicode]] chars: cafe, math symbols, emoji face.";

        // Act
        var result = MemoryTools.WriteMemory(title, content, TestFolder);

        // Assert - Verify UTF-8 encoding
        // This will FAIL until JSON output is implemented
        var bytes = Encoding.UTF8.GetBytes(result);
        var roundTrip = Encoding.UTF8.GetString(bytes);

        Assert.That(roundTrip, Is.EqualTo(result),
            "JSON output should be valid UTF-8 that round-trips correctly");

        // Verify it parses as valid JSON with UTF-8 content
        var doc = JsonDocument.Parse(result);
        Assert.That(doc.RootElement.ValueKind, Is.Not.EqualTo(JsonValueKind.Undefined),
            "JSON should parse with UTF-8 content");
    }

    /// <summary>
    /// TC-CLI-JSON-006: RTM FR-8.2
    /// Verify that JSON with special characters is valid UTF-8.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-006: JSON with special characters is valid UTF-8")]
    public void SearchMemories_JsonWithSpecialChars_IsValidUtf8()
    {
        // Arrange - Create file with special characters
        var title = "Special Chars Test";
        var content = "Content with [[special-chars]]: quotes \"test\", backslash \\, newline \n, tab \t.";
        MemoryTools.WriteMemory(title, content, TestFolder);
        GraphTools.Sync();

        // Act
        var result = MemorySearchTools.SearchMemories(
            query: "special-chars",
            mode: SearchMode.Hybrid,
            folder: TestFolder);

        // Assert - This will FAIL until JSON output is implemented
        // When implemented, special chars should be properly escaped in JSON
        var doc = JsonDocument.Parse(result);
        Assert.That(doc.RootElement.ValueKind, Is.Not.EqualTo(JsonValueKind.Undefined),
            "JSON with special characters should parse correctly");

        // Verify UTF-8 encoding
        var bytes = Encoding.UTF8.GetBytes(result);
        var roundTrip = Encoding.UTF8.GetString(bytes);
        Assert.That(roundTrip, Is.EqualTo(result),
            "JSON with special chars should round-trip as UTF-8");
    }

    #endregion

    #region Additional Tool Coverage Tests

    /// <summary>
    /// TC-CLI-JSON-001: RTM FR-8.2
    /// Verify that ReadMemory success response includes file content in data.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-002: ReadMemory JSON data includes file content")]
    public void ReadMemory_JsonSuccess_DataIncludesContent()
    {
        // Arrange
        var title = "Content Data Test";
        var originalContent = "File content with [[data-test]] concept.";
        MemoryTools.WriteMemory(title, originalContent, TestFolder);

        // Act
        var result = MemoryTools.ReadMemory($"memory://{TestFolder}/content-data-test");

        // Assert - This will FAIL until JSON output is implemented
        var doc = JsonDocument.Parse(result);
        var data = doc.RootElement.GetProperty("data");

        Assert.That(data.TryGetProperty("content", out var contentProp), Is.True,
            "ReadMemory data should include 'content' field");
        Assert.That(contentProp.GetString(), Does.Contain("[[data-test]]"),
            "Content should preserve WikiLinks");

        Assert.That(data.TryGetProperty("uri", out _), Is.True,
            "ReadMemory data should include 'uri' field");
    }

    /// <summary>
    /// TC-CLI-JSON-002: RTM FR-8.3
    /// Verify that SearchMemories JSON data includes pagination info.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-002: SearchMemories JSON data includes pagination")]
    public void SearchMemories_JsonSuccess_DataIncludesPagination()
    {
        // Arrange - Create multiple files
        for (int i = 0; i < 5; i++)
        {
            MemoryTools.WriteMemory($"Pagination Test {i}", $"Content with [[pagination-test]] number {i}.", TestFolder);
        }
        GraphTools.Sync();

        // Act
        var result = MemorySearchTools.SearchMemories(
            query: "pagination-test",
            mode: SearchMode.Hybrid,
            pageSize: 2,
            page: 1,
            folder: TestFolder);

        // Assert - This will FAIL until JSON output is implemented
        var doc = JsonDocument.Parse(result);
        var data = doc.RootElement.GetProperty("data");

        Assert.That(data.TryGetProperty("results", out _), Is.True,
            "SearchMemories data should include 'results' array");
        Assert.That(data.TryGetProperty("totalCount", out _), Is.True,
            "SearchMemories data should include 'totalCount' for pagination");
        Assert.That(data.TryGetProperty("page", out _), Is.True,
            "SearchMemories data should include current 'page'");
        Assert.That(data.TryGetProperty("pageSize", out _), Is.True,
            "SearchMemories data should include 'pageSize'");
    }

    /// <summary>
    /// TC-CLI-JSON-002: RTM FR-8.3
    /// Verify that SequentialThinking JSON data includes session management info.
    /// </summary>
    [Test]
    [Description("TC-CLI-JSON-002: SequentialThinking JSON data includes session info")]
    public void SequentialThinking_JsonSuccess_DataIncludesSessionInfo()
    {
        // Arrange & Act
        var result = SequentialThinkingTools.SequentialThinking(
            response: "Creating session with [[session-info-test]].",
            nextThoughtNeeded: true,
            thoughtNumber: 0,
            totalThoughts: 3);

        // Assert - This will FAIL until JSON output is implemented
        var doc = JsonDocument.Parse(result);
        var data = doc.RootElement.GetProperty("data");

        Assert.That(data.TryGetProperty("sessionId", out var sessionIdProp), Is.True,
            "SequentialThinking data should include 'sessionId'");
        Assert.That(sessionIdProp.GetString(), Does.StartWith("session-"),
            "Session ID should have expected format");

        Assert.That(data.TryGetProperty("thoughtNumber", out _), Is.True,
            "SequentialThinking data should include 'thoughtNumber'");
        Assert.That(data.TryGetProperty("totalThoughts", out _), Is.True,
            "SequentialThinking data should include 'totalThoughts'");
        Assert.That(data.TryGetProperty("status", out _), Is.True,
            "SequentialThinking data should include 'status'");
    }

    #endregion
}
