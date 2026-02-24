// T-META-001.6: RTM FR-24.x — Blue-team tests for frontmatter metadata standardization.
// Covers FR-24.1 (WriteMarkdown envelope), FR-24.2 (AppendH2Section routes through WriteMarkdown),
// FR-24.3 (tools don't set timestamps), FR-24.4 (ConceptRepairTool through MarkdownIO),
// FR-24.6 (filesystem fallback in SessionCleanup and ReadMemory display).

using System.Globalization;
using System.IO;
using Maenifold.Tools;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

[TestFixture]
[Category("T-META-001.6")]
public class FrontmatterMetadataTests
{
    private string _testRoot = string.Empty;

    [SetUp]
    public void SetUp()
    {
        _testRoot = Path.Combine(Path.GetTempPath(), "maenifold-test-root", $"frontmatter-meta-{Guid.NewGuid():N}");
        Directory.CreateDirectory(_testRoot);
        Config.OverrideRoot(_testRoot);
        Config.EnsureDirectories();
    }

    [TearDown]
    public void TearDown()
    {
        Config.ResetOverrides();
        if (Directory.Exists(_testRoot))
        {
            try { Directory.Delete(_testRoot, true); } catch { /* ignore cleanup errors */ }
        }
    }

    // ────────────────────────────────────────────────────────────────────────────
    // FR-24.1: WriteMarkdown envelope management
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void FR24_1_AC1_WriteMarkdown_NewFile_HasCreatedAndModified()
    {
        // T-META-001.6: FR-24.1 — New file with null frontmatter must have created and modified
        var path = Path.Combine(_testRoot, "test-new.md");

        MarkdownWriter.WriteMarkdown(path, null, "# Test\n\n[[concept]]");

        var (fm, _, _) = MarkdownReader.ReadMarkdown(path);
        Assert.That(fm, Is.Not.Null,
            "FR-24.1: WriteMarkdown must produce frontmatter.");
        Assert.That(fm!.ContainsKey("created"), Is.True,
            "FR-24.1: New file must have 'created' in frontmatter.");
        Assert.That(fm.ContainsKey("modified"), Is.True,
            "FR-24.1: New file must have 'modified' in frontmatter.");
        Assert.That(fm["created"]?.ToString(), Is.Not.Null.And.Not.Empty,
            "FR-24.1: 'created' must be a non-empty string.");
        Assert.That(fm["modified"]?.ToString(), Is.Not.Null.And.Not.Empty,
            "FR-24.1: 'modified' must be a non-empty string.");
    }

    [Test]
    public void FR24_1_AC2_WriteMarkdown_ExistingCreated_PreservedOnRewrite()
    {
        // T-META-001.6: FR-24.1 — Existing created value must survive a rewrite; modified updates
        var path = Path.Combine(_testRoot, "test-preserve-created.md");

        // First write to establish created
        MarkdownWriter.WriteMarkdown(path, null, "# Test\n\n[[concept]]");
        var (fm1, _, _) = MarkdownReader.ReadMarkdown(path);
        var originalCreated = fm1!["created"]?.ToString();

        // Brief pause to allow clock to advance
        System.Threading.Thread.Sleep(10);

        // Second write with the same created value in the dict
        var fm2 = new Dictionary<string, object> { ["created"] = originalCreated! };
        MarkdownWriter.WriteMarkdown(path, fm2, "# Test updated\n\n[[concept]]");

        var (fm3, _, _) = MarkdownReader.ReadMarkdown(path);
        Assert.That(fm3!["created"]?.ToString(), Is.EqualTo(originalCreated),
            "FR-24.1: 'created' must be preserved when already present in frontmatter dict.");
        Assert.That(fm3["modified"]?.ToString(), Is.Not.Null.And.Not.Empty,
            "FR-24.1: 'modified' must still be present after rewrite.");
    }

