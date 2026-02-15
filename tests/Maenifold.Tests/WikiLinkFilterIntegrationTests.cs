using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

public class WikiLinkFilterIntegrationTests
{
    private string _filterPath = string.Empty;
    private string _testFolderPath = string.Empty;

    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();
        _filterPath = WikiLinkFilter.FilterPath;
        _testFolderPath = Path.Combine(Config.MemoryPath, "wikilink-filter-integration-tests");
        Directory.CreateDirectory(_testFolderPath);

        // Write a test filter file blocking "tool" and "agent"
        File.WriteAllText(_filterPath, "tool | Generic hub - too connected\nagent | Generic hub - too connected\n");
    }

    [TearDown]
    public void TearDown()
    {
        if (File.Exists(_filterPath))
            File.Delete(_filterPath);
        if (Directory.Exists(_testFolderPath))
            Directory.Delete(_testFolderPath, true);
    }

    [Test]
    public void WriteMemory_WithFilteredConcept_ReturnsError()
    {
        var result = MemoryTools.WriteMemory("Test", "About [[tool]] and [[react]]");

        Assert.That(result, Does.Contain("ERROR"));
        Assert.That(result, Does.Contain("filtered WikiLinks"));
        Assert.That(result, Does.Contain("[[tool]]"));
        // react is not in the filter, so it should NOT appear in the error
        Assert.That(result, Does.Not.Contain("[[react]]"));
    }

    [Test]
    public void WriteMemory_MixedFilteredAndValid_StillBlocked()
    {
        var result = MemoryTools.WriteMemory("Test Mixed", "About [[tool]] and [[react]] and [[agent]]");

        Assert.That(result, Does.Contain("ERROR"));
        Assert.That(result, Does.Contain("[[tool]]"));
        Assert.That(result, Does.Contain("[[agent]]"));
        // Write should NOT succeed — no file created
        Assert.That(result, Does.Not.StartWith("Created memory FILE:"));
    }

    [Test]
    public void WriteMemory_NoFilterFile_AllowsAll()
    {
        // Remove the filter file so nothing is blocked
        if (File.Exists(_filterPath))
            File.Delete(_filterPath);

        var result = MemoryTools.WriteMemory("Filter Removed Test", "About [[tool]] and [[react]]",
            folder: "wikilink-filter-integration-tests");

        Assert.That(result, Does.StartWith("Created memory FILE:"));
    }

    [Test]
    public void EditMemory_WithFilteredConcept_ReturnsError()
    {
        // First, write a file successfully with filter disabled
        if (File.Exists(_filterPath))
            File.Delete(_filterPath);

        var writeResult = MemoryTools.WriteMemory("Edit Filter Test", "Original content with [[react]]",
            folder: "wikilink-filter-integration-tests");
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"));

        // Re-create the filter file
        File.WriteAllText(_filterPath, "tool | Generic hub - too connected\nagent | Generic hub - too connected\n");

        // Now attempt to edit with a filtered concept
        var uri = "memory://wikilink-filter-integration-tests/edit-filter-test";
        var editResult = MemoryTools.EditMemory(uri, "append", "New content with [[tool]]");

        Assert.That(editResult, Does.Contain("ERROR"));
        Assert.That(editResult, Does.Contain("filtered WikiLinks"));
        Assert.That(editResult, Does.Contain("[[tool]]"));
    }

    [Test]
    public void WriteMemory_FilteredConcept_JsonMode_ReturnsStructuredError()
    {
        using (OutputContext.UseJson())
        {
            var result = MemoryTools.WriteMemory("Json Filter Test", "About [[tool]] and [[react]]");

            Assert.That(result, Does.Contain("FILTERED_WIKILINKS"));
        }
    }
}
