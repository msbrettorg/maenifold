using System;
using System.IO;
using System.Text.Json;
using System.Threading.Tasks;
using Maenifold.Tools;
using Maenifold.Utils;
using ModelContextProtocol;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Tests for AdoptTools functionality (role/color/perspective loading).
///
/// Ma Protocol Compliance: These tests use REAL systems only.
/// - Real AssetManager and asset initialization (no mocks)
/// - Real file operations for asset discovery
/// - Real JSON parsing and validation
/// - No mocks, no stubs, real adoption behavior
///
/// These tests verify that the Adopt tool correctly loads and returns
/// cognitive assets (roles, colors, perspectives) with proper instructions.
/// </summary>
[TestFixture]
public class AdoptToolsTests
{
    [OneTimeSetUp]
    public void OneTimeSetup()
    {
        // Ensure assets are initialized from bundled assets
        // This runs once for all tests in this fixture
        Config.EnsureDirectories();
        AssetManager.InitializeAssets();
    }

    #region Adopt - Valid Assets Tests

    /// <summary>
    /// Verify that Adopt with valid role returns role content and instructions.
    /// Tests that role adoption includes JSON configuration and behavioral instructions.
    /// </summary>
    [Test]
    public async Task Adopt_WithValidRole_ReturnsRoleContentAndInstructions()
    {
        // Arrange: Get a valid role ID from the catalog
        var catalog = AssetResources.GetCatalog();
        using var doc = JsonDocument.Parse(catalog);
        var root = doc.RootElement;
        root.TryGetProperty("roles", out var roles);

        // Skip test if no roles available
        if (roles.GetArrayLength() == 0)
        {
            Assert.Ignore("No roles available in asset catalog");
            return;
        }

        var firstRoleId = roles[0].GetProperty("id").GetString();

        // Act
        var result = await AdoptTools.Adopt("role", firstRoleId!);

        // Assert: Result should contain adoption instructions
        Assert.That(result, Is.Not.Null, "Result should not be null");
        Assert.That(result, Is.Not.Empty, "Result should not be empty");
        Assert.That(result, Does.Contain("adopting the role"),
            "Result should contain adoption instructions");
        Assert.That(result, Does.Contain(firstRoleId!),
            "Result should reference the role identifier");
        Assert.That(result, Does.Contain("JSON Configuration"),
            "Result should indicate JSON configuration follows");

        // Result should contain valid JSON configuration
        Assert.That(result, Does.Contain("{").And.Contains("}"),
            "Result should contain JSON object");
    }

    /// <summary>
    /// Verify that Adopt with valid color returns color content and instructions.
    /// Tests that color adoption includes JSON configuration and behavioral instructions.
    /// </summary>
    [Test]
    public async Task Adopt_WithValidColor_ReturnsColorContentAndInstructions()
    {
        // Arrange: Get a valid color ID from the catalog
        var catalog = AssetResources.GetCatalog();
        using var doc = JsonDocument.Parse(catalog);
        var root = doc.RootElement;
        root.TryGetProperty("colors", out var colors);

        // Skip test if no colors available
        if (colors.GetArrayLength() == 0)
        {
            Assert.Ignore("No colors available in asset catalog");
            return;
        }

        var firstColorId = colors[0].GetProperty("id").GetString();

        // Act
        var result = await AdoptTools.Adopt("color", firstColorId!);

        // Assert: Result should contain adoption instructions
        Assert.That(result, Is.Not.Null, "Result should not be null");
        Assert.That(result, Is.Not.Empty, "Result should not be empty");
        Assert.That(result, Does.Contain("adopting the color"),
            "Result should contain adoption instructions");
        Assert.That(result, Does.Contain(firstColorId!),
            "Result should reference the color identifier");
        Assert.That(result, Does.Contain("JSON Configuration"),
            "Result should indicate JSON configuration follows");

        // Result should contain valid JSON configuration
        Assert.That(result, Does.Contain("{").And.Contains("}"),
            "Result should contain JSON object");
    }

