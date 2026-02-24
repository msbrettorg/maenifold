using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using Maenifold.Tools;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

// T-PROX-001.6: RTM FR-23.1–FR-23.5
// Blue-team tests for proximity-weighted co-occurrence feature.
// Tests are written FIRST — they define the behavioral contract.
// SWE implements the methods below to pass these tests.
//
// Methods under test (planned signatures):
//   MarkdownReader.ExtractWikiLinksWithSections(string content)
//     → (IReadOnlyList<(string concept, int sectionIndex)> concepts, bool usedFallback)
//   MarkdownIO.ExtractWikiLinksWithSections(string content)           (delegates to MarkdownReader)
//   ConceptSync.BuildGraphEdges(SqliteConnection conn, string memoryUri,
//       IReadOnlyList<(string concept, int sectionIndex)> conceptsWithSections)  [internal]
//   ConceptSync.ComputeProximityWeight(HashSet<int> sectionsA, HashSet<int> sectionsB) → double  [internal]
//   CommunityDetection.ComputeEdgeDecayWeight(string sourceFilesJson) → double  [internal]
//   ConceptSync.RemoveFile(conn, memoryUri, vectorReady)               [internal, updated]
[TestFixture]
[NonParallelizable]
public class ProximityWeightTests
{
    private string _testRoot = string.Empty;
    private string _previousMaenifoldRootEnv = string.Empty;
    private string _previousDatabasePathEnv = string.Empty;

    [SetUp]
    public void SetUp()
    {
        _previousMaenifoldRootEnv = Environment.GetEnvironmentVariable("MAENIFOLD_ROOT") ?? string.Empty;
        _previousDatabasePathEnv = Environment.GetEnvironmentVariable("MAENIFOLD_DATABASE_PATH") ?? string.Empty;

        var repoRoot = Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", "..", ".."));
        _testRoot = Path.Combine(repoRoot, "test-outputs", "proximity-weight", $"run-{Guid.NewGuid():N}");

        Directory.CreateDirectory(_testRoot);

        var testDbPath = Path.Combine(_testRoot, "memory.db");
        Environment.SetEnvironmentVariable("MAENIFOLD_ROOT", _testRoot);
        Environment.SetEnvironmentVariable("MAENIFOLD_DATABASE_PATH", testDbPath);
        Config.OverrideRoot(_testRoot);
        Config.SetDatabasePath(testDbPath);
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();
    }

    [TearDown]
    public void TearDown()
    {
        try
        {
            if (!string.IsNullOrWhiteSpace(_testRoot) && Directory.Exists(_testRoot))
            {
                var dir = new DirectoryInfo(_testRoot);
                foreach (var f in dir.GetFiles("*", SearchOption.AllDirectories))
                    f.Attributes = FileAttributes.Normal;
                Directory.Delete(_testRoot, recursive: true);
            }
        }
        catch
        {
            // Ignore cleanup failures; artifacts are under test-outputs for debugging.
        }

        Environment.SetEnvironmentVariable("MAENIFOLD_ROOT", string.IsNullOrEmpty(_previousMaenifoldRootEnv) ? null : _previousMaenifoldRootEnv);
        Environment.SetEnvironmentVariable("MAENIFOLD_DATABASE_PATH", string.IsNullOrEmpty(_previousDatabasePathEnv) ? null : _previousDatabasePathEnv);

        if (string.IsNullOrWhiteSpace(_previousMaenifoldRootEnv) && string.IsNullOrWhiteSpace(_previousDatabasePathEnv))
        {
            Config.ResetOverrides();
        }
        else
        {
            if (!string.IsNullOrWhiteSpace(_previousMaenifoldRootEnv))
                Config.OverrideRoot(_previousMaenifoldRootEnv);
            if (!string.IsNullOrWhiteSpace(_previousDatabasePathEnv))
                Config.SetDatabasePath(_previousDatabasePathEnv);
        }
        Config.EnsureDirectories();
        GraphDatabase.InitializeDatabase();
    }

    // ══════════════════════════════════════════════════════════════════════
    // T-PROX-001.1: ExtractWikiLinksWithSections — FR-23.1
    // Tests the MarkdownReader method that tags each WikiLink concept with
    // its H2 section index (0 = preamble, 1 = first H2, 2 = second H2, …).
    // ══════════════════════════════════════════════════════════════════════

    // ──────────────────────────────────────────────────────────────────────
    // AC-1: Multi-section doc → concepts tagged with correct section indices
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_1_AC1_MultiSectionDoc_ConceptsTaggedWithCorrectSectionIndices()
    {
        // T-PROX-001.1: FR-23.1 AC-1
        // Document with two H2 sections. Concepts in each section must be tagged
        // with the corresponding section index (1 for first H2, 2 for second H2).
        var content = """
            # Document Title

            ## Section One

            Discusses [[alpha]] and [[beta]].

            ## Section Two

            Introduces [[gamma]] and [[delta]].
            """;

        (IReadOnlyList<(string concept, int sectionIndex)> concepts, bool usedFallback) result =
            MarkdownReader.ExtractWikiLinksWithSections(content);

        Assert.That(result.usedFallback, Is.False, "Wellformed doc must not trigger fallback.");

        var lookup = result.concepts.ToLookup(c => c.concept, c => c.sectionIndex);

        Assert.That(lookup["alpha"].Single(), Is.EqualTo(1), "alpha is in section 1 (first H2).");
        Assert.That(lookup["beta"].Single(), Is.EqualTo(1), "beta is in section 1 (first H2).");
        Assert.That(lookup["gamma"].Single(), Is.EqualTo(2), "gamma is in section 2 (second H2).");
        Assert.That(lookup["delta"].Single(), Is.EqualTo(2), "delta is in section 2 (second H2).");
    }

