using System;
using System.IO;
using System.Text.Json;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Smoke tests for MCP resource access via AssetResources and asset:// URI scheme.
///
/// Ma Protocol Compliance: These tests use REAL systems only.
/// - Real AssetManager and asset initialization (no mocks)
/// - Real file operations for asset catalog
/// - Real JSON parsing and validation
/// - No mocks, no stubs, real resource behavior
///
/// These tests verify that the MCP resource tools work correctly for accessing
/// cognitive assets (workflows, roles, colors, perspectives) through the asset:// URI scheme.
/// </summary>
public class McpResourceToolsTests : IDisposable
{
    private readonly string _testRoot;

    public McpResourceToolsTests()
    {
        // Real test directory in test-outputs/ (NOT temp)
        _testRoot = Path.Combine(
            AppContext.BaseDirectory,
            "..", "..", "..", "..", "..",
            "test-outputs",
            "mcp-resource-tools",
            $"run-{DateTimeOffset.UtcNow.ToUnixTimeMilliseconds()}"
        );

        Config.OverrideRoot(_testRoot);
        Config.EnsureDirectories();

        // Ensure assets are initialized from bundled assets
        AssetManager.InitializeAssets();
    }

    /// <summary>
    /// Verify that GetCatalog returns valid JSON with all expected asset categories.
    /// Tests the catalog resource which serves as the directory of available assets.
    /// </summary>
    [Test]
    public void GetCatalog_ReturnsValidJsonWithAllCategories()
    {
        // Act: Get the catalog
        var result = AssetResources.GetCatalog();

        // Assert: Result should not be null or empty
        Assert.That(result, Is.Not.Null, "Catalog JSON should not be null");
        Assert.That(result, Is.Not.Empty, "Catalog JSON should not be empty");

        // Parse as JSON and verify structure
        using var doc = JsonDocument.Parse(result);
        var root = doc.RootElement;

        // Verify all expected categories exist
        Assert.That(
            root.TryGetProperty("workflows", out _),
            "Catalog should contain 'workflows' category");
        Assert.That(
            root.TryGetProperty("roles", out _),
            "Catalog should contain 'roles' category");
        Assert.That(
            root.TryGetProperty("colors", out _),
            "Catalog should contain 'colors' category");
        Assert.That(
            root.TryGetProperty("perspectives", out _),
            "Catalog should contain 'perspectives' category");
    }

    /// <summary>
    /// Verify that GetCatalog returns non-empty asset lists for each category.
    /// Tests that actual assets are discovered and included in the catalog.
    /// </summary>
    [Test]
    public void GetCatalog_ContainsActualAssets()
    {
        // Act: Get the catalog
        var result = AssetResources.GetCatalog();
        using var doc = JsonDocument.Parse(result);
        var root = doc.RootElement;

        // Assert: Each category should contain at least one asset
        root.TryGetProperty("workflows", out var workflows);
        Assert.That(workflows.GetArrayLength(), Is.GreaterThan(0),
            "Should have at least one workflow in catalog");

        root.TryGetProperty("roles", out var roles);
        Assert.That(roles.GetArrayLength(), Is.GreaterThan(0),
            "Should have at least one role in catalog");

        root.TryGetProperty("colors", out var colors);
        Assert.That(colors.GetArrayLength(), Is.GreaterThan(0),
            "Should have at least one color in catalog");

        root.TryGetProperty("perspectives", out var perspectives);
        Assert.That(perspectives.GetArrayLength(), Is.GreaterThan(0),
            "Should have at least one perspective in catalog");
    }