    /// <summary>
    /// Verify that Adopt with valid perspective returns perspective content and instructions.
    /// Tests that perspective adoption includes JSON configuration and behavioral instructions.
    /// </summary>
    [Test]
    public async Task Adopt_WithValidPerspective_ReturnsPerspectiveContentAndInstructions()
    {
        // Arrange: Get a valid perspective ID from the catalog
        var catalog = AssetResources.GetCatalog();
        using var doc = JsonDocument.Parse(catalog);
        var root = doc.RootElement;
        root.TryGetProperty("perspectives", out var perspectives);

        // Skip test if no perspectives available
        if (perspectives.GetArrayLength() == 0)
        {
            Assert.Ignore("No perspectives available in asset catalog");
            return;
        }

        var firstPerspectiveId = perspectives[0].GetProperty("id").GetString();

        // Act
        var result = await AdoptTools.Adopt("perspective", firstPerspectiveId!);

        // Assert: Result should contain adoption instructions
        Assert.That(result, Is.Not.Null, "Result should not be null");
        Assert.That(result, Is.Not.Empty, "Result should not be empty");
        Assert.That(result, Does.Contain("adopting the perspective"),
            "Result should contain adoption instructions");
        Assert.That(result, Does.Contain(firstPerspectiveId!),
            "Result should reference the perspective identifier");
        Assert.That(result, Does.Contain("JSON Configuration"),
            "Result should indicate JSON configuration follows");

        // Result should contain valid JSON configuration
        Assert.That(result, Does.Contain("{").And.Contains("}"),
            "Result should contain JSON object");
    }

    #endregion

    #region Adopt - Invalid Type Tests

    /// <summary>
    /// Verify that Adopt with invalid type throws McpException.
    /// Tests error handling for unsupported asset types.
    /// </summary>
    [Test]
    public void Adopt_WithInvalidType_ThrowsArgumentException()
    {
        // Act & Assert: Should throw McpException
        var ex = Assert.ThrowsAsync<McpException>(async () =>
            await AdoptTools.Adopt("invalid-type", "some-identifier"));

        Assert.That(ex, Is.Not.Null, "Should throw McpException");
        Assert.That(ex!.Message, Does.Contain("Invalid type"),
            "Error message should indicate invalid type");
        Assert.That(ex.Message, Does.Contain("role").And.Contains("color").And.Contains("perspective"),
            "Error message should list valid types");
    }

    /// <summary>
    /// Verify that Adopt is case-insensitive for type parameter.
    /// Tests that type normalization works correctly.
    /// </summary>
    [Test]
    public async Task Adopt_TypeIsCaseInsensitive()
    {
        // Arrange: Get a valid role ID
        var catalog = AssetResources.GetCatalog();
        using var doc = JsonDocument.Parse(catalog);
        var root = doc.RootElement;
        root.TryGetProperty("roles", out var roles);

        if (roles.GetArrayLength() == 0)
        {
            Assert.Ignore("No roles available in asset catalog");
            return;
        }

        var firstRoleId = roles[0].GetProperty("id").GetString();

        // Act: Try different case variations
        var lowerResult = await AdoptTools.Adopt("role", firstRoleId!);
        var upperResult = await AdoptTools.Adopt("ROLE", firstRoleId!);
        var mixedResult = await AdoptTools.Adopt("Role", firstRoleId!);

        // Assert: All should succeed and contain adoption instructions (case preserved in output)
        Assert.That(lowerResult, Does.Contain("adopting the role"),
            "Lowercase 'role' should work");
        Assert.That(upperResult, Does.Contain("adopting the ROLE"),
            "Uppercase 'ROLE' should work");
        Assert.That(mixedResult, Does.Contain("adopting the Role"),
            "Mixed case 'Role' should work");
    }

    #endregion

    #region Adopt - Invalid Identifier Tests