    // ──────────────────────────────────────────────────────────────────────
    // AC-2: Flat doc (no H2) → all concepts at sectionIndex=0
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_1_AC2_FlatDoc_AllConceptsAtSectionZero()
    {
        // T-PROX-001.1: FR-23.1 AC-2
        // A document with no H2 headings; all WikiLinks must land at sectionIndex=0.
        var content = """
            # Just a Title

            This note references [[conceptA]], [[conceptB]], and [[conceptC]].
            No H2 headings anywhere.
            """;

        (IReadOnlyList<(string concept, int sectionIndex)> concepts, bool usedFallback) result =
            MarkdownReader.ExtractWikiLinksWithSections(content);

        Assert.That(result.usedFallback, Is.False, "Flat doc must not trigger fallback.");
        Assert.That(result.concepts, Is.Not.Empty, "Flat doc should extract concepts.");

        foreach (var entry in result.concepts)
        {
            Assert.That(entry.sectionIndex, Is.EqualTo(0),
                $"FR-23.1 AC-2: Concept '{entry.concept}' in flat doc must be at sectionIndex=0.");
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    // AC-3: Preamble content → sectionIndex=0
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_1_AC3_PreambleContent_SectionZero()
    {
        // T-PROX-001.1: FR-23.1 AC-3
        // Content appearing BEFORE the first H2 (the preamble) must be assigned sectionIndex=0.
        var content = """
            # Document

            This preamble mentions [[preamble-concept]] before any heading.

            ## First Section

            This section has [[section-concept]].
            """;

        (IReadOnlyList<(string concept, int sectionIndex)> concepts, bool usedFallback) result =
            MarkdownReader.ExtractWikiLinksWithSections(content);

        Assert.That(result.usedFallback, Is.False, "Should not trigger fallback.");

        var lookup = result.concepts.ToLookup(c => c.concept, c => c.sectionIndex);

        Assert.That(lookup["preamble-concept"].Any(), Is.True,
            "preamble-concept must be extracted.");
        Assert.That(lookup["preamble-concept"].Single(), Is.EqualTo(0),
            "FR-23.1 AC-3: preamble-concept must be at sectionIndex=0.");
        Assert.That(lookup["section-concept"].Single(), Is.EqualTo(1),
            "section-concept in the H2 must be at sectionIndex=1.");
    }

    // ──────────────────────────────────────────────────────────────────────
    // AC-4: Concept in multiple sections → appears with each section's index
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_1_AC4_ConceptInMultipleSections_AppearsWithEachIndex()
    {
        // T-PROX-001.1: FR-23.1 AC-4
        // When the same concept appears in both section 1 and section 2,
        // the result should contain a tuple for each section (deduplicated per section,
        // not globally — each section-concept pair appears exactly once).
        var content = """
            ## Section One

            References [[shared-concept]] here.

            ## Section Two

            Also references [[shared-concept]] here and again [[shared-concept]].

            ## Section Three

            Only [[unique-concept]] lives here.
            """;

        (IReadOnlyList<(string concept, int sectionIndex)> concepts, bool usedFallback) result =
            MarkdownReader.ExtractWikiLinksWithSections(content);

        Assert.That(result.usedFallback, Is.False, "Should not trigger fallback.");

        var sharedEntries = result.concepts.Where(c => c.concept == "shared-concept").ToList();
        var sectionIndices = sharedEntries.Select(c => c.sectionIndex).OrderBy(i => i).ToList();

        // shared-concept must appear exactly once per section, not once globally.
        Assert.That(sectionIndices, Does.Contain(1),
            "FR-23.1 AC-4: shared-concept must have an entry for section 1.");
        Assert.That(sectionIndices, Does.Contain(2),
            "FR-23.1 AC-4: shared-concept must have an entry for section 2.");

        // Per-section dedup: shared-concept mentioned twice in section 2 → only one entry for section 2
        Assert.That(sectionIndices.Count(i => i == 2), Is.EqualTo(1),
            "FR-23.1 AC-4: shared-concept must appear exactly once for section 2 (per-section dedup).");

        // unique-concept only in section 3
        var uniqueEntries = result.concepts.Where(c => c.concept == "unique-concept").ToList();
        Assert.That(uniqueEntries.Single().sectionIndex, Is.EqualTo(3),
            "unique-concept must be tagged with sectionIndex=3.");
    }

    // ──────────────────────────────────────────────────────────────────────
    // AC-5: WikiLinks in H2 heading text → extracted with correct sectionIndex
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_1_AC5_WikiLinksInH2Heading_ExtractedWithCorrectSectionIndex()
    {
        // T-PROX-002.1: FR-23.1 AC-5
        // WikiLinks appearing in the H2 heading text itself must be extracted
        // and tagged with the section index of that heading.
        var content = """
            ## Section about [[heading-concept]]

            Body text with [[body-concept]].

            ## Another [[second-heading]] section

            More body with [[second-body]].
            """;

        (IReadOnlyList<(string concept, int sectionIndex)> concepts, bool usedFallback) result =
            MarkdownReader.ExtractWikiLinksWithSections(content);

        Assert.That(result.usedFallback, Is.False, "Should not trigger fallback.");

        var lookup = result.concepts.ToLookup(c => c.concept, c => c.sectionIndex);

        // heading-concept is in the H2 heading that introduces section 1
        Assert.That(lookup["heading-concept"].Any(), Is.True,
            "FR-23.1 AC-5: heading-concept must be extracted from H2 heading text.");
        Assert.That(lookup["heading-concept"].Single(), Is.EqualTo(1),
            "FR-23.1 AC-5: heading-concept must be tagged with sectionIndex=1.");

        // body-concept is in section 1's body
        Assert.That(lookup["body-concept"].Single(), Is.EqualTo(1),
            "body-concept in section 1 body.");

        // second-heading is in the H2 heading that introduces section 2
        Assert.That(lookup["second-heading"].Any(), Is.True,
            "FR-23.1 AC-5: second-heading must be extracted from H2 heading text.");
        Assert.That(lookup["second-heading"].Single(), Is.EqualTo(2),
            "FR-23.1 AC-5: second-heading must be tagged with sectionIndex=2.");

        // second-body is in section 2's body
        Assert.That(lookup["second-body"].Single(), Is.EqualTo(2),
            "second-body in section 2 body.");
    }

    // ══════════════════════════════════════════════════════════════════════
    // T-PROX-001.2: BuildGraphEdges + ComputeProximityWeight — FR-23.1–23.3
    // Tests that edges are created with correct proximity weights and that
    // source_files is serialized as a {"uri": weight} dict (not an array).
    // ══════════════════════════════════════════════════════════════════════

    // ──────────────────────────────────────────────────────────────────────
    // AC-1: Same-section pair → weight 1.0 stored in source_files dict
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_2_AC1_SameSectionPair_Weight1_0()
    {
        // T-PROX-001.2: FR-23.2 AC-1
        // Two concepts both in section 1 → proximity weight = 1.0.
        // The source_files dict entry for the URI must have value 1.0.
        using var conn = OpenTestDb();
        InsertConcept(conn, "alpha");
        InsertConcept(conn, "beta");

        var conceptsWithSections = new List<(string concept, int sectionIndex)>
        {
            ("alpha", 1),
            ("beta", 1),
        };

        var uri = "memory://tests/same-section.md";
        ConceptSync.BuildGraphEdges(conn, uri, conceptsWithSections);

        var (_, weight) = GetEdgeWeight(conn, "alpha", "beta", uri);
        Assert.That(weight, Is.EqualTo(1.0).Within(1e-10),
            "FR-23.2 AC-1: Same-section pair must have proximity weight 1.0.");
    }

    // ──────────────────────────────────────────────────────────────────────
    // AC-2: Cross-section pair → weight 0.5
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_2_AC2_CrossSectionPair_Weight0_5()
    {
        // T-PROX-001.2: FR-23.2 AC-2
        // Concept A in section 1, concept B in section 2 → proximity = 0.5.
        using var conn = OpenTestDb();
        InsertConcept(conn, "alpha");
        InsertConcept(conn, "beta");

        var conceptsWithSections = new List<(string concept, int sectionIndex)>
        {
            ("alpha", 1),
            ("beta", 2),
        };

        var uri = "memory://tests/cross-section.md";
        ConceptSync.BuildGraphEdges(conn, uri, conceptsWithSections);

        var (_, weight) = GetEdgeWeight(conn, "alpha", "beta", uri);
        Assert.That(weight, Is.EqualTo(0.5).Within(1e-10),
            "FR-23.2 AC-2: Cross-section pair must have proximity weight 0.5.");
    }

    // ──────────────────────────────────────────────────────────────────────
    // AC-3: Flat doc (sectionIndex=0 for all) → all pairs weight 0.5
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_2_AC3_FlatDoc_AllPairsWeight0_5()
    {
        // T-PROX-001.2: FR-23.2 AC-3
        // All concepts at sectionIndex=0 (preamble / flat doc).
        // Section 0 counts as "preamble" — no named H2 section, so weight = 0.5.
        using var conn = OpenTestDb();
        InsertConcept(conn, "x");
        InsertConcept(conn, "y");
        InsertConcept(conn, "z");

        var conceptsWithSections = new List<(string concept, int sectionIndex)>
        {
            ("x", 0),
            ("y", 0),
            ("z", 0),
        };

        var uri = "memory://tests/flat-doc.md";
        ConceptSync.BuildGraphEdges(conn, uri, conceptsWithSections);

        var (_, wXY) = GetEdgeWeight(conn, "x", "y", uri);
        var (_, wXZ) = GetEdgeWeight(conn, "x", "z", uri);
        var (_, wYZ) = GetEdgeWeight(conn, "y", "z", uri);

        Assert.That(wXY, Is.EqualTo(0.5).Within(1e-10), "FR-23.2 AC-3: x-y in preamble = 0.5.");
        Assert.That(wXZ, Is.EqualTo(0.5).Within(1e-10), "FR-23.2 AC-3: x-z in preamble = 0.5.");
        Assert.That(wYZ, Is.EqualTo(0.5).Within(1e-10), "FR-23.2 AC-3: y-z in preamble = 0.5.");
    }

    // ──────────────────────────────────────────────────────────────────────
    // AC-4: Concept in preamble + section 1, paired with section 1 concept → 1.0
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_2_AC4_PreambleAndSection_MaxWins()
    {
        // T-PROX-001.2: FR-23.2 AC-4
        // "multi" appears in both preamble (section 0) and section 1.
        // "peer" appears only in section 1.
        // Because "multi" has section 1 in its set AND "peer" has section 1 in its set,
        // the pair has shared named section → weight = 1.0 (max wins over the preamble entry).
        using var conn = OpenTestDb();
        InsertConcept(conn, "multi");
        InsertConcept(conn, "peer");

        // multi at both preamble and section 1; peer at section 1 only.
        var conceptsWithSections = new List<(string concept, int sectionIndex)>
        {
            ("multi", 0),
            ("multi", 1),
            ("peer", 1),
        };

        var uri = "memory://tests/preamble-and-section.md";
        ConceptSync.BuildGraphEdges(conn, uri, conceptsWithSections);

        var (_, weight) = GetEdgeWeight(conn, "multi", "peer", uri);
        Assert.That(weight, Is.EqualTo(1.0).Within(1e-10),
            "FR-23.2 AC-4: Concept spanning preamble+section1 paired with section-1 concept → 1.0.");
    }

    // ──────────────────────────────────────────────────────────────────────
    // AC-5: source_files serialized as {"uri": weight} dict, not array
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_2_AC5_SourceFilesSerializedAsDict()
    {
        // T-PROX-001.2: FR-23.2 AC-5
        // After BuildGraphEdges, the source_files column must contain a JSON object
        // (dictionary) with the URI as key and the proximity weight as value.
        // It must NOT be a JSON array (the old format).
        using var conn = OpenTestDb();
        InsertConcept(conn, "alpha");
        InsertConcept(conn, "beta");

        var uri = "memory://tests/dict-format.md";
        var conceptsWithSections = new List<(string concept, int sectionIndex)>
        {
            ("alpha", 1),
            ("beta", 1),
        };

        ConceptSync.BuildGraphEdges(conn, uri, conceptsWithSections);

        var (sortedA, sortedB) = string.CompareOrdinal("alpha", "beta") < 0 ? ("alpha", "beta") : ("beta", "alpha");
        var rawJson = conn.QuerySingle<string>(
            "SELECT source_files FROM concept_graph WHERE concept_a = @a AND concept_b = @b",
            new { a = sortedA, b = sortedB });

        Assert.That(rawJson, Is.Not.Null.And.Not.Empty,
            "source_files must not be null/empty.");

        // Must parse as a JSON object (dict), not an array.
        using var doc = JsonDocument.Parse(rawJson!);
        Assert.That(doc.RootElement.ValueKind, Is.EqualTo(JsonValueKind.Object),
            "FR-23.2 AC-5: source_files must be a JSON object (dict), not an array.");

        // The URI must be a key in the dict.
        Assert.That(doc.RootElement.TryGetProperty(uri, out var weightElem), Is.True,
            $"FR-23.2 AC-5: URI '{uri}' must be a key in the source_files dict.");

        // The value must be the proximity weight (1.0 for same-section).
        Assert.That(weightElem.GetDouble(), Is.EqualTo(1.0).Within(1e-10),
            "FR-23.2 AC-5: Weight in source_files dict must be 1.0 for same-section pair.");
    }

    // ══════════════════════════════════════════════════════════════════════
    // T-PROX-001.3: ComputeEdgeDecayWeight — FR-23.4
    // Tests the updated CommunityDetection helper that reads the source_files
    // dict and computes effective weight = proximity × decay per source file.
    // ══════════════════════════════════════════════════════════════════════

    // ──────────────────────────────────────────────────────────────────────
    // AC-1: Effective weight = proximity × decay for each source file
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_4_AC1_EffectiveWeight_ProximityTimesDecay()
    {
        // T-PROX-001.3: FR-23.4 AC-1
        // Create a real memory file with a 'created' timestamp set to UtcNow
        // (decay ≈ 1.0). Store a proximity = 0.5 in the source_files dict.
        // ComputeEdgeDecayWeight should return ≈ 0.5 × 1.0 = 0.5.
        //
        // We use a "recently created" file so decay is approximately 1.0.
        var memoryDir = Path.Combine(_testRoot, "memory");
        Directory.CreateDirectory(memoryDir);

        var filePath = Path.Combine(memoryDir, "prox-decay-test.md");
        var createdUtc = DateTime.UtcNow.ToString("O");
        File.WriteAllText(filePath, $"""
            ---
            title: Proximity Decay Test
            created: {createdUtc}
            ---

            Content with [[test-concept]].
            """);

        var uri = MarkdownIO.PathToUri(filePath, Config.MemoryPath);

        // Build source_files JSON dict with proximity = 0.5.
        var sourceFilesJson = JsonSerializer.Serialize(new Dictionary<string, double>
        {
            [uri] = 0.5
        });

        var effectiveWeight = CommunityDetection.ComputeEdgeDecayWeight(sourceFilesJson);

        // With a recently-created file, decay ≈ 1.0, so effective ≈ 0.5.
        // We allow a tolerance because the file was just created (decay should be very close to 1.0).
        Assert.That(effectiveWeight, Is.GreaterThan(0.0),
            "FR-23.4 AC-1: ComputeEdgeDecayWeight must return a positive value.");
        Assert.That(effectiveWeight, Is.LessThanOrEqualTo(0.5 + 1e-6),
            "FR-23.4 AC-1: Effective weight = proximity × decay ≤ 0.5 for proximity=0.5.");
        Assert.That(effectiveWeight, Is.GreaterThan(0.4),
            "FR-23.4 AC-1: Fresh file decay ≈ 1.0, so effective weight must be close to 0.5.");
    }

    // ──────────────────────────────────────────────────────────────────────
    // AC-2: Corrupt source_files data → throws, does not silently return default
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_4_AC2_CorruptSourceFiles_Throws()
    {
        // T-PROX-001.3: FR-23.4 AC-2
        // Passing invalid JSON to ComputeEdgeDecayWeight must throw an exception.
        // The old behavior (catch { return rawCount; }) is removed per TODO.md.
        var corruptJson = "this is not valid json {{{{";

        Assert.That(() =>
        {
            CommunityDetection.ComputeEdgeDecayWeight(corruptJson);
        }, Throws.Exception, "FR-23.4 AC-2: ComputeEdgeDecayWeight must throw on corrupt source_files JSON.");
    }

    // ══════════════════════════════════════════════════════════════════════
    // T-PROX-001.4: BuildContext proximity weighting — FR-23.4
    // Tests that BuildContext uses proximity × decay in its avgDecayWeight
    // calculation and that stronger-proximity concepts rank higher.
    // ══════════════════════════════════════════════════════════════════════

    // ──────────────────────────────────────────────────────────────────────
    // AC-1: avgDecayWeight uses proximity × decay per file
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_4_AC1_BuildContext_AvgDecayWeight_UsesProximityTimesDecay()
    {
        // T-PROX-001.4: FR-23.4 AC-1
        // Create two edges from the same query concept:
        //   query <-> strong: source_files = {uri: 1.0}  (same-section, high proximity)
        //   query <-> weak:   source_files = {uri: 0.5}  (cross-section, low proximity)
        // Both edges reference the same freshly-created file (decay ≈ 1.0).
        // BuildContext on "query" must return:
        //   strong.DecayWeight ≈ 1.0 × decay ≈ 1.0
        //   weak.DecayWeight   ≈ 0.5 × decay ≈ 0.5
        // So strong.DecayWeight > weak.DecayWeight.
        using var conn = OpenTestDb();

        InsertConcept(conn, "prox-query");
        InsertConcept(conn, "prox-strong");
        InsertConcept(conn, "prox-weak");

        var memoryDir = Path.Combine(_testRoot, "memory");
        Directory.CreateDirectory(memoryDir);

        var filePath = Path.Combine(memoryDir, "prox-buildcontext.md");
        var createdUtc = DateTime.UtcNow.ToString("O");
        File.WriteAllText(filePath, $"""
            ---
            title: Proximity BuildContext Test
            created: {createdUtc}
            ---

            References [[prox-query]] and [[prox-strong]] and [[prox-weak]].
            """);

        var uri = MarkdownIO.PathToUri(filePath, Config.MemoryPath);

        // Insert edges directly with proximity-weighted source_files.
        InsertEdgeWithProximity(conn, "prox-query", "prox-strong", uri, 1.0);
        InsertEdgeWithProximity(conn, "prox-query", "prox-weak", uri, 0.5);

        conn.Dispose();

        var result = GraphTools.BuildContext("prox-query", depth: 1, maxEntities: 20);

        var strongRelation = result.DirectRelations.FirstOrDefault(r => r.Name == "prox-strong");
        var weakRelation = result.DirectRelations.FirstOrDefault(r => r.Name == "prox-weak");

        Assert.That(strongRelation, Is.Not.Null,
            "FR-23.4 AC-1: prox-strong must appear in DirectRelations.");
        Assert.That(weakRelation, Is.Not.Null,
            "FR-23.4 AC-1: prox-weak must appear in DirectRelations.");

        Assert.That(strongRelation!.DecayWeight, Is.GreaterThan(weakRelation!.DecayWeight),
            "FR-23.4 AC-1: Strong-proximity concept must have higher DecayWeight than weak-proximity.");
    }

    // ──────────────────────────────────────────────────────────────────────
    // AC-2: Strong-proximity concept ranks higher in results
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_4_AC2_BuildContext_StrongProximity_RanksHigher()
    {
        // T-PROX-001.4: FR-23.4 AC-2
        // Two concepts both co-occur once with the query concept (co_occurrence_count = 1),
        // but strong has proximity 1.0 and weak has proximity 0.5.
        // In BuildContext, WeightedScore = co_occurrence_count × avgDecayWeight.
        // With equal count and same-age files, strong.WeightedScore > weak.WeightedScore.
        // The result.DirectRelations ordering should place strong before weak.
        using var conn = OpenTestDb();

        InsertConcept(conn, "rank-query");
        InsertConcept(conn, "rank-strong");
        InsertConcept(conn, "rank-weak");

        var memoryDir = Path.Combine(_testRoot, "memory");
        Directory.CreateDirectory(memoryDir);

        var filePath = Path.Combine(memoryDir, "prox-ranking.md");
        var createdUtc = DateTime.UtcNow.ToString("O");
        File.WriteAllText(filePath, $"""
            ---
            title: Proximity Ranking Test
            created: {createdUtc}
            ---

            References [[rank-query]], [[rank-strong]], and [[rank-weak]].
            """);

        var uri = MarkdownIO.PathToUri(filePath, Config.MemoryPath);

        // Both edges: co_occurrence_count = 1, same file, different proximity.
        InsertEdgeWithProximity(conn, "rank-query", "rank-strong", uri, 1.0);
        InsertEdgeWithProximity(conn, "rank-query", "rank-weak", uri, 0.5);

        conn.Dispose();

        var result = GraphTools.BuildContext("rank-query", depth: 1, maxEntities: 20);

        var strongRelation = result.DirectRelations.FirstOrDefault(r => r.Name == "rank-strong");
        var weakRelation = result.DirectRelations.FirstOrDefault(r => r.Name == "rank-weak");

        Assert.That(strongRelation, Is.Not.Null, "rank-strong must appear in results.");
        Assert.That(weakRelation, Is.Not.Null, "rank-weak must appear in results.");

        Assert.That(strongRelation!.WeightedScore, Is.GreaterThan(weakRelation!.WeightedScore),
            "FR-23.4 AC-2: Strong-proximity concept must have higher WeightedScore than weak-proximity.");
    }

    // ──────────────────────────────────────────────────────────────────────
    // AC-3: Edge with empty source_files → DecayWeight = 0.0 (not 1.0)
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_4_AC3_BuildContext_EmptySourceFiles_DecayWeightZero()
    {
        // T-PROX-002.2: FR-23.4 AC-3
        // An edge with empty source_files (either NULL or "{}") should produce
        // DecayWeight = 0.0, not 1.0. Empty means no evidence → zero weight.
        using var conn = OpenTestDb();

        InsertConcept(conn, "empty-query");
        InsertConcept(conn, "empty-peer");

        // Insert edge with empty dict source_files.
        var (sortedA, sortedB) = string.CompareOrdinal("empty-query", "empty-peer") < 0
            ? ("empty-query", "empty-peer")
            : ("empty-peer", "empty-query");
        conn.Execute(
            @"INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
              VALUES (@a, @b, 1, '{}')",
            new { a = sortedA, b = sortedB });

        conn.Dispose();

        var result = GraphTools.BuildContext("empty-query", depth: 1, maxEntities: 20);

        var peerRelation = result.DirectRelations.FirstOrDefault(r => r.Name == "empty-peer");
        Assert.That(peerRelation, Is.Not.Null,
            "empty-peer must appear in DirectRelations.");
        Assert.That(peerRelation!.DecayWeight, Is.EqualTo(0.0).Within(1e-10),
            "FR-23.4 AC-3: Empty source_files must produce DecayWeight = 0.0, not 1.0.");
    }

    // ══════════════════════════════════════════════════════════════════════
    // T-PROX-001.5: RemoveFile with dict-format source_files — FR-23.1, FR-23.5
    // Tests that RemoveFile correctly handles the new dict format:
    // removes the target URI key while preserving other URIs and their weights.
    // ══════════════════════════════════════════════════════════════════════

    // ──────────────────────────────────────────────────────────────────────
    // AC-1: After RemoveFile, removed doc's weight is absent from edge
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_5_AC1_RemoveFile_RemovedDocWeightAbsent()
    {
        // T-PROX-001.5: FR-23.5 AC-1
        // Insert an edge with a dict-format source_files containing one URI.
        // After RemoveFile, the edge must be deleted (no docs remain).
        using var conn = OpenTestDb();

        InsertConcept(conn, "alpha");
        InsertConcept(conn, "beta");

        var uri = "memory://tests/to-remove.md";
        InsertEdgeWithProximity(conn, "alpha", "beta", uri, 1.0);

        // Verify edge exists before removal.
        var countBefore = conn.ExecuteScalar<long>(
            "SELECT COUNT(*) FROM concept_graph WHERE (concept_a='alpha' AND concept_b='beta') OR (concept_a='beta' AND concept_b='alpha')");
        Assert.That(countBefore, Is.EqualTo(1), "Precondition: edge must exist.");

        // Also insert concept_mentions to satisfy RemoveFile contract.
        conn.Execute("INSERT OR IGNORE INTO concept_mentions (concept_name, source_file, mention_count) VALUES ('alpha', @f, 1)", new { f = uri });
        conn.Execute("INSERT OR IGNORE INTO concept_mentions (concept_name, source_file, mention_count) VALUES ('beta', @f, 1)", new { f = uri });
        conn.Execute("INSERT OR IGNORE INTO file_content (file_path, title, content, last_indexed, status) VALUES (@f, 'Test', '', '2026-01-01', 'active')", new { f = uri });

        ConceptSync.RemoveFile(conn, uri, vectorReady: false);

        // Edge must be deleted when no URIs remain.
        var countAfter = conn.ExecuteScalar<long>(
            "SELECT COUNT(*) FROM concept_graph WHERE (concept_a='alpha' AND concept_b='beta') OR (concept_a='beta' AND concept_b='alpha')");
        Assert.That(countAfter, Is.EqualTo(0),
            "FR-23.5 AC-1: Edge must be deleted when the last source doc is removed.");
    }

    // ──────────────────────────────────────────────────────────────────────
    // AC-2: After RemoveFile, remaining docs' weights are preserved
    // ──────────────────────────────────────────────────────────────────────
    [Test]
    public void FR23_5_AC2_RemoveFile_RemainingDocsPreserved()
    {
        // T-PROX-001.5: FR-23.5 AC-2
        // Insert an edge with two URIs in the source_files dict.
        // Remove one URI. The edge must remain, with only the other URI + its weight.
        using var conn = OpenTestDb();

        InsertConcept(conn, "alpha");
        InsertConcept(conn, "beta");

        var uriToRemove = "memory://tests/remove-me.md";
        var uriToKeep = "memory://tests/keep-me.md";

        // Insert edge with both URIs in the dict.
        InsertEdgeWithTwoUris(conn, "alpha", "beta", uriToRemove, 1.0, uriToKeep, 0.5);

        // Set up file_content and concept_mentions for the file being removed.
        conn.Execute("INSERT OR IGNORE INTO file_content (file_path, title, content, last_indexed, status) VALUES (@f, 'Remove', '', '2026-01-01', 'active')", new { f = uriToRemove });
        conn.Execute("INSERT OR IGNORE INTO concept_mentions (concept_name, source_file, mention_count) VALUES ('alpha', @f, 1)", new { f = uriToRemove });
        conn.Execute("INSERT OR IGNORE INTO concept_mentions (concept_name, source_file, mention_count) VALUES ('beta', @f, 1)", new { f = uriToRemove });

        ConceptSync.RemoveFile(conn, uriToRemove, vectorReady: false);

        // Edge must still exist (uriToKeep remains).
        var rawJson = conn.QuerySingle<string>(
            "SELECT source_files FROM concept_graph WHERE (concept_a='alpha' AND concept_b='beta') OR (concept_a='beta' AND concept_b='alpha')");

        Assert.That(rawJson, Is.Not.Null.And.Not.Empty,
            "FR-23.5 AC-2: Edge must still exist after removing one of two source docs.");

        using var doc = JsonDocument.Parse(rawJson!);
        Assert.That(doc.RootElement.ValueKind, Is.EqualTo(JsonValueKind.Object),
            "FR-23.5 AC-2: source_files must remain a JSON object after partial removal.");

        // uriToRemove must be gone.
        Assert.That(doc.RootElement.TryGetProperty(uriToRemove, out _), Is.False,
            "FR-23.5 AC-2: Removed URI must not appear in source_files after RemoveFile.");

        // uriToKeep must remain with its original weight.
        Assert.That(doc.RootElement.TryGetProperty(uriToKeep, out var keepWeight), Is.True,
            "FR-23.5 AC-2: Kept URI must still appear in source_files.");
        Assert.That(keepWeight.GetDouble(), Is.EqualTo(0.5).Within(1e-10),
            "FR-23.5 AC-2: Kept URI's proximity weight (0.5) must be preserved exactly.");

        // co_occurrence_count must reflect the remaining doc count (1).
        var count = conn.ExecuteScalar<long>(
            "SELECT co_occurrence_count FROM concept_graph WHERE (concept_a='alpha' AND concept_b='beta') OR (concept_a='beta' AND concept_b='alpha')");
        Assert.That(count, Is.EqualTo(1),
            "FR-23.5 AC-2: co_occurrence_count must be 1 after removing one of two source docs.");
    }

    // ══════════════════════════════════════════════════════════════════════
    // FR-23.6: RunLouvain reads source_files proximity weights (T-PROX-003.2)
    // ══════════════════════════════════════════════════════════════════════

    [Test]
    public void FR23_6_AC1_RunLouvain_ProximityWeightedEdges_ClustersHighSumTogether()
    {
        // T-PROX-003.4: RTM FR-23.6 AC-1
        // After T-PROX-003.2, RunLouvain reads source_files and sums proximity values as edge weight.
        //
        // Topology: two hubs (H-a, H-b) each with 3 satellites.
        //   - H-a — H-b: co_occurrence_count=1, source_files sum=20.0 (the bond we're testing)
        //   - H-a — S1, S2, S3: co_occurrence_count=1, source_files sum=5.0 each
        //   - H-b — S4, S5, S6: co_occurrence_count=1, source_files sum=5.0 each
        //
        // With co_occurrence_count only (current): ALL edges weight=1. Modularity maximization
        // splits the graph as {H-a, S1, S2, S3} vs {H-b, S4, S5, S6}, keeping the hubs
        // with their satellites and IGNORING the H-a/H-b bond. The cross-community H-a—H-b
        // edge has weight=1 just like the satellite edges, so it doesn't overcome the cut.
        // → communities["prox-h-a"] ≠ communities["prox-h-b"]: assertion FAILS.
        //
        // With source_files sums (after fix): H-a—H-b weight=20.0 >> satellite edges (5.0).
        // The strong bond pulls H-a and H-b into the same community.
        // → communities["prox-h-a"] == communities["prox-h-b"]: assertion PASSES.
        //
        // FAILS until SWE implements T-PROX-003.2.
        using var conn = OpenTestDb();

        var strongEdge = JsonSerializer.Serialize(new Dictionary<string, double>
        {
            ["memory://test/prox-h-heavy.md"] = 20.0,
        });
        var satEdge = JsonSerializer.Serialize(new Dictionary<string, double>
        {
            ["memory://test/prox-h-sat.md"] = 5.0,
        });

        // Hubs
        InsertConcept(conn, "prox-h-a");
        InsertConcept(conn, "prox-h-b");

        // Strong bond between hubs (proximity sum=20, co_occurrence_count=1)
        InsertEdgeRawSourceFiles(conn, "prox-h-a", "prox-h-b",
            coOccurrenceCount: 1, sourceFilesJson: strongEdge);

        // H-a satellites (proximity sum=5 each, co_occurrence_count=1)
        foreach (var sat in new[] { "prox-h-s1", "prox-h-s2", "prox-h-s3" })
        {
            InsertConcept(conn, sat);
            InsertEdgeRawSourceFiles(conn, "prox-h-a", sat,
                coOccurrenceCount: 1, sourceFilesJson: satEdge);
        }

        // H-b satellites (proximity sum=5 each, co_occurrence_count=1)
        foreach (var sat in new[] { "prox-h-s4", "prox-h-s5", "prox-h-s6" })
        {
            InsertConcept(conn, sat);
            InsertEdgeRawSourceFiles(conn, "prox-h-b", sat,
                coOccurrenceCount: 1, sourceFilesJson: satEdge);
        }

        var (communities, _) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);

        // After fix: H-a—H-b bond (20.0) dominates satellite bonds (5.0) → same community.
        // Before fix: all edges weight=1 → H-a pulled to its satellites, H-b to its satellites
        //             → H-a and H-b end up in different communities → assertion fails.
        Assert.That(communities["prox-h-a"], Is.EqualTo(communities["prox-h-b"]),
            "FR-23.6 AC-1: Strong H-a—H-b bond (source_files sum=20.0) must overcome the satellite " +
            "pull and place both hubs in the same community. Fails before T-PROX-003.2 because " +
            "all co_occurrence_count=1 edges are treated equally.");
    }

    [Test]
    public void FR23_6_AC2_RunLouvain_EmptySourceFiles_TreatedAsIsolated()
    {
        // T-PROX-003.4: RTM FR-23.6 AC-2
        // After T-PROX-003.2, an edge with source_files='{}' yields proximity sum = 0.0.
        // The two nodes connected only by that edge must be treated as isolated (no real connection).
        // FAILS until T-PROX-003.2 because currently RunLouvain reads co_occurrence_count=5,
        // which strongly connects isolated-x/y, putting them in the same community.
        using var conn = OpenTestDb();

        // Anchor pair: real proximity connection (sum = 1.0)
        InsertConcept(conn, "prox-anchor-a");
        InsertConcept(conn, "prox-anchor-b");
        InsertEdgeWithProximity(conn, "prox-anchor-a", "prox-anchor-b",
            "memory://test/anchor.md", 1.0);

        // Isolated pair: edge with empty source_files but high co_occurrence_count.
        // After fix: proximity sum = 0.0 → each node isolated in its own community.
        // Before fix: co_occurrence_count = 5 → strongly connected → same community.
        InsertConcept(conn, "prox-isolated-x");
        InsertConcept(conn, "prox-isolated-y");
        InsertEdgeRawSourceFiles(conn, "prox-isolated-x", "prox-isolated-y",
            coOccurrenceCount: 5, sourceFilesJson: "{}");

        var (communities, _) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);

        Assert.That(communities["prox-isolated-x"], Is.Not.EqualTo(communities["prox-isolated-y"]),
            "FR-23.6 AC-2: Edge with empty source_files must yield weight 0.0; nodes must be isolated in separate communities.");
    }

