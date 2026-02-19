using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

public class WikiLinkFilterTests
{
    private string _filterPath = string.Empty;

    [SetUp]
    public void SetUp()
    {
        Config.EnsureDirectories();
        _filterPath = WikiLinkFilter.FilterPath;
        // Clean up any existing filter file and reset static cache
        if (File.Exists(_filterPath))
            File.Delete(_filterPath);
        WikiLinkFilter.ResetCache();
    }

    [TearDown]
    public void TearDown()
    {
        if (File.Exists(_filterPath))
            File.Delete(_filterPath);
    }

    [Test]
    public void CheckFilter_WithFilterFile_BlocksMatchingConcepts()
    {
        File.WriteAllText(_filterPath, """{"tool": "Generic hub"}""");

        var (blocked, reasons) = WikiLinkFilter.CheckFilter(["tool", "react"]);

        Assert.That(blocked, Does.Contain("tool"));
        Assert.That(reasons["tool"], Is.EqualTo("Generic hub"));
        Assert.That(blocked, Does.Not.Contain("react"));
    }

    [Test]
    public void CheckFilter_MissingFile_ReturnsEmpty()
    {
        // No filter file exists (SetUp already deleted it)
        var (blocked, reasons) = WikiLinkFilter.CheckFilter(["tool", "agent"]);

        Assert.That(blocked, Is.Empty);
        Assert.That(reasons, Is.Empty);
    }

    [Test]
    public void CheckFilter_CaseInsensitiveMatching()
    {
        File.WriteAllText(_filterPath, """{"Tool": "Hub"}""");

        var (blocked, reasons) = WikiLinkFilter.CheckFilter(["tool"]);

        Assert.That(blocked, Does.Contain("tool"));
        Assert.That(reasons["tool"], Is.EqualTo("Hub"));
    }

    [Test]
    public void CheckFilter_MixedFilteredAndUnfiltered()
    {
        File.WriteAllText(_filterPath, """{"tool": "Generic hub", "agent": "Generic hub"}""");

        var (blocked, reasons) = WikiLinkFilter.CheckFilter(["tool", "react", "agent", "typescript"]);

        Assert.That(blocked, Has.Count.EqualTo(2));
        Assert.That(blocked, Does.Contain("tool"));
        Assert.That(blocked, Does.Contain("agent"));
        Assert.That(blocked, Does.Not.Contain("react"));
        Assert.That(blocked, Does.Not.Contain("typescript"));
    }

    [Test]
    public void CheckFilter_EmptyConceptsList_ReturnsEmpty()
    {
        File.WriteAllText(_filterPath, """{"tool": "Generic hub", "agent": "Generic hub"}""");

        var (blocked, reasons) = WikiLinkFilter.CheckFilter(new List<string>());

        Assert.That(blocked, Is.Empty);
        Assert.That(reasons, Is.Empty);
    }

    [Test]
    public void CheckFilter_MalformedJson_ReturnsEmpty()
    {
        File.WriteAllText(_filterPath, "not valid json {{{");

        var (blocked, reasons) = WikiLinkFilter.CheckFilter(["tool"]);

        Assert.That(blocked, Is.Empty);
        Assert.That(reasons, Is.Empty);
    }
}