    /// <summary>
    /// Verify that Adopt with non-existent identifier throws FileNotFoundException.
    /// Tests error handling for invalid asset identifiers.
    /// </summary>
    [Test]
    public void Adopt_WithNonExistentIdentifier_ThrowsFileNotFoundException()
    {
        // Act & Assert: Should throw FileNotFoundException
        var ex = Assert.ThrowsAsync<FileNotFoundException>(async () =>
            await AdoptTools.Adopt("role", "non-existent-asset-xyz"));

        Assert.That(ex, Is.Not.Null, "Should throw FileNotFoundException");
        Assert.That(ex!.Message, Does.Contain("Asset not found"),
            "Error message should indicate asset not found");
        Assert.That(ex.Message, Does.Contain("role/non-existent-asset-xyz"),
            "Error message should include type and identifier");
    }

    /// <summary>
    /// Verify that Adopt with empty identifier throws FileNotFoundException.
    /// Tests error handling for empty identifier parameter.
    /// </summary>
    [Test]
    public void Adopt_WithEmptyIdentifier_ThrowsFileNotFoundException()
    {
        // Act & Assert: Should throw FileNotFoundException
        var ex = Assert.ThrowsAsync<FileNotFoundException>(async () =>
            await AdoptTools.Adopt("role", string.Empty));

        Assert.That(ex, Is.Not.Null, "Should throw FileNotFoundException");
    }

    #endregion

    #region Adopt - Learn Mode Tests

    /// <summary>
    /// Verify that Adopt with learn=true returns help documentation.
    /// Tests that help mode returns documentation instead of adopting asset.
    /// </summary>
    [Test]
    public async Task Adopt_WithLearnTrue_ReturnsHelpDocumentation()
    {
        // Act: Call with learn=true (identifier doesn't matter for help mode)
        var result = await AdoptTools.Adopt("role", "any-identifier", learn: true);

        // Assert: Should return help documentation
        Assert.That(result, Is.Not.Null, "Help result should not be null");
        Assert.That(result, Is.Not.Empty, "Help result should not be empty");

        // Help docs should either contain actual documentation or error message
        var isHelpDoc = result.Contains("Adopt") || result.Contains("role") || result.Contains("perspective");
        var isErrorMessage = result.Contains("ERROR") && result.Contains("Help file not found");

        Assert.That(isHelpDoc || isErrorMessage, Is.True,
            "Result should be either help documentation or error message about missing help file");

        // Should NOT contain adoption instructions when in learn mode
        Assert.That(result, Does.Not.Contain("adopting the"),
            "Learn mode should not include adoption instructions");
    }

    #endregion

    #region Adopt - JSON Content Validation Tests

    /// <summary>
    /// Verify that Adopt returns valid parseable JSON in the configuration section.
    /// Tests that the JSON embedded in the adoption instructions is well-formed.
    /// </summary>
    [Test]
    public async Task Adopt_ReturnsValidParseableJson()
    {
        // Arrange: Get a valid workflow (workflows are guaranteed to have valid JSON)
        var catalog = AssetResources.GetCatalog();
        using var catalogDoc = JsonDocument.Parse(catalog);
        var root = catalogDoc.RootElement;
        root.TryGetProperty("roles", out var roles);

        if (roles.GetArrayLength() == 0)
        {
            Assert.Ignore("No roles available in asset catalog");
            return;
        }

        var firstRoleId = roles[0].GetProperty("id").GetString();

        // Act
        var result = await AdoptTools.Adopt("role", firstRoleId!);

        // Assert: Extract and parse the JSON portion
        var jsonStart = result.IndexOf('{');
        var jsonEnd = result.LastIndexOf('}');

        Assert.That(jsonStart, Is.GreaterThanOrEqualTo(0),
            "Result should contain opening brace");
        Assert.That(jsonEnd, Is.GreaterThan(jsonStart),
            "Result should contain closing brace after opening brace");

        var jsonContent = result.Substring(jsonStart, jsonEnd - jsonStart + 1);

        // Should be valid JSON (will throw if not)
        Assert.DoesNotThrow(() =>
        {
            using var jsonDoc = JsonDocument.Parse(jsonContent);
            var jsonRoot = jsonDoc.RootElement;

            // Should have 'id' field
            Assert.That(jsonRoot.TryGetProperty("id", out var id),
                "JSON should contain 'id' field");
            Assert.That(id.GetString(), Is.EqualTo(firstRoleId),
                "JSON id should match requested role");
        }, "Embedded JSON should be valid and parseable");
    }

