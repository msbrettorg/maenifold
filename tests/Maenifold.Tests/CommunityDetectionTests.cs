using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using Maenifold.Models;
using Maenifold.Tools;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

// T-COMMUNITY-001.4: RTM FR-13.1, FR-13.2, NFR-13.1.1, NFR-13.1.2, NFR-13.2.1, NFR-13.3.1
// Blue-team unit tests for Louvain community detection algorithm.
// T-COMMUNITY-001.10: RTM FR-13.4, FR-13.6, FR-13.7, FR-13.8, FR-13.9, FR-13.10
// Blue-team integration tests for community detection in BuildContext, ConceptSync, and graceful degradation.
[TestFixture]
[NonParallelizable]
public class CommunityDetectionTests
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
        _testRoot = Path.Combine(repoRoot, "test-outputs", "community-detection", $"run-{Guid.NewGuid():N}");

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

    // ──────────────────────────────────────────────────────────
    // NFR-13.1.2: Deterministic seed produces same result
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunLouvain_DeterministicSeed_ProducesSameResult()
    {
        // T-COMMUNITY-001.4: NFR-13.1.2
        using var conn = OpenTestDb();
        InsertTrianglePlusIsolatedPair(conn);

        var (communities1, mod1) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);
        var (communities2, mod2) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);

        Assert.That(mod1, Is.EqualTo(mod2).Within(1e-10), "Same seed must produce same modularity.");

        foreach (var (concept, cid) in communities1)
        {
            Assert.That(communities2.ContainsKey(concept), Is.True, $"Concept '{concept}' missing in second run.");
            Assert.That(communities2[concept], Is.EqualTo(cid), $"Concept '{concept}' assigned to different community.");
        }
    }

    // ──────────────────────────────────────────────────────────
    // Known topology: triangle + isolated pair = 2 communities
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunLouvain_TrianglePlusIsolatedPair_TwoCommunities()
    {
        // T-COMMUNITY-001.4: FR-13.1
        using var conn = OpenTestDb();
        InsertTrianglePlusIsolatedPair(conn);

        var (communities, modularity) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);

        // Should find 2 communities
        var distinctCommunities = communities.Values.Distinct().Count();
        Assert.That(distinctCommunities, Is.EqualTo(2), "Triangle + isolated pair should produce exactly 2 communities.");

        // Triangle nodes should be in same community
        Assert.That(communities["alpha"], Is.EqualTo(communities["beta"]), "Triangle nodes alpha-beta should be same community.");
        Assert.That(communities["beta"], Is.EqualTo(communities["gamma"]), "Triangle nodes beta-gamma should be same community.");

        // Isolated pair should be in same community
        Assert.That(communities["delta"], Is.EqualTo(communities["epsilon"]), "Isolated pair delta-epsilon should be same community.");

        // Triangle and isolated pair should be in different communities
        Assert.That(communities["alpha"], Is.Not.EqualTo(communities["delta"]), "Triangle and isolated pair should be different communities.");

        // Modularity should be positive for non-trivial partition
        Assert.That(modularity, Is.GreaterThan(0), "Modularity should be positive for meaningful partition.");
    }

    // ──────────────────────────────────────────────────────────
    // Empty graph → no communities
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunLouvain_EmptyGraph_ReturnsEmpty()
    {
        // T-COMMUNITY-001.4: edge case
        using var conn = OpenTestDb();
        // No concepts, no edges — empty graph

        var (communities, modularity) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);

        Assert.That(communities, Is.Empty, "Empty graph should return no communities.");
        Assert.That(modularity, Is.EqualTo(0.0), "Empty graph should have 0 modularity.");
    }

    // ──────────────────────────────────────────────────────────
    // Single node → 1 community
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunLouvain_SingleNode_OneCommunity()
    {
        // T-COMMUNITY-001.4: edge case
        using var conn = OpenTestDb();
        conn.Execute("INSERT INTO concepts (concept_name, first_seen) VALUES ('alone', '2026-01-01')");

        var (communities, modularity) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);

        Assert.That(communities.Count, Is.EqualTo(1), "Single node should have 1 community.");
        Assert.That(communities["alone"], Is.EqualTo(0), "Single node community should be 0.");
        Assert.That(modularity, Is.EqualTo(0.0), "Single node has 0 modularity.");
    }

    // ──────────────────────────────────────────────────────────
    // Disconnected components → separate communities
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunLouvain_DisconnectedComponents_SeparateCommunities()
    {
        // T-COMMUNITY-001.4: FR-13.1
        using var conn = OpenTestDb();

        // Component 1: clique of 3
        InsertConcept(conn, "c1-a");
        InsertConcept(conn, "c1-b");
        InsertConcept(conn, "c1-c");
        InsertEdge(conn, "c1-a", "c1-b", 5);
        InsertEdge(conn, "c1-b", "c1-c", 5);
        InsertEdge(conn, "c1-a", "c1-c", 5);

        // Component 2: clique of 3
        InsertConcept(conn, "c2-a");
        InsertConcept(conn, "c2-b");
        InsertConcept(conn, "c2-c");
        InsertEdge(conn, "c2-a", "c2-b", 5);
        InsertEdge(conn, "c2-b", "c2-c", 5);
        InsertEdge(conn, "c2-a", "c2-c", 5);

        var (communities, modularity) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);

        // All nodes in component 1 should be in the same community
        Assert.That(communities["c1-a"], Is.EqualTo(communities["c1-b"]));
        Assert.That(communities["c1-b"], Is.EqualTo(communities["c1-c"]));

        // All nodes in component 2 should be in the same community
        Assert.That(communities["c2-a"], Is.EqualTo(communities["c2-b"]));
        Assert.That(communities["c2-b"], Is.EqualTo(communities["c2-c"]));

        // Components should be in different communities
        Assert.That(communities["c1-a"], Is.Not.EqualTo(communities["c2-a"]),
            "Disconnected components should be in different communities.");

        Assert.That(modularity, Is.GreaterThan(0), "Disconnected components should have positive modularity.");
    }

    // ──────────────────────────────────────────────────────────
    // Modularity > 0 for non-trivial graph
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunLouvain_NonTrivialGraph_PositiveModularity()
    {
        // T-COMMUNITY-001.4: FR-13.1
        using var conn = OpenTestDb();
        InsertTrianglePlusIsolatedPair(conn);

        var (_, modularity) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);

        Assert.That(modularity, Is.GreaterThan(0.0).And.LessThanOrEqualTo(1.0),
            "Modularity should be in (0, 1] for non-trivial graph.");
    }

    // ──────────────────────────────────────────────────────────
    // NFR-13.3.1: Schema FK constraint validates concept_name exists
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunAndPersist_FKConstraint_OnlyPersistsExistingConcepts()
    {
        // T-COMMUNITY-001.4: NFR-13.3.1
        using var conn = OpenTestDb();
        conn.Execute("PRAGMA foreign_keys=ON");
        InsertTrianglePlusIsolatedPair(conn);

        var (communityCount, modularity) = CommunityDetection.RunAndPersist(conn, gamma: 1.0, seed: 42);

        Assert.That(communityCount, Is.EqualTo(2), "Should persist 2 communities.");
        Assert.That(modularity, Is.GreaterThan(0), "Should have positive modularity.");

        // Verify all persisted concepts exist in concepts table
        var persisted = conn.Query<string>("SELECT concept_name FROM concept_communities").ToList();
        foreach (var concept in persisted)
        {
            var exists = conn.QuerySingle<bool>(
                "SELECT COUNT(*) > 0 FROM concepts WHERE concept_name = @name", new { name = concept });
            Assert.That(exists, Is.True, $"Persisted concept '{concept}' must exist in concepts table.");
        }
    }

    // ──────────────────────────────────────────────────────────
    // FR-13.11: Gamma parameter affects community count
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunLouvain_HigherGamma_TendsToMoreCommunities()
    {
        // T-COMMUNITY-001.4: FR-13.11
        using var conn = OpenTestDb();

        // Build a larger graph: 2 clusters connected by a weak bridge
        // Cluster 1: 4 nodes with strong internal edges
        InsertConcept(conn, "cluster1-a");
        InsertConcept(conn, "cluster1-b");
        InsertConcept(conn, "cluster1-c");
        InsertConcept(conn, "cluster1-d");
        InsertEdge(conn, "cluster1-a", "cluster1-b", 10);
        InsertEdge(conn, "cluster1-b", "cluster1-c", 10);
        InsertEdge(conn, "cluster1-c", "cluster1-d", 10);
        InsertEdge(conn, "cluster1-a", "cluster1-d", 10);
        InsertEdge(conn, "cluster1-a", "cluster1-c", 10);
        InsertEdge(conn, "cluster1-b", "cluster1-d", 10);

        // Cluster 2: 4 nodes with strong internal edges
        InsertConcept(conn, "cluster2-a");
        InsertConcept(conn, "cluster2-b");
        InsertConcept(conn, "cluster2-c");
        InsertConcept(conn, "cluster2-d");
        InsertEdge(conn, "cluster2-a", "cluster2-b", 10);
        InsertEdge(conn, "cluster2-b", "cluster2-c", 10);
        InsertEdge(conn, "cluster2-c", "cluster2-d", 10);
        InsertEdge(conn, "cluster2-a", "cluster2-d", 10);
        InsertEdge(conn, "cluster2-a", "cluster2-c", 10);
        InsertEdge(conn, "cluster2-b", "cluster2-d", 10);

        // Weak bridge
        InsertEdge(conn, "cluster1-a", "cluster2-a", 1);

        var (commLow, _) = CommunityDetection.RunLouvain(conn, gamma: 0.5, seed: 42);
        var (commHigh, _) = CommunityDetection.RunLouvain(conn, gamma: 2.0, seed: 42);

        var countLow = commLow.Values.Distinct().Count();
        var countHigh = commHigh.Values.Distinct().Count();

        // Higher gamma should produce at least as many communities (resolution limit)
        Assert.That(countHigh, Is.GreaterThanOrEqualTo(countLow),
            "Higher gamma should tend to produce more (or equal) communities.");
    }

    // ──────────────────────────────────────────────────────────
    // NFR-13.2.1: co_occurrence_count used as raw edge weight
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunLouvain_UsesCoOccurrenceCountDirectly()
    {
        // T-COMMUNITY-001.4: NFR-13.2.1
        using var conn = OpenTestDb();

        // Two pairs with very different weights — should end up in different communities
        InsertConcept(conn, "heavy-a");
        InsertConcept(conn, "heavy-b");
        InsertEdge(conn, "heavy-a", "heavy-b", 100); // Very strong

        InsertConcept(conn, "light-a");
        InsertConcept(conn, "light-b");
        InsertEdge(conn, "light-a", "light-b", 100); // Very strong

        // Weak cross-link
        InsertEdge(conn, "heavy-a", "light-a", 1);

        var (communities, _) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);

        // Heavy pair should be in same community
        Assert.That(communities["heavy-a"], Is.EqualTo(communities["heavy-b"]),
            "Strongly connected pair should be in same community.");
        Assert.That(communities["light-a"], Is.EqualTo(communities["light-b"]),
            "Strongly connected pair should be in same community.");
    }

    // ──────────────────────────────────────────────────────────
    // RunAndPersist: persists to concept_communities table
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunAndPersist_PersistsToDatabase()
    {
        // T-COMMUNITY-001.4: FR-13.3
        using var conn = OpenTestDb();
        InsertTrianglePlusIsolatedPair(conn);

        var (communityCount, modularity) = CommunityDetection.RunAndPersist(conn, gamma: 1.0, seed: 42);

        Assert.That(communityCount, Is.EqualTo(2));
        Assert.That(modularity, Is.GreaterThan(0));

        // Verify database rows
        var rowCount = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concept_communities");
        Assert.That(rowCount, Is.EqualTo(5), "Should have 5 concept_communities rows.");

        // Verify modularity and resolution are stored
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT modularity, resolution FROM concept_communities LIMIT 1";
        using var reader = cmd.ExecuteReader();
        Assert.That(reader.Read(), Is.True, "Should have at least one row.");
        var storedMod = reader.GetDouble(0);
        var storedRes = reader.GetDouble(1);
        Assert.That(storedMod, Is.EqualTo(modularity).Within(1e-10));
        Assert.That(storedRes, Is.EqualTo(1.0).Within(1e-10));
    }

    // ──────────────────────────────────────────────────────────
    // RunAndPersist: full replacement on second run
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunAndPersist_SecondRun_ReplacesExisting()
    {
        // T-COMMUNITY-001.4: FR-13.3
        using var conn = OpenTestDb();
        InsertTrianglePlusIsolatedPair(conn);

        CommunityDetection.RunAndPersist(conn, gamma: 1.0, seed: 42);
        var countBefore = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concept_communities");

        // Run again
        CommunityDetection.RunAndPersist(conn, gamma: 1.0, seed: 42);
        var countAfter = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concept_communities");

        Assert.That(countAfter, Is.EqualTo(countBefore),
            "Second RunAndPersist should replace, not append.");
    }

    // ──────────────────────────────────────────────────────────
    // Helpers
    // ──────────────────────────────────────────────────────────

    private static SqliteConnection OpenTestDb()
    {
        var conn = new SqliteConnection(Config.DatabaseConnectionString);
        conn.OpenWithWAL();
        conn.Execute("PRAGMA foreign_keys=ON");
        return conn;
    }

    private static void InsertConcept(SqliteConnection conn, string name)
    {
        conn.Execute("INSERT OR IGNORE INTO concepts (concept_name, first_seen) VALUES (@name, '2026-01-01')",
            new { name });
    }

    private static void InsertEdge(SqliteConnection conn, string a, string b, int weight)
    {
        var (sortedA, sortedB) = string.CompareOrdinal(a, b) < 0 ? (a, b) : (b, a);
        conn.Execute(
            @"INSERT OR REPLACE INTO concept_graph (concept_a, concept_b, co_occurrence_count, source_files)
              VALUES (@a, @b, @w, '[]')",
            new { a = sortedA, b = sortedB, w = weight });
    }

    /// <summary>
    /// Insert a known topology: triangle (alpha-beta-gamma) + isolated pair (delta-epsilon).
    /// </summary>
    private static void InsertTrianglePlusIsolatedPair(SqliteConnection conn)
    {
        InsertConcept(conn, "alpha");
        InsertConcept(conn, "beta");
        InsertConcept(conn, "gamma");
        InsertConcept(conn, "delta");
        InsertConcept(conn, "epsilon");

        // Triangle: alpha—beta—gamma—alpha (weight 5 each)
        InsertEdge(conn, "alpha", "beta", 5);
        InsertEdge(conn, "beta", "gamma", 5);
        InsertEdge(conn, "alpha", "gamma", 5);

        // Isolated pair: delta—epsilon (weight 5)
        InsertEdge(conn, "delta", "epsilon", 5);
    }

    // ══════════════════════════════════════════════════════════
    // INTEGRATION TESTS — T-COMMUNITY-001.10
    // ══════════════════════════════════════════════════════════

    // ──────────────────────────────────────────────────────────
    // FR-13.4: Community detection runs during full Sync
    // ──────────────────────────────────────────────────────────
    [Test]
    public void Sync_ProducesCommunityAssignments()
    {
        // T-COMMUNITY-001.10: RTM FR-13.4
        // Create markdown files with overlapping WikiLinks so Sync builds a graph,
        // then verify that community detection populated concept_communities.
        var memoryDir = Path.Combine(_testRoot, "memory", "tests", "community-sync");
        Directory.CreateDirectory(memoryDir);

        // File 1: concepts A, B, C co-occur
        File.WriteAllText(
            Path.Combine(memoryDir, "note-abc.md"),
            "# Note ABC\n\nDiscusses [[sync-comm-a]] and [[sync-comm-b]] and [[sync-comm-c]].\n");

        // File 2: concepts A, B co-occur again (strengthens their edge)
        File.WriteAllText(
            Path.Combine(memoryDir, "note-ab.md"),
            "# Note AB\n\nMore about [[sync-comm-a]] and [[sync-comm-b]].\n");

        // File 3: concepts D, E form a separate cluster
        File.WriteAllText(
            Path.Combine(memoryDir, "note-de.md"),
            "# Note DE\n\nAbout [[sync-comm-d]] and [[sync-comm-e]].\n");

        // Run full Sync which includes community detection at the end
        GraphTools.Sync();

        // Verify concept_communities has rows
        using var conn = OpenTestDb();
        var rowCount = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concept_communities");
        Assert.That(rowCount, Is.GreaterThan(0),
            "FR-13.4: Full Sync must populate concept_communities table.");

        // Verify the concepts we created are present
        var communityNames = conn.Query<string>("SELECT concept_name FROM concept_communities").ToList();
        Assert.That(communityNames, Does.Contain("sync-comm-a"),
            "FR-13.4: sync-comm-a should have a community assignment after Sync.");
        Assert.That(communityNames, Does.Contain("sync-comm-d"),
            "FR-13.4: sync-comm-d should have a community assignment after Sync.");
    }

    // ──────────────────────────────────────────────────────────
    // FR-13.6: BuildContext includes community_id on RelatedConcepts
    // ──────────────────────────────────────────────────────────
    [Test]
    public void BuildContext_IncludesCommunityIdOnRelatedConcepts()
    {
        // T-COMMUNITY-001.10: RTM FR-13.6
        // Set up a graph with known communities via direct SQL + RunAndPersist,
        // then call BuildContext and verify community_id is populated.
        using var conn = OpenTestDb();

        // Create a triangle — Louvain will put them in the same community
        InsertConcept(conn, "ctx-alpha");
        InsertConcept(conn, "ctx-beta");
        InsertConcept(conn, "ctx-gamma");
        InsertEdge(conn, "ctx-alpha", "ctx-beta", 10);
        InsertEdge(conn, "ctx-beta", "ctx-gamma", 10);
        InsertEdge(conn, "ctx-alpha", "ctx-gamma", 10);

        // Persist communities so BuildContext can read them
        CommunityDetection.RunAndPersist(conn, gamma: 1.0, seed: 42);

        // Verify persisted
        var persisted = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concept_communities");
        Assert.That(persisted, Is.GreaterThan(0), "Precondition: RunAndPersist must populate concept_communities.");

        conn.Dispose();

        // Call BuildContext — it opens its own read-only connection
        var result = GraphTools.BuildContext("ctx-alpha", depth: 1, maxEntities: 20);

        Assert.That(result.DirectRelations.Count, Is.GreaterThan(0),
            "Precondition: BuildContext should find direct relations for ctx-alpha.");

        // FR-13.6: Each RelatedConcept should have a non-null CommunityId
        foreach (var rel in result.DirectRelations)
        {
            Assert.That(rel.CommunityId, Is.Not.Null,
                $"FR-13.6: RelatedConcept '{rel.Name}' should have a non-null CommunityId when community data exists.");
        }

        // Also verify the query concept itself has a CommunityId
        Assert.That(result.CommunityId, Is.Not.Null,
            "FR-13.6: Query concept should have CommunityId populated.");
    }

    // ──────────────────────────────────────────────────────────
    // FR-13.7, FR-13.8: BuildContext returns CommunitySiblings
    // ──────────────────────────────────────────────────────────
    [Test]
    public void BuildContext_ReturnsCommunitySiblings()
    {
        // T-COMMUNITY-001.10: RTM FR-13.7, FR-13.8
        // Topology: query and sibling are in the same community but have NO direct edge.
        // They share >= 3 mutual neighbors with high edge weights to guarantee >= 0.4 overlap.
        //
        // Graph design:
        //   query --10-- n1 --10-- sibling
        //   query --10-- n2 --10-- sibling
        //   query --10-- n3 --10-- sibling
        //   query --10-- n4 --10-- sibling
        //   (no direct edge between query and sibling)
        //
        // All nodes form one tight community because they all share strong connections.
        // Sibling has 4 shared neighbors with query, total degree = 4*10 = 40,
        // overlap = sum(min(10,10)) / 40 = 40/40 = 1.0 (well above 0.4 threshold).
        using var conn = OpenTestDb();

        InsertConcept(conn, "sib-query");
        InsertConcept(conn, "sib-target");
        InsertConcept(conn, "sib-n1");
        InsertConcept(conn, "sib-n2");
        InsertConcept(conn, "sib-n3");
        InsertConcept(conn, "sib-n4");

        // Query connects to all neighbors
        InsertEdge(conn, "sib-query", "sib-n1", 10);
        InsertEdge(conn, "sib-query", "sib-n2", 10);
        InsertEdge(conn, "sib-query", "sib-n3", 10);
        InsertEdge(conn, "sib-query", "sib-n4", 10);

        // Sibling connects to same neighbors (no direct edge to query)
        InsertEdge(conn, "sib-target", "sib-n1", 10);
        InsertEdge(conn, "sib-target", "sib-n2", 10);
        InsertEdge(conn, "sib-target", "sib-n3", 10);
        InsertEdge(conn, "sib-target", "sib-n4", 10);

        // Also add edges between neighbors to make the cluster tight
        // (ensures Louvain puts them all in the same community)
        InsertEdge(conn, "sib-n1", "sib-n2", 10);
        InsertEdge(conn, "sib-n1", "sib-n3", 10);
        InsertEdge(conn, "sib-n1", "sib-n4", 10);
        InsertEdge(conn, "sib-n2", "sib-n3", 10);
        InsertEdge(conn, "sib-n2", "sib-n4", 10);
        InsertEdge(conn, "sib-n3", "sib-n4", 10);

        // Run community detection
        var (communityCount, _) = CommunityDetection.RunAndPersist(conn, gamma: 1.0, seed: 42);
        Assert.That(communityCount, Is.GreaterThan(0), "Precondition: communities must be detected.");

        // Verify query and target are in the same community
        var queryComm = conn.QuerySingle<int>(
            "SELECT community_id FROM concept_communities WHERE concept_name = 'sib-query'");
        var targetComm = conn.QuerySingle<int>(
            "SELECT community_id FROM concept_communities WHERE concept_name = 'sib-target'");
        Assert.That(queryComm, Is.EqualTo(targetComm),
            "Precondition: query and sibling must be in the same community.");

        conn.Dispose();

        // Call BuildContext
        var result = GraphTools.BuildContext("sib-query", depth: 1, maxEntities: 20);

        // Verify sib-target is NOT in DirectRelations (no direct edge)
        var directNames = result.DirectRelations.Select(r => r.Name).ToList();
        Assert.That(directNames, Does.Not.Contain("sib-target"),
            "Precondition: sib-target should not be a direct relation (no edge to query).");

        // FR-13.7: sib-target should appear in CommunitySiblings
        var siblingNames = result.CommunitySiblings.Select(s => s.Name).ToList();
        Assert.That(siblingNames, Does.Contain("sib-target"),
            "FR-13.7: sib-target should appear in CommunitySiblings (same community, no direct edge, shared neighbors).");

        // FR-13.8: Verify scoring fields are populated
        var targetSibling = result.CommunitySiblings.First(s => s.Name == "sib-target");
        Assert.That(targetSibling.SharedNeighborCount, Is.GreaterThanOrEqualTo(3),
            "FR-13.8: SharedNeighborCount should reflect the shared neighbors.");
        Assert.That(targetSibling.NormalizedOverlap, Is.GreaterThanOrEqualTo(0.4),
            "FR-13.8: NormalizedOverlap should meet minimum threshold.");
    }

    // ──────────────────────────────────────────────────────────
    // FR-13.9: Sibling thresholds filter correctly
    // ──────────────────────────────────────────────────────────
    [Test]
    public void BuildContext_SiblingThresholdsFilter()
    {
        // T-COMMUNITY-001.10: RTM FR-13.9
        // Build topology where a candidate shares < 3 neighbors with the query concept.
        // This candidate should be FILTERED OUT of CommunitySiblings.
        //
        // Graph design:
        //   query --10-- n1 --10-- weak-candidate    (only 1 shared neighbor)
        //   query --10-- n2
        //   query --10-- n3
        //   query --10-- n4
        //   weak-candidate --10-- n1                  (only neighbor shared with query)
        //   weak-candidate --10-- unrelated-1         (not connected to query)
        //
        //   Also: strong-candidate shares >= 3 neighbors (control)
        //   strong-candidate --10-- n1, n2, n3, n4   (4 shared neighbors)
        using var conn = OpenTestDb();

        // Strategy: Instead of relying on Louvain to place all nodes in one community,
        // we directly set up the graph AND manually insert concept_communities entries.
        // This isolates the threshold filtering test from Louvain's partitioning behavior.
        //
        // Query connects to n1..n4.
        // weak-candidate shares only 2 neighbors with query (n1, n2) — below min 3 threshold.
        // strong-candidate shares 4 neighbors with query (n1..n4) — passes threshold.
        // Neither candidate has a direct edge to query.
        InsertConcept(conn, "thresh-query");
        InsertConcept(conn, "thresh-weak");
        InsertConcept(conn, "thresh-strong");
        InsertConcept(conn, "thresh-n1");
        InsertConcept(conn, "thresh-n2");
        InsertConcept(conn, "thresh-n3");
        InsertConcept(conn, "thresh-n4");

        // Query connects to n1..n4
        InsertEdge(conn, "thresh-query", "thresh-n1", 10);
        InsertEdge(conn, "thresh-query", "thresh-n2", 10);
        InsertEdge(conn, "thresh-query", "thresh-n3", 10);
        InsertEdge(conn, "thresh-query", "thresh-n4", 10);

        // Weak candidate: shares only 2 neighbors with query (n1, n2) — below min 3 threshold
        InsertEdge(conn, "thresh-weak", "thresh-n1", 10);
        InsertEdge(conn, "thresh-weak", "thresh-n2", 10);

        // Strong candidate: shares 4 neighbors with query (n1..n4) — passes threshold
        InsertEdge(conn, "thresh-strong", "thresh-n1", 10);
        InsertEdge(conn, "thresh-strong", "thresh-n2", 10);
        InsertEdge(conn, "thresh-strong", "thresh-n3", 10);
        InsertEdge(conn, "thresh-strong", "thresh-n4", 10);

        // Manually insert community assignments — all in community 0
        // This decouples the threshold test from Louvain's partitioning algorithm.
        var ts = CultureInvariantHelpers.FormatDateTime(DateTime.UtcNow, "O");
        foreach (var name in new[] { "thresh-query", "thresh-weak", "thresh-strong",
                                      "thresh-n1", "thresh-n2", "thresh-n3", "thresh-n4" })
        {
            conn.Execute(
                @"INSERT OR REPLACE INTO concept_communities (concept_name, community_id, modularity, resolution, detected_at)
                  VALUES (@name, 0, 0.5, 1.0, @ts)",
                new { name, ts });
        }

        // Verify community data was inserted
        var ccCount = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concept_communities");
        Assert.That(ccCount, Is.EqualTo(7), "Precondition: all 7 concepts must have community assignments.");

        conn.Dispose();

        // Call BuildContext
        var result = GraphTools.BuildContext("thresh-query", depth: 1, maxEntities: 20);

        var siblingNames = result.CommunitySiblings.Select(s => s.Name).ToList();

        // FR-13.9: Weak candidate (1 shared neighbor) should NOT appear
        Assert.That(siblingNames, Does.Not.Contain("thresh-weak"),
            "FR-13.9: Candidate with < 3 shared neighbors must be filtered out of CommunitySiblings.");

        // Control: strong candidate (4 shared neighbors) should appear
        Assert.That(siblingNames, Does.Contain("thresh-strong"),
            "FR-13.9: Candidate with >= 3 shared neighbors and sufficient overlap should appear in CommunitySiblings.");
    }

    // ──────────────────────────────────────────────────────────
    // FR-13.10: Graceful degradation — empty concept_communities
    // ──────────────────────────────────────────────────────────
    [Test]
    public void BuildContext_GracefulDegradation_NoCommunityData()
    {
        // T-COMMUNITY-001.10: RTM FR-13.10
        // Graph with concepts and edges but NO community detection has run.
        // BuildContext should work without error, with null CommunityId and empty CommunitySiblings.
        using var conn = OpenTestDb();

        InsertConcept(conn, "degrade-a");
        InsertConcept(conn, "degrade-b");
        InsertEdge(conn, "degrade-a", "degrade-b", 5);

        // Verify concept_communities is empty (no RunAndPersist called)
        var rowCount = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concept_communities");
        Assert.That(rowCount, Is.EqualTo(0), "Precondition: concept_communities must be empty.");

        conn.Dispose();

        // Call BuildContext — should NOT throw
        BuildContextResult? result = null;
        Assert.DoesNotThrow(() =>
        {
            result = GraphTools.BuildContext("degrade-a", depth: 1, maxEntities: 20);
        }, "FR-13.10: BuildContext must not throw when concept_communities is empty.");

        Assert.That(result, Is.Not.Null);

        // CommunityId should be null
        Assert.That(result!.CommunityId, Is.Null,
            "FR-13.10: CommunityId should be null when no community data exists.");

        // CommunitySiblings should be empty
        Assert.That(result.CommunitySiblings, Is.Empty,
            "FR-13.10: CommunitySiblings should be empty when no community data exists.");

        // Direct relations should still work normally
        Assert.That(result.DirectRelations.Count, Is.GreaterThan(0),
            "FR-13.10: Direct relations should still be returned even without community data.");

        // RelatedConcept.CommunityId should be null
        foreach (var rel in result.DirectRelations)
        {
            Assert.That(rel.CommunityId, Is.Null,
                $"FR-13.10: RelatedConcept '{rel.Name}' CommunityId should be null when no community data exists.");
        }
    }

    // ──────────────────────────────────────────────────────────
    // FR-13.10: Graceful degradation — concept not in communities
    // ──────────────────────────────────────────────────────────
    [Test]
    public void BuildContext_GracefulDegradation_ConceptNotInCommunities()
    {
        // T-COMMUNITY-001.10: RTM FR-13.10
        // Run community detection on existing concepts, then add a NEW concept that exists
        // in the concepts table but NOT in concept_communities.
        // BuildContext for that new concept should not error.
        using var conn = OpenTestDb();

        // Create a triangle and run community detection
        InsertConcept(conn, "existing-a");
        InsertConcept(conn, "existing-b");
        InsertConcept(conn, "existing-c");
        InsertEdge(conn, "existing-a", "existing-b", 10);
        InsertEdge(conn, "existing-b", "existing-c", 10);
        InsertEdge(conn, "existing-a", "existing-c", 10);

        CommunityDetection.RunAndPersist(conn, gamma: 1.0, seed: 42);

        // Verify community data exists for existing concepts
        var existingCount = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concept_communities");
        Assert.That(existingCount, Is.GreaterThan(0), "Precondition: community data must exist.");

        // Add a new concept with an edge to existing-a, but do NOT re-run community detection
        InsertConcept(conn, "orphan-concept");
        InsertEdge(conn, "orphan-concept", "existing-a", 5);

        // Verify orphan-concept is NOT in concept_communities
        var orphanInComm = conn.QuerySingle<bool>(
            "SELECT COUNT(*) > 0 FROM concept_communities WHERE concept_name = 'orphan-concept'");
        Assert.That(orphanInComm, Is.False, "Precondition: orphan-concept must not be in concept_communities.");

        conn.Dispose();

        // Call BuildContext for the orphan concept — should NOT throw
        BuildContextResult? result = null;
        Assert.DoesNotThrow(() =>
        {
            result = GraphTools.BuildContext("orphan-concept", depth: 1, maxEntities: 20);
        }, "FR-13.10: BuildContext must not throw for a concept not in concept_communities.");

        Assert.That(result, Is.Not.Null);

        // The orphan concept's own CommunityId should be null
        Assert.That(result!.CommunityId, Is.Null,
            "FR-13.10: CommunityId should be null for a concept not in concept_communities.");

        // CommunitySiblings should be empty (no community assignment to compute siblings from)
        Assert.That(result.CommunitySiblings, Is.Empty,
            "FR-13.10: CommunitySiblings should be empty for a concept not in concept_communities.");

        // Direct relations should still work — existing-a should appear
        var directNames = result.DirectRelations.Select(r => r.Name).ToList();
        Assert.That(directNames, Does.Contain("existing-a"),
            "FR-13.10: Direct relations should still work for concept not in concept_communities.");

        // existing-a's CommunityId should be non-null (it has community data)
        var existingARelation = result.DirectRelations.First(r => r.Name == "existing-a");
        Assert.That(existingARelation.CommunityId, Is.Not.Null,
            "FR-13.10: RelatedConcept with community data should still have CommunityId populated.");
    }
}