    // ══════════════════════════════════════════════════════════════════════
    // FR-23.7: NMI computation + RunAndPersist drift warning (T-PROX-003.3)
    // ══════════════════════════════════════════════════════════════════════

    [Test]
    public void FR23_7_AC3_ComputeNMI_IdenticalClusterings_Returns1()
    {
        // T-PROX-003.4: RTM FR-23.7 AC-3
        // NMI of identical clusterings must equal 1.0.
        // FAILS until T-PROX-003.3 adds CommunityDetection.ComputeNMI.
        var clustering = new Dictionary<string, int>
        {
            ["alpha"] = 0,
            ["beta"]  = 0,
            ["gamma"] = 1,
            ["delta"] = 1,
        };

        double nmi = CommunityDetection.ComputeNMI(clustering, clustering);

        Assert.That(nmi, Is.EqualTo(1.0).Within(1e-10),
            "FR-23.7 AC-3: NMI of a clustering with itself must be exactly 1.0.");
    }

    [Test]
    public void FR23_7_AC4_ComputeNMI_DifferentClusterings_LessThan1()
    {
        // T-PROX-003.4: RTM FR-23.7 AC-4
        // NMI of two distinct, non-identical clusterings must be strictly less than 1.0.
        // FAILS until T-PROX-003.3 adds CommunityDetection.ComputeNMI.
        var clusteringA = new Dictionary<string, int>
        {
            ["alpha"] = 0,
            ["beta"]  = 0,
            ["gamma"] = 1,
            ["delta"] = 1,
        };

        // Genuinely different partition: {{alpha, gamma}, {beta, delta}}
        // (NMI is label-invariant — swapping 0↔1 labels produces the same partition,
        //  so we use a structurally different assignment instead.)
        var clusteringB = new Dictionary<string, int>
        {
            ["alpha"] = 0,
            ["beta"]  = 1,  // Moved to different community
            ["gamma"] = 0,  // Moved to different community
            ["delta"] = 1,
        };

        double nmi = CommunityDetection.ComputeNMI(clusteringA, clusteringB);

        Assert.That(nmi, Is.LessThan(1.0),
            "FR-23.7 AC-4: NMI of two different clusterings must be strictly less than 1.0.");
        Assert.That(nmi, Is.GreaterThanOrEqualTo(0.0),
            "FR-23.7 AC-4: NMI must be in [0, 1] range.");
    }