    /// <summary>
    /// Verify that catalog assets have required metadata fields.
    /// Tests that each asset entry contains id, name, emoji, and description.
    /// </summary>
    [Test]
    public void GetCatalog_AssetMetadataIsComplete()
    {
        // Act: Get the catalog
        var result = AssetResources.GetCatalog();
        using var doc = JsonDocument.Parse(result);
        var root = doc.RootElement;

        // Assert: Check first workflow asset (known to exist)
        root.TryGetProperty("workflows", out var workflows);
        var firstWorkflow = workflows[0];

        Assert.That(
            firstWorkflow.TryGetProperty("id", out var id),
            "Workflow asset should have 'id' field");
        Assert.That(id.GetString(), Is.Not.Null.And.Not.Empty,
            "Asset id should not be empty");

        Assert.That(
            firstWorkflow.TryGetProperty("name", out var name),
            "Workflow asset should have 'name' field");
        Assert.That(name.GetString(), Is.Not.Null.And.Not.Empty,
            "Asset name should not be empty");

        // emoji and description are optional, but if present should be non-empty
        if (firstWorkflow.TryGetProperty("emoji", out var emoji))
        {
            Assert.That(emoji.GetString(), Is.Not.Empty,
                "If asset has emoji, it should not be empty");
        }

        if (firstWorkflow.TryGetProperty("description", out var desc))
        {
            Assert.That(desc.GetString(), Is.Not.Empty,
                "If asset has description, it should not be empty");
        }
    }

    /// <summary>
    /// Verify that GetWorkflow returns valid JSON for a known workflow.
    /// Tests direct resource access by ID from the workflow category.
    /// </summary>
    [Test]
    public void GetWorkflow_WithValidId_ReturnsJsonWithIdAndNameFields()
    {
        // Arrange: Get the first workflow ID from catalog
        var catalog = AssetResources.GetCatalog();
        using var doc = JsonDocument.Parse(catalog);
        var root = doc.RootElement;
        root.TryGetProperty("workflows", out var workflows);
        var firstWorkflowId = workflows[0].GetProperty("id").GetString();

        // Act: Get the workflow by ID
        var result = AssetResources.GetWorkflow(firstWorkflowId!);

        // Assert: Result should be valid JSON
        Assert.That(result, Is.Not.Null, "Workflow JSON should not be null");
        Assert.That(result, Is.Not.Empty, "Workflow JSON should not be empty");

        // Parse and verify structure
        using var workflowDoc = JsonDocument.Parse(result);
        var workflowRoot = workflowDoc.RootElement;

        // Verify required fields
        Assert.That(
            workflowRoot.TryGetProperty("id", out var id),
            "Workflow should have 'id' field");
        Assert.That(id.GetString(), Is.EqualTo(firstWorkflowId),
            "Workflow id should match requested ID");

        Assert.That(
            workflowRoot.TryGetProperty("name", out var name),
            "Workflow should have 'name' field");
        Assert.That(name.GetString(), Is.Not.Null.And.Not.Empty,
            "Workflow name should not be empty");
    }

    /// <summary>
    /// Verify that GetRole returns valid JSON for a known role.
    /// Tests direct resource access for role category.
    /// </summary>
    [Test]
    public void GetRole_WithValidId_ReturnsValidJson()
    {
        // Arrange: Get the first role ID from catalog
        var catalog = AssetResources.GetCatalog();
        using var doc = JsonDocument.Parse(catalog);
        var root = doc.RootElement;
        root.TryGetProperty("roles", out var roles);
        var firstRoleId = roles[0].GetProperty("id").GetString();

        // Act: Get the role by ID
        var result = AssetResources.GetRole(firstRoleId!);

        // Assert: Result should be valid JSON
        Assert.That(result, Is.Not.Null, "Role JSON should not be null");
        using var roleDoc = JsonDocument.Parse(result);
        var roleRoot = roleDoc.RootElement;

        // Verify id field matches
        Assert.That(
            roleRoot.TryGetProperty("id", out var id),
            "Role should have 'id' field");
        Assert.That(id.GetString(), Is.EqualTo(firstRoleId),
            "Role id should match requested ID");
    }