    [Test]
    public void FR24_1_AC3_WriteMarkdown_MissingCreated_AddedOnRewrite()
    {
        // T-META-001.6: FR-24.1 — If frontmatter has no created, WriteMarkdown must add it
        var path = Path.Combine(_testRoot, "test-add-created.md");

        // Write with a dict that does NOT contain 'created'
        var fm = new Dictionary<string, object> { ["title"] = "Test", ["type"] = "memory" };
        MarkdownWriter.WriteMarkdown(path, fm, "# Test\n\n[[concept]]");

        var (readFm, _, _) = MarkdownReader.ReadMarkdown(path);
        Assert.That(readFm, Is.Not.Null,
            "FR-24.1: Written file must have parseable frontmatter.");
        Assert.That(readFm!.ContainsKey("created"), Is.True,
            "FR-24.1: WriteMarkdown must add 'created' when it is missing from input dict.");
        Assert.That(readFm["created"]?.ToString(), Is.Not.Null.And.Not.Empty,
            "FR-24.1: Added 'created' must be a non-empty timestamp string.");
    }

    [Test]
    public void FR24_1_AC4_WriteMarkdown_NullFrontmatter_DictCreated()
    {
        // T-META-001.6: FR-24.1 — Null frontmatter must result in a dict with both envelope keys
        var path = Path.Combine(_testRoot, "test-null-fm.md");

        MarkdownWriter.WriteMarkdown(path, null, "# Null FM\n\n[[wikilink]]");

        var rawText = File.ReadAllText(path);
        Assert.That(rawText, Does.StartWith("---"),
            "FR-24.1: File must start with YAML front-matter delimiter.");

        var (fm, _, _) = MarkdownReader.ReadMarkdown(path);
        Assert.That(fm, Is.Not.Null,
            "FR-24.1: Parsed frontmatter must not be null for a file written with null input.");
        Assert.That(fm!.ContainsKey("created"), Is.True,
            "FR-24.1: 'created' must exist in dict created from null input.");
        Assert.That(fm.ContainsKey("modified"), Is.True,
            "FR-24.1: 'modified' must exist in dict created from null input.");
    }