    /// <summary>
    /// Verify that Adopt instructions include critical behavioral directives.
    /// Tests that adoption response contains all necessary instructions for LLM behavior.
    /// </summary>
    [Test]
    public async Task Adopt_IncludesCriticalInstructions()
    {
        // Arrange: Get a valid color
        var catalog = AssetResources.GetCatalog();
        using var doc = JsonDocument.Parse(catalog);
        var root = doc.RootElement;
        root.TryGetProperty("colors", out var colors);

        if (colors.GetArrayLength() == 0)
        {
            Assert.Ignore("No colors available in asset catalog");
            return;
        }

        var firstColorId = colors[0].GetProperty("id").GetString();

        // Act
        var result = await AdoptTools.Adopt("color", firstColorId!);

        // Assert: Should include critical behavioral directives
        Assert.That(result, Does.Contain("CRITICAL INSTRUCTIONS"),
            "Result should include critical instructions header");
        Assert.That(result, Does.Contain("parse the JSON"),
            "Instructions should mention parsing JSON");
        Assert.That(result, Does.Contain("Incorporate").Or.Contains("incorporate"),
            "Instructions should mention incorporating attributes");
        Assert.That(result, Does.Contain("Acknowledge").Or.Contains("acknowledge"),
            "Instructions should request acknowledgment");
    }

    #endregion

    #region Adopt - Asset Discovery Tests

    /// <summary>
    /// Verify that Adopt can find assets by ID even when filename differs.
    /// Tests the ID-based lookup mechanism for perspectives with mismatched filenames.
    /// </summary>
    [Test]
    public async Task Adopt_FindsAssetByIdRegardlessOfFilename()
    {
        // Arrange: Perspectives often have filename != id, so use those for testing
        var catalog = AssetResources.GetCatalog();
        using var doc = JsonDocument.Parse(catalog);
        var root = doc.RootElement;
        root.TryGetProperty("perspectives", out var perspectives);

        if (perspectives.GetArrayLength() == 0)
        {
            Assert.Ignore("No perspectives available in asset catalog");
            return;
        }

        // Find a perspective and use its ID
        var firstPerspectiveId = perspectives[0].GetProperty("id").GetString();

        // Act: Should find by ID even if filename differs
        var result = await AdoptTools.Adopt("perspective", firstPerspectiveId!);

        // Assert: Should successfully retrieve the perspective
        Assert.That(result, Is.Not.Null, "Should find perspective by ID");
        Assert.That(result, Does.Contain("adopting the perspective"),
            "Should contain adoption instructions");
        Assert.That(result, Does.Contain(firstPerspectiveId!),
            "Should reference the correct perspective ID");
    }

    /// <summary>
    /// Verify that Adopt handles multiple assets of the same type.
    /// Tests that the tool can distinguish and load different assets within a category.
    /// </summary>
    [Test]
    public async Task Adopt_DistinguishesBetweenDifferentAssetsOfSameType()
    {
        // Arrange: Get two different roles
        var catalog = AssetResources.GetCatalog();
        using var doc = JsonDocument.Parse(catalog);
        var root = doc.RootElement;
        root.TryGetProperty("roles", out var roles);

        if (roles.GetArrayLength() < 2)
        {
            Assert.Ignore("Need at least 2 roles to test distinction");
            return;
        }

        var firstRoleId = roles[0].GetProperty("id").GetString();
        var secondRoleId = roles[1].GetProperty("id").GetString();

        // Act
        var firstResult = await AdoptTools.Adopt("role", firstRoleId!);
        var secondResult = await AdoptTools.Adopt("role", secondRoleId!);

        // Assert: Results should be different and reference correct IDs
        Assert.That(firstResult, Does.Contain(firstRoleId!),
            "First result should reference first role ID");
        Assert.That(secondResult, Does.Contain(secondRoleId!),
            "Second result should reference second role ID");

        Assert.That(firstResult, Is.Not.EqualTo(secondResult),
            "Different roles should return different results");
    }

    #endregion
}
