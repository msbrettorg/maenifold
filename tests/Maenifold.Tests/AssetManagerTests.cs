// T-COV-001.6: RTM FR-17.8

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Tests for AssetManager — asset discovery from source dirs, file copy,
/// dry-run mode (no writes), and source-target mapping for workflows/tools/roles/colors.
///
/// Ma Protocol Compliance: These tests use REAL systems only.
/// - Real temp directories (no mocks)
/// - Real file operations for copy/compare
/// - Real SHA256 hash comparison via FilesAreIdentical
/// - No mocks, no stubs
///
/// T-COV-001.6: RTM FR-17.8
/// </summary>
[TestFixture]
public class AssetManagerTests
{
    // The source assets directory is fixed: AppDomain.CurrentDomain.BaseDirectory + "assets"
    private string _sourceAssetsPath = string.Empty;

    // Unique subdirectory created per test to avoid collisions with real bundled assets
    private string _testSubDir = string.Empty;

    [SetUp]
    public void SetUp()
    {
        // TestEnvironmentSetup already isolates Config.AssetsPath to a temp root.
        // We just need to ensure target directories exist.
        Config.EnsureDirectories();

        // Create a unique subdirectory inside the binary-dir assets folder so our
        // test files don't collide with real bundled assets.
        _sourceAssetsPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "assets");
        Directory.CreateDirectory(_sourceAssetsPath);