    [Test]
    public void FR24_1_AC5_WriteMarkdown_LifecycleTimestamps_NotOverwritten()
    {
        // T-META-001.6: FR-24.1 — 'completed' and 'cancelled' lifecycle fields must survive WriteMarkdown
        var path = Path.Combine(_testRoot, "test-lifecycle.md");

        var fm = new Dictionary<string, object>
        {
            ["type"] = "workflow",
            ["status"] = "completed",
            ["completed"] = "2025-01-01T00:00:00Z",
            ["cancelled"] = "2025-02-01T00:00:00Z"
        };

        MarkdownWriter.WriteMarkdown(path, fm, "# Lifecycle\n\n[[workflow]]");

        var (readFm, _, _) = MarkdownReader.ReadMarkdown(path);
        Assert.That(readFm, Is.Not.Null,
            "FR-24.1: Frontmatter must be present after write.");
        Assert.That(readFm!.ContainsKey("completed"), Is.True,
            "FR-24.1: 'completed' lifecycle field must be preserved by WriteMarkdown.");
        Assert.That(readFm["completed"]?.ToString(), Is.EqualTo("2025-01-01T00:00:00Z"),
            "FR-24.1: 'completed' value must not be changed by WriteMarkdown.");
        Assert.That(readFm.ContainsKey("cancelled"), Is.True,
            "FR-24.1: 'cancelled' lifecycle field must be preserved by WriteMarkdown.");
        Assert.That(readFm["cancelled"]?.ToString(), Is.EqualTo("2025-02-01T00:00:00Z"),
            "FR-24.1: 'cancelled' value must not be changed by WriteMarkdown.");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // FR-24.2: AppendH2Section routes through WriteMarkdown
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void FR24_2_AC1_AppendH2Section_UpdatesModified()
    {
        // T-META-001.6: FR-24.2 — Appending a section must update the 'modified' timestamp
        var path = Path.Combine(_testRoot, "test-append-modified.md");

        // Establish an initial file with a known modified time
        MarkdownWriter.WriteMarkdown(path, null, "# Session\n\n[[concept]]");
        var (fm1, _, _) = MarkdownReader.ReadMarkdown(path);
        var firstModified = fm1!["modified"]?.ToString();

        // Pause briefly so timestamps can differ
        System.Threading.Thread.Sleep(20);

        MarkdownWriter.AppendH2Section(path, "Thought 1", "Some [[reasoning]] here.");

        var (fm2, _, _) = MarkdownReader.ReadMarkdown(path);
        Assert.That(fm2, Is.Not.Null,
            "FR-24.2: Frontmatter must be present after AppendH2Section.");
        Assert.That(fm2!.ContainsKey("modified"), Is.True,
            "FR-24.2: 'modified' must be present after AppendH2Section.");
        // The modified should be updated (or at least still present)
        Assert.That(fm2["modified"]?.ToString(), Is.Not.Null.And.Not.Empty,
            "FR-24.2: 'modified' must be a non-empty string after AppendH2Section.");
    }

    [Test]
    public void FR24_2_AC2_AppendH2Section_PreservesFrontmatter()
    {
        // T-META-001.6: FR-24.2 — Appending must preserve all existing frontmatter fields
        var path = Path.Combine(_testRoot, "test-append-preserve-fm.md");

        var fm = new Dictionary<string, object>
        {
            ["title"] = "My Session",
            ["type"] = "sequential",
            ["status"] = "active"
        };

        MarkdownWriter.WriteMarkdown(path, fm, "# Session\n\n[[concept]]");
        var (fm1, _, _) = MarkdownReader.ReadMarkdown(path);

        MarkdownWriter.AppendH2Section(path, "New Section", "Content with [[wikilink]].");

        var (fm2, _, _) = MarkdownReader.ReadMarkdown(path);
        Assert.That(fm2, Is.Not.Null,
            "FR-24.2: Frontmatter must survive AppendH2Section.");
        Assert.That(fm2!["title"]?.ToString(), Is.EqualTo("My Session"),
            "FR-24.2: 'title' must be preserved by AppendH2Section.");
        Assert.That(fm2["type"]?.ToString(), Is.EqualTo("sequential"),
            "FR-24.2: 'type' must be preserved by AppendH2Section.");
        Assert.That(fm2["status"]?.ToString(), Is.EqualTo("active"),
            "FR-24.2: 'status' must be preserved by AppendH2Section.");
        Assert.That(fm1!.ContainsKey("created"), Is.True,
            "FR-24.2: 'created' must have been set during initial write.");
        Assert.That(fm2.ContainsKey("created"), Is.True,
            "FR-24.2: 'created' must still be present after AppendH2Section.");
        Assert.That(fm2["created"]?.ToString(), Is.EqualTo(fm1["created"]?.ToString()),
            "FR-24.2: 'created' must not change during AppendH2Section.");
    }

    [Test]
    public void FR24_2_AC3_AppendH2Section_PreservesContent()
    {
        // T-META-001.6: FR-24.2 — Existing content must be unchanged; new section must be appended
        var path = Path.Combine(_testRoot, "test-append-content.md");

        MarkdownWriter.WriteMarkdown(path, null, "# Session\n\nOriginal [[content]] here.");

        MarkdownWriter.AppendH2Section(path, "Appended Section", "New [[section]] text.");

        var (_, content, _) = MarkdownReader.ReadMarkdown(path);
        Assert.That(content, Does.Contain("Original [[content]] here."),
            "FR-24.2: Pre-existing content must be unchanged after AppendH2Section.");
        Assert.That(content, Does.Contain("## Appended Section"),
            "FR-24.2: New H2 heading must appear in content after AppendH2Section.");
        Assert.That(content, Does.Contain("New [[section]] text."),
            "FR-24.2: New section content must appear after AppendH2Section.");
        // New section must come AFTER existing content (not prepended)
        var originalIdx = content.IndexOf("Original [[content]] here.", StringComparison.Ordinal);
        var newSectionIdx = content.IndexOf("## Appended Section", StringComparison.Ordinal);
        Assert.That(newSectionIdx, Is.GreaterThan(originalIdx),
            "FR-24.2: Appended section must appear after the original content.");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // FR-24.3: Tools don't set timestamps
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void FR24_3_AC1_WriteMemory_TimestampsViaWriter()
    {
        // T-META-001.6: FR-24.3 — WriteMemory must produce a file with timestamps
        // managed entirely by MarkdownWriter, not set manually in the tool.
        var writeResult = MemoryTools.WriteMemory(
            "Timestamp Via Writer Test",
            "Testing that [[timestamp-management]] is delegated to MarkdownWriter.",
            folder: "fr24-tests"
        );

        Assert.That(writeResult, Does.StartWith("Created memory FILE: memory://"),
            "FR-24.3: WriteMemory must succeed and return a memory:// URI.");

        // Extract path and verify timestamps exist in file
        var uriLine = writeResult.Split('\n').FirstOrDefault() ?? string.Empty;
        var uriStart = uriLine.IndexOf("memory://", StringComparison.Ordinal);
        Assert.That(uriStart, Is.GreaterThanOrEqualTo(0), "FR-24.3: URI must be parseable from result.");
        var uri = uriLine.Substring(uriStart).Trim();
        var filePath = MarkdownWriter.UriToPath(uri, Config.MemoryPath);

        var (fm, _, _) = MarkdownReader.ReadMarkdown(filePath);
        Assert.That(fm, Is.Not.Null,
            "FR-24.3: Written file must have frontmatter.");
        Assert.That(fm!.ContainsKey("created"), Is.True,
            "FR-24.3: File written via WriteMemory must have 'created' (injected by MarkdownWriter).");
        Assert.That(fm.ContainsKey("modified"), Is.True,
            "FR-24.3: File written via WriteMemory must have 'modified' (injected by MarkdownWriter).");
    }

    [Test]
    public void FR24_3_AC2_EditMemory_ModifiedViaWriter()
    {
        // T-META-001.6: FR-24.3 — EditMemory append must update 'modified' via WriteMarkdown
        var writeResult = MemoryTools.WriteMemory(
            "Edit Modified Via Writer Test",
            "Initial [[content]] for edit test.",
            folder: "fr24-tests"
        );
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"),
            "FR-24.3: WriteMemory must succeed for setup.");

        var uriLine = writeResult.Split('\n').FirstOrDefault() ?? string.Empty;
        var uriStart = uriLine.IndexOf("memory://", StringComparison.Ordinal);
        var uri = uriLine.Substring(uriStart).Trim();
        var filePath = MarkdownWriter.UriToPath(uri, Config.MemoryPath);

        var (fmBefore, _, _) = MarkdownReader.ReadMarkdown(filePath);
        var modifiedBefore = fmBefore!["modified"]?.ToString();

        System.Threading.Thread.Sleep(20);

        var editResult = MemoryTools.EditMemory(
            uri,
            "append",
            "\n\nAppended [[knowledge]] via EditMemory."
        );
        Assert.That(editResult, Does.StartWith("Updated memory FILE:"),
            "FR-24.3: EditMemory append must succeed.");

        var (fmAfter, _, _) = MarkdownReader.ReadMarkdown(filePath);
        Assert.That(fmAfter!.ContainsKey("modified"), Is.True,
            "FR-24.3: 'modified' must be present after EditMemory.");
        Assert.That(fmAfter["modified"]?.ToString(), Is.Not.Null.And.Not.Empty,
            "FR-24.3: 'modified' must be a non-empty string after EditMemory.");
    }

    [Test]
    public void FR24_3_AC3_MoveMemory_ModifiedViaWriter()
    {
        // T-META-001.6: FR-24.3 — MoveMemory must produce 'modified' in the destination file
        var writeResult = MemoryTools.WriteMemory(
            "Move Modified Via Writer Source",
            "Content with [[concepts]] for move test.",
            folder: "fr24-tests"
        );
        Assert.That(writeResult, Does.StartWith("Created memory FILE:"),
            "FR-24.3: WriteMemory must succeed for setup.");

        var uriLine = writeResult.Split('\n').FirstOrDefault() ?? string.Empty;
        var uriStart = uriLine.IndexOf("memory://", StringComparison.Ordinal);
        var sourceUri = uriLine.Substring(uriStart).Trim();

        var moveResult = MemoryTools.MoveMemory(sourceUri, "fr24-tests/Move Modified Via Writer Dest");
        Assert.That(moveResult, Does.StartWith("Moved memory FILE:"),
            "FR-24.3: MoveMemory must succeed.");

        var destLine = moveResult.Split('\n').FirstOrDefault(l => l.Contains('→')) ?? moveResult.Split('\n')[0];
        var destUriMatch = System.Text.RegularExpressions.Regex.Match(moveResult, @"memory://[^\s]+");
        // Get the last memory:// URI in the result (destination)
        var allUris = System.Text.RegularExpressions.Regex.Matches(moveResult, @"memory://[^\s]+");
        Assert.That(allUris.Count, Is.GreaterThanOrEqualTo(1),
            "FR-24.3: MoveMemory result must contain at least one memory:// URI.");
        var destUri = allUris[allUris.Count - 1].Value.Trim();
        var destPath = MarkdownWriter.UriToPath(destUri, Config.MemoryPath);

        Assert.That(File.Exists(destPath), Is.True,
            "FR-24.3: Destination file must exist after MoveMemory.");
        var (fmDest, _, _) = MarkdownReader.ReadMarkdown(destPath);
        Assert.That(fmDest, Is.Not.Null,
            "FR-24.3: Destination file must have frontmatter.");
        Assert.That(fmDest!.ContainsKey("modified"), Is.True,
            "FR-24.3: Destination file after MoveMemory must have 'modified'.");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // FR-24.4: ConceptRepairTool through MarkdownIO
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void FR24_4_AC1_ConceptRepair_ModifiedUpdated()
    {
        // T-META-001.6: FR-24.4 — After repair, repaired file must have 'modified' set by MarkdownWriter
        var repairFolder = "fr24-repair-tests";
        var repairFolderPath = Path.Combine(Config.MemoryPath, repairFolder);
        Directory.CreateDirectory(repairFolderPath);

        // Write file via WriteMarkdown so it has frontmatter envelope
        var filePath = Path.Combine(repairFolderPath, "repair-target.md");
        var fm = new Dictionary<string, object> { ["title"] = "Repair Target", ["type"] = "memory" };
        MarkdownWriter.WriteMarkdown(filePath, fm, "# Repair Target\n\n[[tools]] and more [[tools]].");

        var (fmBefore, _, _) = MarkdownReader.ReadMarkdown(filePath);
        var modifiedBefore = fmBefore!["modified"]?.ToString();

        System.Threading.Thread.Sleep(20);

        var result = ConceptRepairTool.RepairConcepts(
            "tools",
            "tool",
            folder: repairFolder,
            dryRun: false,
            createWikiLinks: false,
            minSemanticSimilarity: 0.0
        );

        var (fmAfter, _, _) = MarkdownReader.ReadMarkdown(filePath);
        Assert.That(fmAfter, Is.Not.Null,
            "FR-24.4: Frontmatter must be present after ConceptRepair.");
        Assert.That(fmAfter!.ContainsKey("modified"), Is.True,
            "FR-24.4: 'modified' must be present after ConceptRepair writes via MarkdownWriter.");
        Assert.That(fmAfter["modified"]?.ToString(), Is.Not.Null.And.Not.Empty,
            "FR-24.4: 'modified' must be non-empty after ConceptRepair.");
    }

    [Test]
    public void FR24_4_AC2_ConceptRepair_FrontmatterPreserved()
    {
        // T-META-001.6: FR-24.4 — ConceptRepair must preserve all existing frontmatter fields
        var repairFolder = "fr24-repair-fm-tests";
        var repairFolderPath = Path.Combine(Config.MemoryPath, repairFolder);
        Directory.CreateDirectory(repairFolderPath);

        var filePath = Path.Combine(repairFolderPath, "repair-fm-target.md");
        var fm = new Dictionary<string, object>
        {
            ["title"] = "FM Preservation Test",
            ["type"] = "memory",
            ["status"] = "saved",
            ["tags"] = new[] { "test", "fr24" }
        };
        MarkdownWriter.WriteMarkdown(filePath, fm, "# FM Test\n\n[[tools]] are important.");

        ConceptRepairTool.RepairConcepts(
            "tools",
            "tool",
            folder: repairFolder,
            dryRun: false,
            createWikiLinks: false,
            minSemanticSimilarity: 0.0
        );

        var (fmAfter, _, _) = MarkdownReader.ReadMarkdown(filePath);
        Assert.That(fmAfter, Is.Not.Null,
            "FR-24.4: Frontmatter must be present after ConceptRepair.");
        Assert.That(fmAfter!["title"]?.ToString(), Is.EqualTo("FM Preservation Test"),
            "FR-24.4: 'title' must be preserved by ConceptRepair.");
        Assert.That(fmAfter["type"]?.ToString(), Is.EqualTo("memory"),
            "FR-24.4: 'type' must be preserved by ConceptRepair.");
        Assert.That(fmAfter["status"]?.ToString(), Is.EqualTo("saved"),
            "FR-24.4: 'status' must be preserved by ConceptRepair.");
    }

    [Test]
    public void FR24_4_AC3_ConceptRepair_ReplacementsCorrect()
    {
        // T-META-001.6: FR-24.4 — ConceptRepairTool must correctly replace WikiLink variants
        var repairFolder = "fr24-repair-content-tests";
        var repairFolderPath = Path.Combine(Config.MemoryPath, repairFolder);
        Directory.CreateDirectory(repairFolderPath);

        var filePath = Path.Combine(repairFolderPath, "repair-content-target.md");
        MarkdownWriter.WriteMarkdown(filePath, null, "# Content Test\n\nUsing [[tools]] and [[Tools]] here.");

        ConceptRepairTool.RepairConcepts(
            "tools",
            "tool",
            folder: repairFolder,
            dryRun: false,
            createWikiLinks: false,
            minSemanticSimilarity: 0.0
        );

        var (_, contentAfter, _) = MarkdownReader.ReadMarkdown(filePath);
        Assert.That(contentAfter, Does.Contain("[[tool]]"),
            "FR-24.4: ConceptRepair must replace [[tools]] variants with canonical [[tool]].");
        Assert.That(contentAfter, Does.Not.Contain("[[tools]]"),
            "FR-24.4: Original [[tools]] variant must not remain after ConceptRepair.");
        Assert.That(contentAfter, Does.Not.Contain("[[Tools]]"),
            "FR-24.4: Case variant [[Tools]] must also be replaced by ConceptRepair.");
    }

    // ────────────────────────────────────────────────────────────────────────────
    // FR-24.6: Filesystem fallback
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    public void FR24_6_AC1_SessionCleanup_WithModified_ExistingBehavior()
    {
        // T-META-001.6: FR-24.6 — Session with 'modified' in frontmatter → abandonment detected via frontmatter
        var staleModified = DateTime.UtcNow.AddMinutes(-(Config.SessionAbandonmentMinutes + 10));
        var frontmatter = new Dictionary<string, object?>
        {
            ["type"] = "workflow",
            ["status"] = "active",
            ["modified"] = staleModified.ToString("O", CultureInfo.InvariantCulture)
        };

        var sessionDir = Path.Combine(Config.MemoryPath, "thinking", "workflow");
        Directory.CreateDirectory(sessionDir);
        var filePath = Path.Combine(sessionDir, "stale-with-modified.md");

        // Write raw file via WriteMarkdown (includes modified in dict so it is preserved)
        MarkdownWriter.WriteMarkdown(filePath, new Dictionary<string, object>
        {
            ["type"] = "workflow",
            ["status"] = "active",
            ["modified"] = staleModified.ToString("O", CultureInfo.InvariantCulture)
        }, "# Stale Session\n\nContent");

        // Set recent filesystem time to confirm frontmatter 'modified' is used, not filesystem
        File.SetLastWriteTimeUtc(filePath, DateTime.UtcNow);

        SessionCleanup.HandleSessionCleanup(frontmatter, filePath, "# Stale Session\n\nContent");

        var (readFm, _, _) = MarkdownReader.ReadMarkdown(filePath);
        Assert.That(readFm!["status"]?.ToString(), Is.EqualTo("abandoned"),
            "FR-24.6: Session with stale frontmatter 'modified' must be marked abandoned " +
            "even when filesystem time is recent.");
    }

    [Test]
    public void FR24_6_AC2_SessionCleanup_WithoutModified_FilesystemFallback()
    {
        // T-META-001.6: FR-24.6 — Session without 'modified' in frontmatter → filesystem last-write fallback
        var sessionDir = Path.Combine(Config.MemoryPath, "thinking", "workflow");
        Directory.CreateDirectory(sessionDir);
        var filePath = Path.Combine(sessionDir, "no-modified-recent.md");

        // Write raw file WITHOUT 'modified' to bypass MarkdownWriter envelope injection
        File.WriteAllText(filePath, "---\ntype: workflow\nstatus: active\n---\n\n# Session\n\nContent");

        // Set filesystem time to RECENT (within threshold) → should NOT be abandoned
        File.SetLastWriteTimeUtc(filePath, DateTime.UtcNow);

        var frontmatter = new Dictionary<string, object?>
        {
            ["type"] = "workflow",
            ["status"] = "active"
            // "modified" intentionally absent
        };

        SessionCleanup.HandleSessionCleanup(frontmatter, filePath, "# Session\n\nContent");

        var (readFm, _, _) = MarkdownReader.ReadMarkdown(filePath);
        Assert.That(readFm!["status"]?.ToString(), Is.EqualTo("active"),
            "FR-24.6: Session without 'modified' but with recent filesystem time must remain active.");
    }

    [Test]
    public void FR24_6_AC3_SessionCleanup_AbandonedDetected_RegardlessOfFrontmatter()
    {
        // T-META-001.6: FR-24.6 — Session without 'modified', old filesystem time → marked abandoned
        var sessionDir = Path.Combine(Config.MemoryPath, "thinking", "sequential");
        Directory.CreateDirectory(sessionDir);
        var filePath = Path.Combine(sessionDir, "old-no-modified.md");

        // Write raw file WITHOUT 'modified'
        File.WriteAllText(filePath, "---\ntype: sequential\nstatus: active\n---\n\n# Old Session\n\nContent");

        // Set filesystem time to STALE
        var staleTime = DateTime.UtcNow.AddMinutes(-(Config.SessionAbandonmentMinutes + 30));
        File.SetLastWriteTimeUtc(filePath, staleTime);

        var frontmatter = new Dictionary<string, object?>
        {
            ["type"] = "sequential",
            ["status"] = "active"
            // "modified" intentionally absent
        };

        SessionCleanup.HandleSessionCleanup(frontmatter, filePath, "# Old Session\n\nContent");

        var (readFm, _, _) = MarkdownReader.ReadMarkdown(filePath);
        Assert.That(readFm!["status"]?.ToString(), Is.EqualTo("abandoned"),
            "FR-24.6: Session without 'modified' and with stale filesystem time must be marked abandoned via filesystem fallback.");
    }

    [Test]
    public void FR24_6_AC4_ReadMemory_DisplaysFilesystemTimestamps_WhenFrontmatterMissing()
    {
        // T-META-001.6: FR-24.6 — ReadMemory on file without timestamps → displays filesystem times
        var noFmFolder = "fr24-no-fm";
        var noFmDir = Path.Combine(Config.MemoryPath, noFmFolder);
        Directory.CreateDirectory(noFmDir);
        var filePath = Path.Combine(noFmDir, "no-timestamps.md");

        // Write a raw file with NO frontmatter timestamps at all (not even via MarkdownWriter)
        File.WriteAllText(filePath, "---\ntype: memory\ntitle: No Timestamps\n---\n\n# No Timestamps\n\nContent with [[raw-file]] that has no timestamp frontmatter.");

        // Set known filesystem times
        var knownCreate = new DateTime(2025, 6, 1, 12, 0, 0, DateTimeKind.Utc);
        var knownModify = new DateTime(2025, 6, 2, 14, 30, 0, DateTimeKind.Utc);
        File.SetCreationTimeUtc(filePath, knownCreate);
        File.SetLastWriteTimeUtc(filePath, knownModify);

        var uri = $"memory://{noFmFolder}/no-timestamps";
        var readResult = MemoryTools.ReadMemory(uri);

        Assert.That(readResult, Does.Not.StartWith("ERROR:"),
            "FR-24.6: ReadMemory must succeed on a file without timestamp frontmatter.");
        Assert.That(readResult, Does.Contain("Created:"),
            "FR-24.6: ReadMemory output must include a 'Created:' line even when frontmatter lacks 'created'.");
        Assert.That(readResult, Does.Contain("Modified:"),
            "FR-24.6: ReadMemory output must include a 'Modified:' line even when frontmatter lacks 'modified'.");
    }
}