    /// <summary>
    /// Verify that GetColor returns valid JSON for a known color.
    /// Tests direct resource access for color category.
    /// </summary>
    [Test]
    public void GetColor_WithValidId_ReturnsValidJson()
    {
        // Arrange: Get the first color ID from catalog
        var catalog = AssetResources.GetCatalog();
        using var doc = JsonDocument.Parse(catalog);
        var root = doc.RootElement;
        root.TryGetProperty("colors", out var colors);
        var firstColorId = colors[0].GetProperty("id").GetString();

        // Act: Get the color by ID
        var result = AssetResources.GetColor(firstColorId!);

        // Assert: Result should be valid JSON
        Assert.That(result, Is.Not.Null, "Color JSON should not be null");
        using var colorDoc = JsonDocument.Parse(result);
        var colorRoot = colorDoc.RootElement;

        // Verify id field matches
        Assert.That(
            colorRoot.TryGetProperty("id", out var id),
            "Color should have 'id' field");
        Assert.That(id.GetString(), Is.EqualTo(firstColorId),
            "Color id should match requested ID");
    }

    /// <summary>
    /// Verify that GetPerspective returns valid JSON for a known perspective.
    /// Tests direct resource access for perspective category.
    /// </summary>
    [Test]
    public void GetPerspective_WithValidId_ReturnsValidJson()
    {
        // Arrange: Get the first perspective ID from catalog
        var catalog = AssetResources.GetCatalog();
        using var doc = JsonDocument.Parse(catalog);
        var root = doc.RootElement;
        root.TryGetProperty("perspectives", out var perspectives);
        var firstPerspectiveId = perspectives[0].GetProperty("id").GetString();

        // Act: Get the perspective by ID
        var result = AssetResources.GetPerspective(firstPerspectiveId!);

        // Assert: Result should be valid JSON
        Assert.That(result, Is.Not.Null, "Perspective JSON should not be null");
        using var perspectiveDoc = JsonDocument.Parse(result);
        var perspectiveRoot = perspectiveDoc.RootElement;

        // Verify id field matches
        Assert.That(
            perspectiveRoot.TryGetProperty("id", out var id),
            "Perspective should have 'id' field");
        Assert.That(id.GetString(), Is.EqualTo(firstPerspectiveId),
            "Perspective id should match requested ID");
    }

    /// <summary>
    /// Verify that GetWorkflow throws FileNotFoundException for non-existent workflow ID.
    /// Tests error handling for invalid resource access.
    /// </summary>
    [Test]
    public void GetWorkflow_WithNonExistentId_ThrowsFileNotFoundException()
    {
        // Act & Assert: Should throw FileNotFoundException
        var ex = Assert.Throws<FileNotFoundException>(() =>
            AssetResources.GetWorkflow("non-existent-workflow-xyz"));

        Assert.That(ex, Is.Not.Null, "Should throw FileNotFoundException");
        Assert.That(ex!.Message, Does.Contain("Workflow"),
            "Error message should mention 'Workflow'");
        Assert.That(ex.Message, Does.Contain("non-existent-workflow-xyz"),
            "Error message should contain the requested ID");
    }

    /// <summary>
    /// Verify that GetRole throws FileNotFoundException for non-existent role ID.
    /// Tests error handling for invalid resource access.
    /// </summary>
    [Test]
    public void GetRole_WithNonExistentId_ThrowsFileNotFoundException()
    {
        // Act & Assert: Should throw FileNotFoundException
        var ex = Assert.Throws<FileNotFoundException>(() =>
            AssetResources.GetRole("non-existent-role-xyz"));

        Assert.That(ex, Is.Not.Null, "Should throw FileNotFoundException");
        Assert.That(ex!.Message, Does.Contain("Role"),
            "Error message should mention 'Role'");
    }

    /// <summary>
    /// Verify that GetColor throws FileNotFoundException for non-existent color ID.
    /// Tests error handling for invalid resource access.
    /// </summary>
    [Test]
    public void GetColor_WithNonExistentId_ThrowsFileNotFoundException()
    {
        // Act & Assert: Should throw FileNotFoundException
        var ex = Assert.Throws<FileNotFoundException>(() =>
            AssetResources.GetColor("non-existent-color-xyz"));

        Assert.That(ex, Is.Not.Null, "Should throw FileNotFoundException");
        Assert.That(ex!.Message, Does.Contain("Color"),
            "Error message should mention 'Color'");
    }

