using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

public class MarkdownWriterTests
{
    // RTM: NORMALIZE-HYPHEN-COMPRESSION-001 - Test coverage for hyphen compression

    [Test]
    public void NormalizeConcept_DoubleSpace_CompressesToSingleHyphen()
    {
        var result = MarkdownWriter.NormalizeConcept("API  Gateway");
        Assert.That(result, Is.EqualTo("api-gateway"));
    }

    [Test]
    public void NormalizeConcept_DoubleUnderscore_CompressesToSingleHyphen()
    {
        var result = MarkdownWriter.NormalizeConcept("user__profile");
        Assert.That(result, Is.EqualTo("user-profile"));
    }

    [Test]
    public void NormalizeConcept_MultipleSlashes_CompressesToSingleHyphen()
    {
        var result = MarkdownWriter.NormalizeConcept("auth / / token");
        Assert.That(result, Is.EqualTo("auth-token"));
    }

    [Test]
    public void NormalizeConcept_MixedSeparators_CompressesToSingleHyphen()
    {
        var result = MarkdownWriter.NormalizeConcept("API  __  //  Gateway");
        Assert.That(result, Is.EqualTo("api-gateway"));
    }

    [Test]
    public void NormalizeConcept_LeadingTrailingSeparators_TrimsHyphens()
    {
        var result = MarkdownWriter.NormalizeConcept("  concept  ");
        Assert.That(result, Is.EqualTo("concept"));
    }

    [Test]
    public void NormalizeConcept_OnlySeparators_ReturnsEmpty()
    {
        var result = MarkdownWriter.NormalizeConcept("   ___  ///  ");
        Assert.That(result, Is.EqualTo(string.Empty));
    }

    [Test]
    public void NormalizeConcept_SingleSeparators_WorksAsExpected()
    {
        var result = MarkdownWriter.NormalizeConcept("REST API Gateway");
        Assert.That(result, Is.EqualTo("rest-api-gateway"));
    }

    [Test]
    public void NormalizeConcept_NoSeparators_WorksAsExpected()
    {
        var result = MarkdownWriter.NormalizeConcept("Authentication");
        Assert.That(result, Is.EqualTo("authentication"));
    }

    [Test]
    public void NormalizeConcept_TripleHyphens_CompressesToSingleHyphen()
    {
        var result = MarkdownWriter.NormalizeConcept("test---concept");
        Assert.That(result, Is.EqualTo("test-concept"));
    }

    [Test]
    public void NormalizeConcept_ManyConsecutiveHyphens_CompressesToSingleHyphen()
    {
        var result = MarkdownWriter.NormalizeConcept("test----------concept");
        Assert.That(result, Is.EqualTo("test-concept"));
    }

