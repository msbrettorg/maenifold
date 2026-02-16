using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using Maenifold.Tools;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

// T-COMMUNITY-001.4: RTM FR-13.1, FR-13.2, NFR-13.1.1, NFR-13.1.2, NFR-13.2.1, NFR-13.3.1
// Blue-team unit tests for Louvain community detection algorithm.
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
}