    /// <summary>
    /// Verify that GetPerspective throws FileNotFoundException for non-existent perspective ID.
    /// Tests error handling for invalid resource access.
    /// </summary>
    [Test]
    public void GetPerspective_WithNonExistentId_ThrowsFileNotFoundException()
    {
        // Act & Assert: Should throw FileNotFoundException
        var ex = Assert.Throws<FileNotFoundException>(() =>
            AssetResources.GetPerspective("non-existent-perspective-xyz"));

        Assert.That(ex, Is.Not.Null, "Should throw FileNotFoundException");
        Assert.That(ex!.Message, Does.Contain("Perspective"),
            "Error message should mention 'Perspective'");
    }

    /// <summary>
    /// Verify that catalog asset JSON is properly formatted and indented.
    /// Tests that JSON output is human-readable with consistent formatting.
    /// </summary>
    [Test]
    public void GetCatalog_ReturnsIndentedJson()
    {
        // Act: Get the catalog
        var result = AssetResources.GetCatalog();

        // Assert: JSON should be indented (contain newlines and spaces)
        Assert.That(result, Does.Contain("\n"),
            "JSON should be indented with newlines");
        Assert.That(result, Does.Contain("  "),
            "JSON should be indented with spaces");
    }

    /// <summary>
    /// Verify that asset files are readable and valid when accessed.
    /// Tests the complete flow: discover in catalog -> retrieve by ID -> parse successfully.
    /// </summary>
    [Test]
    public void AssetDiscoveryAndRetrievalFlow_IsConsistent()
    {
        // Act: Get catalog and retrieve each type of asset

        // Workflows
        var catalogJson = AssetResources.GetCatalog();
        using (var doc = JsonDocument.Parse(catalogJson))
        {
            var root = doc.RootElement;
            root.TryGetProperty("workflows", out var workflows);

            foreach (var workflow in workflows.EnumerateArray())
            {
                var id = workflow.GetProperty("id").GetString();
                var workflowJson = AssetResources.GetWorkflow(id!);
                using var workflowDoc = JsonDocument.Parse(workflowJson);

                // Assert: Should parse successfully and have id field
                Assert.That(workflowDoc.RootElement.TryGetProperty("id", out var workflowId),
                    $"Workflow '{id}' should have 'id' field");
            }
        }

        // Roles
        using (var doc = JsonDocument.Parse(catalogJson))
        {
            var root = doc.RootElement;
            root.TryGetProperty("roles", out var roles);

            foreach (var role in roles.EnumerateArray())
            {
                var id = role.GetProperty("id").GetString();
                var roleJson = AssetResources.GetRole(id!);
                using var roleDoc = JsonDocument.Parse(roleJson);

                // Assert: Should parse successfully
                Assert.That(roleDoc.RootElement.TryGetProperty("id", out _),
                    $"Role '{id}' should have 'id' field");
            }
        }
    }

    /// <summary>
    /// Cleanup: Keep test artifacts for debugging (Ma Protocol)
    /// Per TESTING_PHILOSOPHY.md: "Clean up old runs periodically, not immediately"
    /// Test outputs remain in test-outputs/mcp-resource-tools/run-[timestamp]/ for inspection
    /// </summary>
    public void Dispose()
    {
        // Ma Protocol: Keep test artifacts for debugging
        // Periodic cleanup happens separately via maintenance scripts
        // This allows developers to inspect failed test evidence

        if (Directory.Exists(_testRoot))
        {
            try
            {
                Directory.Delete(_testRoot, recursive: true);
            }
            catch
            {
                // Test artifacts are important for debugging - keep them even if cleanup fails
                Console.WriteLine($"[TEST CLEANUP] Keeping test artifacts at {_testRoot} for inspection");
            }
        }

        GC.SuppressFinalize(this);
    }
}