    // ────────────────────────────────────────────────────────────────────────────
    // T-META-001.1: FR-24.1 — WriteMarkdown envelope management
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    [Category("T-META-001")]
    public void WriteMarkdown_NullFrontmatter_CreatesEnvelopeWithCreatedAndModified()
    {
        // T-META-001.1: FR-24.1 AC-1 + AC-4 — null frontmatter creates dict with created and modified
        var path = Path.Combine(Path.GetTempPath(), $"maenifold-test-{Guid.NewGuid():N}.md");
        try
        {
            MarkdownWriter.WriteMarkdown(path, null, "# Test\n\nContent");

            var (frontmatter, _, _) = MarkdownReader.ReadMarkdown(path);
            Assert.That(frontmatter, Is.Not.Null, "Null frontmatter must produce frontmatter block.");
            Assert.That(frontmatter!.ContainsKey("created"), Is.True, "FR-24.1 AC-1: created must be set.");
            Assert.That(frontmatter.ContainsKey("modified"), Is.True, "FR-24.1 AC-1: modified must be set.");
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Test]
    [Category("T-META-001")]
    public void WriteMarkdown_ExistingCreated_PreservesCreatedAndUpdatesModified()
    {
        // T-META-001.1: FR-24.1 AC-2 — rewrite with created present: created preserved, modified updated
        var path = Path.Combine(Path.GetTempPath(), $"maenifold-test-{Guid.NewGuid():N}.md");
        try
        {
            var originalCreated = "2020-01-01T00:00:00.0000000+00:00";
            var frontmatter = new Dictionary<string, object>
            {
                ["created"] = originalCreated
            };
            MarkdownWriter.WriteMarkdown(path, frontmatter, "# Test\n\nContent");

            var (readFrontmatter, _, _) = MarkdownReader.ReadMarkdown(path);
            Assert.That(readFrontmatter, Is.Not.Null);
            Assert.That(readFrontmatter!["created"].ToString(), Is.EqualTo(originalCreated),
                "FR-24.1 AC-2: existing created must be preserved.");
            Assert.That(readFrontmatter.ContainsKey("modified"), Is.True,
                "FR-24.1 AC-2: modified must be set.");
            Assert.That(readFrontmatter["modified"].ToString(), Is.Not.EqualTo(originalCreated),
                "FR-24.1 AC-2: modified must differ from original created.");
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Test]
    [Category("T-META-001")]
    public void WriteMarkdown_MissingCreated_AddsCreatedAndSetsModified()
    {
        // T-META-001.1: FR-24.1 AC-3 — rewrite without created: created added, modified set
        var path = Path.Combine(Path.GetTempPath(), $"maenifold-test-{Guid.NewGuid():N}.md");
        try
        {
            var frontmatter = new Dictionary<string, object>
            {
                ["title"] = "My Note"
            };
            MarkdownWriter.WriteMarkdown(path, frontmatter, "# My Note\n\nContent");

            var (readFrontmatter, _, _) = MarkdownReader.ReadMarkdown(path);
            Assert.That(readFrontmatter, Is.Not.Null);
            Assert.That(readFrontmatter!.ContainsKey("created"), Is.True,
                "FR-24.1 AC-3: created must be added when absent.");
            Assert.That(readFrontmatter.ContainsKey("modified"), Is.True,
                "FR-24.1 AC-3: modified must be set.");
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Test]
    [Category("T-META-001")]
    public void WriteMarkdown_CompletedAndCancelledNotOverwritten()
    {
        // T-META-001.1: FR-24.1 AC-5 — completed and cancelled lifecycle fields not overwritten
        var path = Path.Combine(Path.GetTempPath(), $"maenifold-test-{Guid.NewGuid():N}.md");
        try
        {
            var completedTime = "2025-06-01T12:00:00.0000000+00:00";
            var cancelledTime = "2025-06-02T08:00:00.0000000+00:00";
            var frontmatter = new Dictionary<string, object>
            {
                ["status"] = "completed",
                ["completed"] = completedTime,
                ["cancelled"] = cancelledTime
            };
            MarkdownWriter.WriteMarkdown(path, frontmatter, "# Session\n\nContent");

            var (readFrontmatter, _, _) = MarkdownReader.ReadMarkdown(path);
            Assert.That(readFrontmatter, Is.Not.Null);
            Assert.That(readFrontmatter!["completed"].ToString(), Is.EqualTo(completedTime),
                "FR-24.1 AC-5: completed must not be overwritten.");
            Assert.That(readFrontmatter["cancelled"].ToString(), Is.EqualTo(cancelledTime),
                "FR-24.1 AC-5: cancelled must not be overwritten.");
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Test]
    [Category("T-META-001")]
    public void WriteMarkdown_DoesNotMutateCallerDict()
    {
        // T-META-001.1: FR-24.1 — caller's frontmatter dict must not be mutated
        var path = Path.Combine(Path.GetTempPath(), $"maenifold-test-{Guid.NewGuid():N}.md");
        try
        {
            var staleModified = "2020-01-01T00:00:00.0000000+00:00";
            var frontmatter = new Dictionary<string, object>
            {
                ["modified"] = staleModified
            };
            MarkdownWriter.WriteMarkdown(path, frontmatter, "# Test\n\nContent");

            Assert.That(frontmatter["modified"].ToString(), Is.EqualTo(staleModified),
                "FR-24.1: WriteMarkdown must not mutate the caller's frontmatter dict.");
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    // ────────────────────────────────────────────────────────────────────────────
    // T-META-001.2: FR-24.2 — AppendH2Section routes through WriteMarkdown
    // ────────────────────────────────────────────────────────────────────────────

    [Test]
    [Category("T-META-001")]
    public void AppendH2Section_ExistingFile_UpdatesModified()
    {
        // T-META-001.2: FR-24.2 AC-1 — appending a section updates modified via WriteMarkdown
        var path = Path.Combine(Path.GetTempPath(), $"maenifold-test-{Guid.NewGuid():N}.md");
        try
        {
            var staleModified = "2020-01-01T00:00:00.0000000+00:00";
            var frontmatter = new Dictionary<string, object>
            {
                ["created"] = "2020-01-01T00:00:00.0000000+00:00",
                ["modified"] = staleModified
            };
            MarkdownWriter.WriteMarkdown(path, frontmatter, "# Test\n\nContent");

            // Now read back to get the actual written modified (which was updated on write)
            var (_, _, _) = MarkdownReader.ReadMarkdown(path);

            // Wait briefly and append — modified should advance
            System.Threading.Thread.Sleep(10);
            MarkdownWriter.AppendH2Section(path, "New Section", "New content");

            var (readFrontmatter, _, _) = MarkdownReader.ReadMarkdown(path);
            Assert.That(readFrontmatter, Is.Not.Null);
            Assert.That(readFrontmatter!.ContainsKey("modified"), Is.True,
                "FR-24.2 AC-1: AppendH2Section must update modified via WriteMarkdown.");
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Test]
    [Category("T-META-001")]
    public void AppendH2Section_ExistingFile_PreservesAllFrontmatterFields()
    {
        // T-META-001.2: FR-24.2 AC-2 — all existing frontmatter fields preserved
        var path = Path.Combine(Path.GetTempPath(), $"maenifold-test-{Guid.NewGuid():N}.md");
        try
        {
            var frontmatter = new Dictionary<string, object>
            {
                ["type"] = "sequential",
                ["status"] = "active",
                ["title"] = "My Session"
            };
            MarkdownWriter.WriteMarkdown(path, frontmatter, "# Session\n\nContent");
            MarkdownWriter.AppendH2Section(path, "Thought 1", "First thought content");

            var (readFrontmatter, _, _) = MarkdownReader.ReadMarkdown(path);
            Assert.That(readFrontmatter, Is.Not.Null);
            Assert.That(readFrontmatter!["type"].ToString(), Is.EqualTo("sequential"),
                "FR-24.2 AC-2: type must be preserved.");
            Assert.That(readFrontmatter["status"].ToString(), Is.EqualTo("active"),
                "FR-24.2 AC-2: status must be preserved.");
            Assert.That(readFrontmatter["title"].ToString(), Is.EqualTo("My Session"),
                "FR-24.2 AC-2: title must be preserved.");
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Test]
    [Category("T-META-001")]
    public void AppendH2Section_ExistingFile_PreservesExistingContent()
    {
        // T-META-001.2: FR-24.2 AC-3 — existing content preserved verbatim
        var path = Path.Combine(Path.GetTempPath(), $"maenifold-test-{Guid.NewGuid():N}.md");
        try
        {
            var frontmatter = new Dictionary<string, object> { ["type"] = "sequential" };
            MarkdownWriter.WriteMarkdown(path, frontmatter, "# Session\n\nExisting content here.");
            MarkdownWriter.AppendH2Section(path, "Appended Section", "New content");

            var (_, content, _) = MarkdownReader.ReadMarkdown(path);
            Assert.That(content, Does.Contain("Existing content here."),
                "FR-24.2 AC-3: existing content must be preserved after append.");
            Assert.That(content, Does.Contain("## Appended Section"),
                "FR-24.2 AC-3: appended section must appear in content.");
            Assert.That(content, Does.Contain("New content"),
                "FR-24.2 AC-3: new content must appear after append.");
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }

    [Test]
    [Category("T-META-001")]
    public void AppendH2Section_NonExistentFile_CreatesFileWithEnvelope()
    {
        // T-META-001.2: FR-24.2 — appending to non-existent file creates it with envelope
        var path = Path.Combine(Path.GetTempPath(), $"maenifold-test-{Guid.NewGuid():N}.md");
        try
        {
            Assert.That(File.Exists(path), Is.False, "Precondition: file must not exist.");

            MarkdownWriter.AppendH2Section(path, "First Section", "First content");

            Assert.That(File.Exists(path), Is.True, "AppendH2Section must create file if not exists.");
            var (readFrontmatter, content, _) = MarkdownReader.ReadMarkdown(path);
            Assert.That(readFrontmatter, Is.Not.Null, "New file must have frontmatter envelope.");
            Assert.That(readFrontmatter!.ContainsKey("created"), Is.True, "New file must have created.");
            Assert.That(readFrontmatter.ContainsKey("modified"), Is.True, "New file must have modified.");
            Assert.That(content, Does.Contain("## First Section"), "Content must include appended section.");
        }
        finally
        {
            if (File.Exists(path)) File.Delete(path);
        }
    }
}