    [Test]
    public void FR23_7_AC5_RunAndPersist_LogsWarning_WhenCommunityDriftExceedsThreshold()
    {
        // T-PROX-003.4: RTM FR-23.7 AC-5
        // After T-PROX-003.3, RunAndPersist computes NMI between old and new communities.
        // When NMI falls below the drift threshold it must emit a warning to Console.Error.
        // FAILS until T-PROX-003.3 adds NMI tracking + warning logic to RunAndPersist.
        using var conn = OpenTestDb();

        // --- Topology A: two tight cliques (a0/a1/a2 and b0/b1/b2) ---
        foreach (var name in new[] { "drift-a0", "drift-a1", "drift-a2" })
            InsertConcept(conn, name);
        InsertEdgeWithProximity(conn, "drift-a0", "drift-a1", "memory://test/drift-a.md", 1.0);
        InsertEdgeWithProximity(conn, "drift-a1", "drift-a2", "memory://test/drift-a.md", 1.0);
        InsertEdgeWithProximity(conn, "drift-a0", "drift-a2", "memory://test/drift-a.md", 1.0);

        foreach (var name in new[] { "drift-b0", "drift-b1", "drift-b2" })
            InsertConcept(conn, name);
        InsertEdgeWithProximity(conn, "drift-b0", "drift-b1", "memory://test/drift-b.md", 1.0);
        InsertEdgeWithProximity(conn, "drift-b1", "drift-b2", "memory://test/drift-b.md", 1.0);
        InsertEdgeWithProximity(conn, "drift-b0", "drift-b2", "memory://test/drift-b.md", 1.0);

        // First run: establishes the baseline community partition.
        CommunityDetection.RunAndPersist(conn, gamma: 1.0, seed: 42);

        // --- Topology B: completely rewired — a-nodes now connect to b-nodes ---
        conn.Execute("DELETE FROM concept_graph");
        InsertEdgeWithProximity(conn, "drift-a0", "drift-b0", "memory://test/drift-cross.md", 1.0);
        InsertEdgeWithProximity(conn, "drift-a1", "drift-b1", "memory://test/drift-cross.md", 1.0);
        InsertEdgeWithProximity(conn, "drift-a2", "drift-b2", "memory://test/drift-cross.md", 1.0);

        // Second run: should detect NMI drop and emit a drift warning.
        var stderr = new StringBuilder();
        var previousStderr = Console.Error;
        using var capture = new StringWriter(stderr, CultureInfo.InvariantCulture);
        Console.SetError(capture);

        try
        {
            CommunityDetection.RunAndPersist(conn, gamma: 1.0, seed: 42);
        }
        finally
        {
            Console.SetError(previousStderr);
        }

        Assert.That(stderr.ToString(), Does.Contain("drift").Or.Contain("DRIFT").Or.Contain("NMI"),
            "FR-23.7 AC-5: RunAndPersist must emit a drift warning to Console.Error when community NMI drops below threshold.");
    }

