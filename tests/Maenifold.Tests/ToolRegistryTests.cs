// T-COV-001.2: RTM FR-17.4
using System.Reflection;
using System.Text.Json;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Tests for ToolRegistry — tool registration, case-insensitive lookup, dispatch, and unknown-tool error.
///
/// Ma Protocol Compliance: These tests use REAL tool dispatch only.
/// - No mocks, no stubs, no test doubles
/// - Real Config, GraphDatabase, AssetManager
/// - Behavioral correctness (does the right tool get called?) not implementation details
/// </summary>
[TestFixture]
public class ToolRegistryTests
{
    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();
        AssetManager.InitializeAssets();

        // ToolRegistry uses static state. Reset it so each test starts clean.
        var initializedField = typeof(ToolRegistry).GetField(
            "_initialized",
            BindingFlags.Static | BindingFlags.NonPublic
        );
        initializedField!.SetValue(null, false);

        var byNameField = typeof(ToolRegistry).GetField(
            "_byName",
            BindingFlags.Static | BindingFlags.NonPublic
        );
        var byName = (Dictionary<string, ToolDescriptor>)byNameField!.GetValue(null)!;
        byName.Clear();
    }

    // ---------------------------------------------------------------------------
    // 1. Initialize — known tools are registered after Initialize()
    // ---------------------------------------------------------------------------

    [Test]
    public void Initialize_RegistersKnownTools()
    {
        // Act
        ToolRegistry.Initialize();

        // Inspect the internal dictionary directly — avoids invoking tools that require
        // mandatory payload properties (e.g. WriteMemory requires "title").
        var byNameField = typeof(ToolRegistry).GetField(
            "_byName",
            BindingFlags.Static | BindingFlags.NonPublic
        );
        var byName = (Dictionary<string, ToolDescriptor>)byNameField!.GetValue(null)!;

        // Assert: a representative set of canonical tool names must be registered
        string[] expectedTools = [
            "WriteMemory", "ReadMemory", "EditMemory", "DeleteMemory", "MoveMemory",
            "SearchMemories", "BuildContext", "Sync", "SequentialThinking", "Workflow",
            "RecentActivity", "MemoryStatus", "GetConfig", "GetHelp", "ListMemories",
            "UpdateAssets", "RepairConcepts", "AnalyzeConceptCorruption",
            "FindSimilarConcepts", "StartWatcher", "StopWatcher", "RunFullBenchmark",
            "Adopt", "AssumptionLedger", "ListAssets", "ReadMcpResource",
            "Visualize", "ExtractConceptsFromFile"
        ];

        foreach (var toolName in expectedTools)
        {
            Assert.That(
                byName.ContainsKey(toolName),
                Is.True,
                $"Tool '{toolName}' should be registered after Initialize()"
            );
        }
    }

    // ---------------------------------------------------------------------------
    // 2. TryInvoke with valid tool name — returns true
    // ---------------------------------------------------------------------------

    [Test]
    public void TryInvoke_WithValidToolName_ReturnsTrue()
    {
        // Act
        var payload = JsonDocument.Parse("{}").RootElement;
        var found = ToolRegistry.TryInvoke("GetConfig", payload, out var result);

        // Assert
        Assert.That(found, Is.True, "TryInvoke should return true for a registered tool");
        Assert.That(result, Is.Not.Null, "Result should not be null for a registered tool");
    }

    // ---------------------------------------------------------------------------
    // 3. TryInvoke with alias — dispatches same as canonical name
    // ---------------------------------------------------------------------------

    [Test]
    public void TryInvoke_WithLowercaseAlias_DispatchesSameAsCanonical()
    {
        // WriteMemory registers "writememory" as its lowercase alias
        var canonicalPayload = JsonDocument.Parse("""{"title":"alias-test","content":"[[test]] content"}""").RootElement;
        var aliasPayload = JsonDocument.Parse("""{"title":"alias-test","content":"[[test]] content"}""").RootElement;

        var canonicalFound = ToolRegistry.TryInvoke("WriteMemory", canonicalPayload, out var canonicalResult);
        var aliasFound = ToolRegistry.TryInvoke("writememory", aliasPayload, out var aliasResult);

        Assert.That(canonicalFound, Is.True, "Canonical name 'WriteMemory' should be found");
        Assert.That(aliasFound, Is.True, "Lowercase alias 'writememory' should be found");

        // Both must produce a non-null result (actual output content may differ due to timestamp, but both succeed)
        Assert.That(canonicalResult, Is.Not.Null, "Canonical invocation result should not be null");
        Assert.That(aliasResult, Is.Not.Null, "Alias invocation result should not be null");
    }

    // ---------------------------------------------------------------------------
    // 4. Case-insensitive lookup — various casings all resolve
    // ---------------------------------------------------------------------------

    [Test]
    [TestCase("WRITEMEMORY")]
    [TestCase("WriteMemory")]
    [TestCase("writememory")]
    [TestCase("WRITEMEMORY")]
    [TestCase("wRiTeMemOrY")]
    public void TryInvoke_CaseInsensitiveLookup_AlwaysResolves(string name)
    {
        // Each variant should find the WriteMemory tool
        var payload = JsonDocument.Parse("""{"title":"casing-test","content":"[[casing]] lookup test"}""").RootElement;
        var found = ToolRegistry.TryInvoke(name, payload, out var result);

        Assert.That(found, Is.True, $"TryInvoke('{name}') should resolve regardless of casing");
        Assert.That(result, Is.Not.Null, $"Result for '{name}' should not be null");
    }

    // ---------------------------------------------------------------------------
    // 5. TryInvoke with unknown tool — returns false, result is null
    // ---------------------------------------------------------------------------

    [Test]
    public void TryInvoke_WithUnknownTool_ReturnsFalse()
    {
        var payload = JsonDocument.Parse("{}").RootElement;
        var found = ToolRegistry.TryInvoke("NonExistentTool_XYZ", payload, out var result);

        Assert.That(found, Is.False, "TryInvoke should return false for an unknown tool name");
        Assert.That(result, Is.Null, "Result should be null when tool is not found");
    }

    // ---------------------------------------------------------------------------
    // 6. TryInvoke with empty or null name — returns false
    // ---------------------------------------------------------------------------

    [Test]
    [TestCase("")]
    [TestCase("   ")]
    public void TryInvoke_WithEmptyOrWhitespaceName_ReturnsFalse(string name)
    {
        var payload = JsonDocument.Parse("{}").RootElement;
        var found = ToolRegistry.TryInvoke(name, payload, out var result);

        Assert.That(found, Is.False, $"TryInvoke('{name}') should return false for empty/whitespace name");
        Assert.That(result, Is.Null, "Result should be null for empty/whitespace name");
    }

    // ---------------------------------------------------------------------------
    // 7. Dispatch to at least 5 tools — each returns non-null result
    // ---------------------------------------------------------------------------

    [Test]
    public void TryInvoke_ReadMemory_WithMissingFile_ReturnsNonNull()
    {
        // ReadMemory returns an error string (not null) when the file does not exist
        var payload = JsonDocument.Parse("""{"identifier":"memory://tool-registry-test-does-not-exist"}""").RootElement;
        var found = ToolRegistry.TryInvoke("ReadMemory", payload, out var result);

        Assert.That(found, Is.True, "ReadMemory should be registered");
        Assert.That(result, Is.Not.Null, "ReadMemory result should not be null");
        Assert.That(result, Does.Contain("not found").IgnoreCase.Or.Contain("error").IgnoreCase, "ReadMemory should indicate file not found");
    }

    [Test]
    public void TryInvoke_GetConfig_ReturnsNonNullResult()
    {
        var payload = JsonDocument.Parse("{}").RootElement;
        var found = ToolRegistry.TryInvoke("GetConfig", payload, out var result);

        Assert.That(found, Is.True, "GetConfig should be registered");
        Assert.That(result, Is.Not.Null, "GetConfig result should not be null");
        Assert.That(result, Does.Contain("root"), "GetConfig should include root path info");
    }

    [Test]
    public void TryInvoke_MemoryStatus_ReturnsNonNullResult()
    {
        var payload = JsonDocument.Parse("{}").RootElement;
        var found = ToolRegistry.TryInvoke("MemoryStatus", payload, out var result);

        Assert.That(found, Is.True, "MemoryStatus should be registered");
        Assert.That(result, Is.Not.Null, "MemoryStatus result should not be null");
        Assert.That(result, Does.Contain("memor").IgnoreCase, "MemoryStatus should include memory information");
    }

    [Test]
    public void TryInvoke_GetHelp_WithToolName_ReturnsNonNullResult()
    {
        var payload = JsonDocument.Parse("""{"toolName":"GetHelp"}""").RootElement;
        var found = ToolRegistry.TryInvoke("GetHelp", payload, out var result);

        Assert.That(found, Is.True, "GetHelp should be registered");
        Assert.That(result, Is.Not.Null, "GetHelp result should not be null");
        Assert.That(result, Does.Contain("GetHelp").IgnoreCase, "GetHelp should return help about the requested tool");
    }

    [Test]
    public void TryInvoke_ListMemories_ReturnsNonNullResult()
    {
        var payload = JsonDocument.Parse("{}").RootElement;
        var found = ToolRegistry.TryInvoke("ListMemories", payload, out var result);

        Assert.That(found, Is.True, "ListMemories should be registered");
        Assert.That(result, Is.Not.Null, "ListMemories result should not be null");
        Assert.That(result, Does.Contain("Directory").IgnoreCase.Or.Contain("Files").IgnoreCase, "ListMemories should return directory or file listing");
    }

    // ---------------------------------------------------------------------------
    // 8. Initialize is idempotent — calling it twice does not duplicate registrations
    // ---------------------------------------------------------------------------

    [Test]
    public void Initialize_CalledTwice_DoesNotDuplicateRegistrations()
    {
        ToolRegistry.Initialize();
        ToolRegistry.Initialize(); // second call is a no-op

        // The registry should still return exactly one result for each known name
        var payload = JsonDocument.Parse("{}").RootElement;
        var found = ToolRegistry.TryInvoke("GetConfig", payload, out var result);

        Assert.That(found, Is.True, "GetConfig should still resolve after double Initialize");
        Assert.That(result, Is.Not.Null, "Result should not be null after double Initialize");
    }

    // ---------------------------------------------------------------------------
    // 9. TryInvoke auto-initializes — works without explicit Initialize() call
    // ---------------------------------------------------------------------------

    [Test]
    public void TryInvoke_WithoutExplicitInitialize_AutoInitializesAndReturnsTrue()
    {
        // SetUp has already cleared _initialized = false and _byName, simulating a fresh registry.
        // TryInvoke must auto-call Initialize() before lookup.
        var payload = JsonDocument.Parse("{}").RootElement;
        var found = ToolRegistry.TryInvoke("MemoryStatus", payload, out var result);

        Assert.That(found, Is.True, "TryInvoke should auto-initialize and find the tool");
        Assert.That(result, Is.Not.Null, "Auto-initialized result should not be null");
    }
}