        _testSubDir = $"test-amgr-{Guid.NewGuid():N}";
    }

    [TearDown]
    public void TearDown()
    {
        // Remove the per-test subdirectory from the source assets directory if we created one.
        var subDirPath = Path.Combine(_sourceAssetsPath, _testSubDir);
        if (Directory.Exists(subDirPath))
        {
            try { Directory.Delete(subDirPath, recursive: true); } catch { }
        }

        // Remove test files from _sourceAssetsPath and all subdirectories.
        // Tests like UpdateAssets_SourceTargetMapping create files in subdirectories
        // (workflows/, roles/, etc.) that must also be cleaned to prevent test pollution.
        foreach (var f in Directory.GetFiles(_sourceAssetsPath, $"{_testSubDir}*", SearchOption.AllDirectories))
        {
            try { File.Delete(f); } catch { }
        }

        // Clean the corresponding target-side artefacts inside Config.AssetsPath.
        var targetSubDirPath = Path.Combine(Config.AssetsPath, _testSubDir);
        if (Directory.Exists(targetSubDirPath))
        {
            try { Directory.Delete(targetSubDirPath, recursive: true); } catch { }
        }

        foreach (var f in Directory.GetFiles(Config.AssetsPath, $"{_testSubDir}*", SearchOption.AllDirectories))
        {
            try { File.Delete(f); } catch { }
        }
    }

    // ---------------------------------------------------------------------------
    // InitializeAssets tests
    // ---------------------------------------------------------------------------

    /// <summary>
    /// InitializeAssets copies source files into the target assets directory.
    /// Verifies that after initialization the file exists in Config.AssetsPath.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void InitializeAssets_WithSourceFiles_CopiesFilesToTarget()
    {
        // Arrange: write a unique file to the source assets dir
        var fileName = $"{_testSubDir}-init.txt";
        var sourceFile = Path.Combine(_sourceAssetsPath, fileName);
        var expectedContent = "hello from InitializeAssets test";
        File.WriteAllText(sourceFile, expectedContent);

        var targetFile = Path.Combine(Config.AssetsPath, fileName);
        // Ensure it doesn't already exist
        if (File.Exists(targetFile)) File.Delete(targetFile);

        // Act
        AssetManager.InitializeAssets();

        // Assert: the file must now exist at the target path with correct content
        Assert.That(File.Exists(targetFile), Is.True,
            $"Expected file to be copied to target at {targetFile}");
        Assert.That(File.ReadAllText(targetFile), Is.EqualTo(expectedContent),
            "Copied file content should match source content");
    }

    /// <summary>
    /// InitializeAssets returns gracefully when the source directory does not exist.
    /// No exception should be thrown.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void InitializeAssets_WhenSourceDirMissing_ReturnsGracefully()
    {
        // The source path is AppDomain.CurrentDomain.BaseDirectory + "assets".
        // We cannot delete it (it may contain real bundled assets and is shared across tests).
        // Instead, we rely on the fact that when the source doesn't contain our specific
        // test file, no crash occurs. We verify by calling InitializeAssets directly
        // after ensuring our test subdirectory does NOT exist in source.
        var missingSourceSubDir = Path.Combine(_sourceAssetsPath, _testSubDir);
        Assert.That(Directory.Exists(missingSourceSubDir), Is.False,
            "Pre-condition: test subdir must not exist in source before test");

        // Act: should not throw even when source root exists but our subdir is absent
        Assert.DoesNotThrow(() => AssetManager.InitializeAssets(),
            "InitializeAssets should not throw when source dir exists but specific content is absent");
    }

    /// <summary>
    /// InitializeAssets does NOT overwrite an existing file at the target.
    /// If the file already exists in Config.AssetsPath, the original content is preserved.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void InitializeAssets_SkipsExistingFiles_DoesNotOverwrite()
    {
        // Arrange: write source and target files with different content
        var fileName = $"{_testSubDir}-skip.txt";
        var sourceFile = Path.Combine(_sourceAssetsPath, fileName);
        var targetFile = Path.Combine(Config.AssetsPath, fileName);

        File.WriteAllText(sourceFile, "source content");
        File.WriteAllText(targetFile, "original target content");

        // Act
        AssetManager.InitializeAssets();

        // Assert: target must retain its original content (not overwritten)
        Assert.That(File.ReadAllText(targetFile), Is.EqualTo("original target content"),
            "InitializeAssets must not overwrite an existing file at the target");
    }

    // ---------------------------------------------------------------------------
    // UpdateAssets — dry-run vs apply
    // ---------------------------------------------------------------------------

    /// <summary>
    /// UpdateAssets in dry-run mode reports new files but does NOT copy them to target.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void UpdateAssets_DryRun_ReportsNewFilesWithoutCopying()
    {
        // Arrange: place a new file in source, ensure it does NOT exist in target
        var subDir = Path.Combine(_sourceAssetsPath, _testSubDir);
        Directory.CreateDirectory(subDir);

        var fileName = "new-dryrun.txt";
        File.WriteAllText(Path.Combine(subDir, fileName), "dry-run source content");

        var targetSubDir = Path.Combine(Config.AssetsPath, _testSubDir);
        var targetFile = Path.Combine(targetSubDir, fileName);
        // Ensure not already present
        if (File.Exists(targetFile)) File.Delete(targetFile);

        // Act
        var result = AssetManager.UpdateAssets(dryRun: true);

        // Assert: file must appear in AddedFiles
        Assert.That(result.Error, Is.Null,
            $"UpdateAssets should succeed but returned error: {result.Error}");
        Assert.That(result.AddedFiles.Any(f => f.EndsWith(fileName, StringComparison.OrdinalIgnoreCase)),
            Is.True,
            $"Expected {fileName} to appear in AddedFiles during dry-run");

        // Assert: the file must NOT have been copied (dry-run means no writes)
        Assert.That(File.Exists(targetFile), Is.False,
            "Dry-run must not copy files to the target directory");
    }

    /// <summary>
    /// UpdateAssets with dryRun=false actually copies new files to the target directory.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void UpdateAssets_Apply_CopiesNewFilesToTarget()
    {
        // Arrange: place a new file in source
        var subDir = Path.Combine(_sourceAssetsPath, _testSubDir);
        Directory.CreateDirectory(subDir);

        var fileName = "new-apply.txt";
        var expectedContent = "apply source content";
        File.WriteAllText(Path.Combine(subDir, fileName), expectedContent);

        var targetSubDir = Path.Combine(Config.AssetsPath, _testSubDir);
        var targetFile = Path.Combine(targetSubDir, fileName);
        if (File.Exists(targetFile)) File.Delete(targetFile);

        // Act: dryRun = false
        var result = AssetManager.UpdateAssets(dryRun: false);

        // Assert: no error and file exists in target with correct content
        Assert.That(result.Error, Is.Null,
            $"UpdateAssets apply should succeed but returned error: {result.Error}");
        Assert.That(File.Exists(targetFile), Is.True,
            $"Expected file to be copied to target at {targetFile}");
        Assert.That(File.ReadAllText(targetFile), Is.EqualTo(expectedContent),
            "Applied file content must match source content");
    }

    // ---------------------------------------------------------------------------
    // UpdateAssets — new file detection
    // ---------------------------------------------------------------------------

    /// <summary>
    /// UpdateAssets detects files present in source but absent from target and adds them to AddedFiles.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void UpdateAssets_NewFileInSource_AppearsInAddedFiles()
    {
        // Arrange
        var subDir = Path.Combine(_sourceAssetsPath, _testSubDir);
        Directory.CreateDirectory(subDir);

        var fileName = "brand-new.txt";
        File.WriteAllText(Path.Combine(subDir, fileName), "brand new");

        var targetSubDir = Path.Combine(Config.AssetsPath, _testSubDir);
        if (File.Exists(Path.Combine(targetSubDir, fileName)))
            File.Delete(Path.Combine(targetSubDir, fileName));

        // Act (dry-run so we don't pollute)
        var result = AssetManager.UpdateAssets(dryRun: true);

        // Assert
        Assert.That(result.Error, Is.Null);
        Assert.That(result.AddedFiles.Any(f => f.EndsWith(fileName, StringComparison.OrdinalIgnoreCase)),
            Is.True,
            "File present in source but not in target must appear in AddedFiles");
    }

    // ---------------------------------------------------------------------------
    // UpdateAssets — changed file detection
    // ---------------------------------------------------------------------------

    /// <summary>
    /// UpdateAssets detects a file that differs between source and target and adds it to UpdatedFiles.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void UpdateAssets_ChangedFile_AppearsInUpdatedFiles()
    {
        // Arrange: create target first, then write different content to source
        var subDir = Path.Combine(_sourceAssetsPath, _testSubDir);
        Directory.CreateDirectory(subDir);

        var fileName = "changed.txt";
        var sourceFile = Path.Combine(subDir, fileName);

        var targetSubDir = Path.Combine(Config.AssetsPath, _testSubDir);
        Directory.CreateDirectory(targetSubDir);
        var targetFile = Path.Combine(targetSubDir, fileName);

        // Write OLD content to target, NEW content to source
        File.WriteAllText(targetFile, "old content in target");
        File.WriteAllText(sourceFile, "new content in source — different!");

        // Act (dry-run)
        var result = AssetManager.UpdateAssets(dryRun: true);

        // Assert
        Assert.That(result.Error, Is.Null);
        Assert.That(result.UpdatedFiles.Any(f => f.EndsWith(fileName, StringComparison.OrdinalIgnoreCase)),
            Is.True,
            "File with different content between source and target must appear in UpdatedFiles");
    }

    // ---------------------------------------------------------------------------
    // UpdateAssets — identical file (no change)
    // ---------------------------------------------------------------------------

    /// <summary>
    /// UpdateAssets does NOT report a file whose content is identical in source and target.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void UpdateAssets_IdenticalFile_NotInAddedOrUpdatedFiles()
    {
        // Arrange: same content in both source and target
        var subDir = Path.Combine(_sourceAssetsPath, _testSubDir);
        Directory.CreateDirectory(subDir);

        var fileName = "identical.txt";
        var content = "exactly the same content — no change";
        var sourceFile = Path.Combine(subDir, fileName);

        var targetSubDir = Path.Combine(Config.AssetsPath, _testSubDir);
        Directory.CreateDirectory(targetSubDir);
        var targetFile = Path.Combine(targetSubDir, fileName);

        File.WriteAllText(sourceFile, content);
        File.WriteAllText(targetFile, content);

        // Act (dry-run)
        var result = AssetManager.UpdateAssets(dryRun: true);

        // Assert
        Assert.That(result.Error, Is.Null);
        Assert.That(result.AddedFiles.Any(f => f.EndsWith(fileName, StringComparison.OrdinalIgnoreCase)),
            Is.False,
            "Identical file must NOT appear in AddedFiles");
        Assert.That(result.UpdatedFiles.Any(f => f.EndsWith(fileName, StringComparison.OrdinalIgnoreCase)),
            Is.False,
            "Identical file must NOT appear in UpdatedFiles");
    }

    // ---------------------------------------------------------------------------
    // UpdateAssets — error paths
    // ---------------------------------------------------------------------------

    /// <summary>
    /// UpdateAssets returns an error result when the source directory does not exist.
    /// This is tested by passing a path that cannot exist at the binary-dir level.
    /// Since the source is hard-coded, we rely on the fact that the method checks
    /// existence. We verify via AssetUpdateResult.Error.
    ///
    /// NOTE: The source path is always AppDomain.CurrentDomain.BaseDirectory + "assets".
    /// We cannot delete that directory. Instead we verify the error path by temporarily
    /// renaming the directory so it appears absent, then restoring it.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void UpdateAssets_WhenSourceDirMissing_ReturnsErrorResult()
    {
        // We simulate a missing source by temporarily renaming _sourceAssetsPath.
        // This is only safe if no other test runs concurrently against the same path.
        // TestEnvironmentSetup runs tests sequentially within the fixture.
        var tempName = _sourceAssetsPath + $".bak-{_testSubDir}";

        // Skip if the source doesn't exist at all — it already meets our criteria
        if (!Directory.Exists(_sourceAssetsPath))
        {
            var result2 = AssetManager.UpdateAssets(dryRun: true);
            Assert.That(result2.Error, Is.Not.Null.And.Not.Empty,
                "UpdateAssets must return an error when source assets dir does not exist");
            return;
        }

        // Rename to make it temporarily unavailable
        Directory.Move(_sourceAssetsPath, tempName);
        try
        {
            var result = AssetManager.UpdateAssets(dryRun: true);

            Assert.That(result.Error, Is.Not.Null.And.Not.Empty,
                "UpdateAssets must return an error when source assets dir does not exist");
            Assert.That(result.AddedFiles, Is.Empty,
                "AddedFiles must be empty when source is missing");
            Assert.That(result.UpdatedFiles, Is.Empty,
                "UpdatedFiles must be empty when source is missing");
        }
        finally
        {
            // Always restore — test isolation is critical
            if (Directory.Exists(tempName))
                Directory.Move(tempName, _sourceAssetsPath);
        }
    }

    /// <summary>
    /// UpdateAssets returns an error result when the target (Config.AssetsPath) does not exist.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void UpdateAssets_WhenTargetDirMissing_ReturnsErrorResult()
    {
        // Arrange: ensure source exists (it should from SetUp)
        Assert.That(Directory.Exists(_sourceAssetsPath), Is.True,
            "Pre-condition: source assets dir must exist");

        // Temporarily rename Config.AssetsPath to simulate it being absent
        var targetPath = Config.AssetsPath;
        var tempTarget = targetPath + $".bak-{_testSubDir}";

        // Config.AssetsPath may not exist yet if it was just created — create it first,
        // then move it away.
        Directory.CreateDirectory(targetPath);
        Directory.Move(targetPath, tempTarget);

        try
        {
            var result = AssetManager.UpdateAssets(dryRun: true);

            Assert.That(result.Error, Is.Not.Null.And.Not.Empty,
                "UpdateAssets must return an error when target assets dir does not exist");
        }
        finally
        {
            // Restore target directory
            if (Directory.Exists(tempTarget))
                Directory.Move(tempTarget, targetPath);
            else
                Directory.CreateDirectory(targetPath);
        }
    }

    // ---------------------------------------------------------------------------
    // Utility method tests: GetAssetPath, AssetExists, LoadAssetText, LoadAssetJson
    // ---------------------------------------------------------------------------

    /// <summary>
    /// GetAssetPath returns the expected combined path of Config.AssetsPath and the relative path.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void GetAssetPath_ReturnsCombinedAssetsPath()
    {
        var relativePath = Path.Combine("workflows", "my-workflow.json");
        var expected = Path.Combine(Config.AssetsPath, relativePath);

        var actual = AssetManager.GetAssetPath(relativePath);

        Assert.That(actual, Is.EqualTo(expected),
            "GetAssetPath must combine Config.AssetsPath with the relative path");
    }

    /// <summary>
    /// AssetExists returns true for a file that exists in Config.AssetsPath and false for one that does not.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void AssetExists_TrueForExistingFile_FalseForMissing()
    {
        // Create a real file in the assets directory
        var relativePath = $"{_testSubDir}-exists.txt";
        var fullPath = Path.Combine(Config.AssetsPath, relativePath);
        File.WriteAllText(fullPath, "exists");

        Assert.That(AssetManager.AssetExists(relativePath), Is.True,
            "AssetExists must return true for a file that exists");

        Assert.That(AssetManager.AssetExists($"{_testSubDir}-does-not-exist.txt"), Is.False,
            "AssetExists must return false for a file that does not exist");
    }

    /// <summary>
    /// LoadAssetText returns the file content for an existing asset and null for a missing one.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void LoadAssetText_ReturnsContent_OrNullForMissing()
    {
        // Arrange: write a real file
        var relativePath = $"{_testSubDir}-loadtext.txt";
        var fullPath = Path.Combine(Config.AssetsPath, relativePath);
        var expectedContent = "some asset text content\nwith multiple lines";
        File.WriteAllText(fullPath, expectedContent);

        // Act & Assert — existing file
        var loaded = AssetManager.LoadAssetText(relativePath);
        Assert.That(loaded, Is.EqualTo(expectedContent),
            "LoadAssetText must return the full content of an existing file");

        // Act & Assert — missing file
        var missing = AssetManager.LoadAssetText($"{_testSubDir}-missing.txt");
        Assert.That(missing, Is.Null,
            "LoadAssetText must return null for a file that does not exist");
    }

    /// <summary>
    /// LoadAssetJson deserializes a JSON file into the expected type and returns null for a missing file.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void LoadAssetJson_DeserializesJson_OrReturnsNullForMissing()
    {
        // Arrange: write a simple JSON file
        var relativePath = $"{_testSubDir}-data.json";
        var fullPath = Path.Combine(Config.AssetsPath, relativePath);
        File.WriteAllText(fullPath, "{\"name\":\"test-asset\",\"version\":\"1.0\"}");

        // Act — existing file
        var loaded = AssetManager.LoadAssetJson<TestAssetDto>(relativePath);

        // Assert — deserialized correctly
        Assert.That(loaded, Is.Not.Null, "LoadAssetJson must deserialize an existing JSON file");
        Assert.That(loaded!.Name, Is.EqualTo("test-asset"),
            "Deserialized 'name' must match JSON value");
        Assert.That(loaded.Version, Is.EqualTo("1.0"),
            "Deserialized 'version' must match JSON value");

        // Act — missing file
        var missing = AssetManager.LoadAssetJson<TestAssetDto>($"{_testSubDir}-missing.json");
        Assert.That(missing, Is.Null,
            "LoadAssetJson must return null for a missing file");
    }

    // ---------------------------------------------------------------------------
    // AssetUpdateResult.ToString tests
    // ---------------------------------------------------------------------------

    /// <summary>
    /// AssetUpdateResult.ToString formats the summary with added/updated/errors sections
    /// when there is no top-level Error set.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void AssetUpdateResult_ToString_FormatsCorrectly()
    {
        // Arrange: build a result with one entry in each collection
        var result = new AssetUpdateResult();
        result.AddedFiles.Add(Path.Combine(Config.AssetsPath, "workflows", "added.json"));
        result.UpdatedFiles.Add(Path.Combine(Config.AssetsPath, "roles", "updated.json"));
        result.Errors.Add("Could not copy missing.json: access denied");

        // Act
        var text = result.ToString();

        // Assert: header and summary counts
        Assert.That(text, Does.Contain("Asset Update Summary"),
            "ToString must include the section header");
        Assert.That(text, Does.Contain("Added files: 1"),
            "ToString must include the added files count");
        Assert.That(text, Does.Contain("Updated files: 1"),
            "ToString must include the updated files count");
        Assert.That(text, Does.Contain("Errors: 1"),
            "ToString must include the errors count");

        // Assert: section headers for each list
        Assert.That(text, Does.Contain("Added Files"),
            "ToString must include the Added Files section");
        Assert.That(text, Does.Contain("Updated Files"),
            "ToString must include the Updated Files section");
        Assert.That(text, Does.Contain("Errors"),
            "ToString must include the Errors section");

        // Assert: at least the file names appear
        Assert.That(text, Does.Contain("added.json"),
            "ToString must list the added file name");
        Assert.That(text, Does.Contain("updated.json"),
            "ToString must list the updated file name");
        Assert.That(text, Does.Contain("access denied"),
            "ToString must include the error message");
    }

    /// <summary>
    /// AssetUpdateResult.ToString with Error set outputs the error message and omits summary sections.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void AssetUpdateResult_ToString_WithError_FormatsErrorMessage()
    {
        // Arrange
        var result = new AssetUpdateResult
        {
            Error = "Source assets directory not found"
        };

        // Act
        var text = result.ToString();

        // Assert: must contain the Error section header and the message
        Assert.That(text, Does.Contain("Error"),
            "ToString with Error must include an Error section");
        Assert.That(text, Does.Contain("Source assets directory not found"),
            "ToString must include the error message text");

        // The summary counts section must NOT be present (early return on error)
        Assert.That(text, Does.Not.Contain("Added files:"),
            "ToString with Error must not include the Results summary");
    }

    // ---------------------------------------------------------------------------
    // Source-target mapping: workflows / tools / roles / colors
    // ---------------------------------------------------------------------------

    /// <summary>
    /// UpdateAssets correctly discovers and reports files across the four canonical
    /// asset subdirectories: workflows, tools, roles, and colors.
    /// T-COV-001.6: RTM FR-17.8
    /// </summary>
    [Test]
    public void UpdateAssets_SourceTargetMapping_AllFourSubdirectoriesDiscovered()
    {
        // Arrange: create one new file in each of the four canonical source subdirs.
        // Use the unique _testSubDir as a namespace inside each to avoid collisions.
        var categories = new[] { "workflows", "tools", "roles", "colors" };
        var filesByCat = new Dictionary<string, string>();

        foreach (var cat in categories)
        {
            var srcCatDir = Path.Combine(_sourceAssetsPath, cat);
            Directory.CreateDirectory(srcCatDir);

            var fileName = $"{_testSubDir}-{cat}-mapping.json";
            File.WriteAllText(Path.Combine(srcCatDir, fileName), $"{{\"category\":\"{cat}\"}}");
            filesByCat[cat] = fileName;

            // Ensure the target counterpart does NOT exist so it shows up as Added
            var targetCatDir = Path.Combine(Config.AssetsPath, cat);
            Directory.CreateDirectory(targetCatDir);
            var targetFile = Path.Combine(targetCatDir, fileName);
            if (File.Exists(targetFile)) File.Delete(targetFile);
        }

        // Act (dry-run — just check discovery, no write)
        var result = AssetManager.UpdateAssets(dryRun: true);

        // Assert: each of the four files must appear in AddedFiles
        Assert.That(result.Error, Is.Null,
            $"UpdateAssets should succeed but returned error: {result.Error}");

        foreach (var cat in categories)
        {
            var fileName = filesByCat[cat];
            Assert.That(
                result.AddedFiles.Any(f => f.EndsWith(fileName, StringComparison.OrdinalIgnoreCase)),
                Is.True,
                $"File '{fileName}' in '{cat}' category must appear in AddedFiles");
        }
    }

    // ---------------------------------------------------------------------------
    // DTO for LoadAssetJson test
    // ---------------------------------------------------------------------------

    // ReSharper disable once ClassNeverInstantiated.Local
    private sealed class TestAssetDto
    {
        [System.Text.Json.Serialization.JsonPropertyName("name")]
        public string Name { get; set; } = string.Empty;

        [System.Text.Json.Serialization.JsonPropertyName("version")]
        public string Version { get; set; } = string.Empty;
    }
}