    // ══════════════════════════════════════════════════════════════════════
    // Helpers
    // ══════════════════════════════════════════════════════════════════════

    private static SqliteConnection OpenTestDb()
    {
        var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();
        conn.Execute("PRAGMA foreign_keys=ON");
        return conn;
    }

    private static void InsertConcept(SqliteConnection conn, string name)
    {
        conn.Execute(
            "INSERT OR IGNORE INTO concepts (concept_name, first_seen) VALUES (@name, '2026-01-01')",
            new { name });
    }

    /// <summary>
    /// Insert a proximity-weighted edge: source_files = {"uri": weight}, co_occurrence_count = 1.
    /// </summary>
    private static void InsertEdgeWithProximity(SqliteConnection conn, string a, string b, string uri, double weight)
    {
        var (sortedA, sortedB) = string.CompareOrdinal(a, b) < 0 ? (a, b) : (b, a);
        var sourceFiles = JsonSerializer.Serialize(new Dictionary<string, double> { [uri] = weight });
        conn.Execute(
            @"INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
              VALUES (@a, @b, 1, @files)",
            new { a = sortedA, b = sortedB, files = sourceFiles });
    }

    /// <summary>
    /// Insert an edge with two URIs in the source_files dict.
    /// co_occurrence_count = 2 (number of distinct docs).
    /// </summary>
    private static void InsertEdgeWithTwoUris(
        SqliteConnection conn, string a, string b,
        string uri1, double weight1,
        string uri2, double weight2)
    {
        var (sortedA, sortedB) = string.CompareOrdinal(a, b) < 0 ? (a, b) : (b, a);
        var sourceFiles = JsonSerializer.Serialize(new Dictionary<string, double>
        {
            [uri1] = weight1,
            [uri2] = weight2,
        });
        conn.Execute(
            @"INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
              VALUES (@a, @b, 2, @files)",
            new { a = sortedA, b = sortedB, files = sourceFiles });
    }

    /// <summary>
    /// Insert an edge with an explicit source_files string and co_occurrence_count.
    /// Use this to test how RunLouvain handles edge cases like empty source_files ('{}'
    /// with a non-zero co_occurrence_count that would mislead the pre-T-PROX-003.2 reader).
    /// </summary>
    private static void InsertEdgeRawSourceFiles(
        SqliteConnection conn, string a, string b,
        int coOccurrenceCount, string sourceFilesJson)
    {
        var (sortedA, sortedB) = string.CompareOrdinal(a, b) < 0 ? (a, b) : (b, a);
        conn.Execute(
            @"INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
              VALUES (@a, @b, @count, @files)",
            new { a = sortedA, b = sortedB, count = coOccurrenceCount, files = sourceFilesJson });
    }

    /// <summary>
    /// Read the proximity weight stored for a specific URI in source_files dict.
    /// Returns (rawJson, weight).
    /// </summary>
    private static (string rawJson, double weight) GetEdgeWeight(
        SqliteConnection conn, string a, string b, string uri)
    {
        var (sortedA, sortedB) = string.CompareOrdinal(a, b) < 0 ? (a, b) : (b, a);
        var rawJson = conn.QuerySingle<string>(
            "SELECT source_files FROM concept_graph WHERE concept_a = @a AND concept_b = @b",
            new { a = sortedA, b = sortedB }) ?? string.Empty;

        using var doc = JsonDocument.Parse(rawJson);
        Assert.That(doc.RootElement.ValueKind, Is.EqualTo(JsonValueKind.Object),
            $"source_files for edge ({a},{b}) must be a JSON object.");
        Assert.That(doc.RootElement.TryGetProperty(uri, out var weightElem), Is.True,
            $"URI '{uri}' must be a key in source_files for edge ({a},{b}).");

        return (rawJson, weightElem.GetDouble());
    }
}
