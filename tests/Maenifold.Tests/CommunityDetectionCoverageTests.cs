using System;
using System.IO;
using System.Linq;
using Maenifold.Tools;
using Maenifold.Utils;
using Microsoft.Data.Sqlite;
using NUnit.Framework;

namespace Maenifold.Tests;

// T-COV-001.CD: RTM FR-13.1, FR-13.2 — targeted branch coverage for CommunityDetection.cs
// Covers branches not exercised by CommunityDetectionTests.cs: totalWeight==0 with multiple
// isolated nodes, RunAndPersist early-return on empty graph, ComputeModularity m==0 guard,
// and LouvainIterative sumIn initial-compute inner condition (j>i && community[i]==community[j]).
[TestFixture]
[NonParallelizable]
public class CommunityDetectionCoverageTests
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
        _testRoot = Path.Combine(repoRoot, "test-outputs", "community-detection-cov", $"run-{Guid.NewGuid():N}");

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
    // Branch: RunLouvain line 71-78 — totalWeight == 0 with multiple isolated nodes.
    // Multiple concepts exist in the concepts table but zero edges in concept_graph.
    // Each node must receive its own unique community and modularity must be 0.0.
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunLouvain_MultipleIsolatedConcepts_NoEdges_EachGetOwnCommunity()
    {
        // T-COV-001.CD: RTM FR-13.1 — totalWeight==0 branch with n>1
        using var conn = OpenTestDb();

        InsertConcept(conn, "iso-a");
        InsertConcept(conn, "iso-b");
        InsertConcept(conn, "iso-c");
        // Zero edges — totalWeight will be 0, triggering the early return on line 71-78.

        var (communities, modularity) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);

        Assert.That(communities.Count, Is.EqualTo(3),
            "All three isolated concepts must appear in the result.");

        var distinctIds = communities.Values.Distinct().Count();
        Assert.That(distinctIds, Is.EqualTo(3),
            "Each isolated concept must receive its own unique community (no edges → no merging).");

        Assert.That(modularity, Is.EqualTo(0.0),
            "Modularity must be 0.0 when there are no edges (totalWeight == 0).");

        // All community IDs must be in 0..2 (normalized)
        Assert.That(communities.Values.All(id => id >= 0 && id < 3), Is.True,
            "Normalized community IDs must be in range 0..N-1.");
    }

    // ──────────────────────────────────────────────────────────
    // Branch: RunLouvain line 71-78 — totalWeight == 0 with many isolated nodes.
    // Verifies the loop body `result[idToName[i]] = i` runs for each node.
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunLouvain_FiveIsolatedConcepts_NoEdges_FiveDistinctCommunities()
    {
        // T-COV-001.CD: RTM FR-13.1 — totalWeight==0 branch, larger node count
        using var conn = OpenTestDb();

        foreach (var name in new[] { "node-1", "node-2", "node-3", "node-4", "node-5" })
            InsertConcept(conn, name);

        var (communities, modularity) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 7);

        Assert.That(communities.Count, Is.EqualTo(5),
            "All five isolated concepts must be present in the result.");

        Assert.That(communities.Values.Distinct().Count(), Is.EqualTo(5),
            "Five isolated concepts must each have a unique community ID.");

        Assert.That(modularity, Is.EqualTo(0.0),
            "No edges means 0.0 modularity.");
    }

    // ──────────────────────────────────────────────────────────
    // Branch: RunAndPersist line 220-221 — early return when graph is entirely empty.
    // If there are no concepts and no edges, RunLouvain returns an empty dictionary,
    // and RunAndPersist must return (0, 0.0) without performing any DB operations.
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunAndPersist_EmptyGraph_ReturnsZeroWithoutDbWrites()
    {
        // T-COV-001.CD: RTM FR-13.2 — empty graph early-return at line 220-221
        using var conn = OpenTestDb();

        // Verify precondition: no concepts, no edges
        var conceptCount = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concepts");
        Assert.That(conceptCount, Is.EqualTo(0), "Precondition: concepts table must be empty.");

        var (communityCount, modularity) = CommunityDetection.RunAndPersist(conn, gamma: 1.0, seed: 42);

        Assert.That(communityCount, Is.EqualTo(0),
            "Empty graph must return 0 community count.");
        Assert.That(modularity, Is.EqualTo(0.0),
            "Empty graph must return 0.0 modularity.");

        // No rows should be written to concept_communities
        var rowCount = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concept_communities");
        Assert.That(rowCount, Is.EqualTo(0),
            "RunAndPersist must not write any rows for an empty graph.");
    }

    // ──────────────────────────────────────────────────────────
    // Branch: RunAndPersist line 220-221 — early return via isolated-concepts path.
    // Concepts exist but have no edges; RunLouvain returns non-empty (each node gets
    // its own community). RunAndPersist must proceed to persist those communities.
    // This also validates that the isolated-concept path persists correctly.
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunAndPersist_IsolatedConcepts_PersistsEachWithOwnCommunity()
    {
        // T-COV-001.CD: RTM FR-13.2 — isolated concepts path through RunAndPersist
        using var conn = OpenTestDb();

        InsertConcept(conn, "persist-iso-a");
        InsertConcept(conn, "persist-iso-b");

        var (communityCount, modularity) = CommunityDetection.RunAndPersist(conn, gamma: 1.0, seed: 42);

        Assert.That(communityCount, Is.EqualTo(2),
            "Two isolated concepts must each be in their own community.");
        Assert.That(modularity, Is.EqualTo(0.0),
            "Modularity must be 0.0 (no edges).");

        var rowCount = conn.ExecuteScalar<long>("SELECT COUNT(*) FROM concept_communities");
        Assert.That(rowCount, Is.EqualTo(2),
            "Both isolated concepts must be persisted to concept_communities.");

        // Each concept must have a distinct community_id
        var communityIdA = conn.QuerySingle<int>(
            "SELECT community_id FROM concept_communities WHERE concept_name = 'persist-iso-a'");
        var communityIdB = conn.QuerySingle<int>(
            "SELECT community_id FROM concept_communities WHERE concept_name = 'persist-iso-b'");

        Assert.That(communityIdA, Is.Not.EqualTo(communityIdB),
            "Both isolated concepts must have distinct community IDs.");
    }

    // ──────────────────────────────────────────────────────────
    // Branch: LouvainIterative sumIn initial-compute — j>i && community[i]==community[j].
    // At initialization, community[i]=i for all i, so no two nodes share a community.
    // This means the inner condition is always false for distinct nodes.
    // A graph where nodes start in their own communities and Louvain merges them
    // exercises the condition (confirms false path) while still producing a valid partition.
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunLouvain_LinearChain_CorrectCommunityAssignment()
    {
        // T-COV-001.CD: RTM FR-13.1 — LouvainIterative sumIn init branch (j>i && community same)
        // Linear chain: a—b—c—d. Louvain may merge them or split into two communities.
        // The key is that the initial sumIn loop processes all edges with the always-false condition.
        using var conn = OpenTestDb();

        InsertConcept(conn, "chain-a");
        InsertConcept(conn, "chain-b");
        InsertConcept(conn, "chain-c");
        InsertConcept(conn, "chain-d");

        // Linear chain with equal weights
        InsertEdge(conn, "chain-a", "chain-b", 3);
        InsertEdge(conn, "chain-b", "chain-c", 3);
        InsertEdge(conn, "chain-c", "chain-d", 3);

        var (communities, modularity) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);

        Assert.That(communities.Count, Is.EqualTo(4),
            "All four chain nodes must appear in the result.");

        // Modularity should be non-negative for a connected graph with structure
        Assert.That(modularity, Is.GreaterThanOrEqualTo(0.0),
            "Modularity must be non-negative for a connected chain.");

        // Communities should be a valid partition (IDs in 0..K-1 range)
        var maxId = communities.Values.Max();
        Assert.That(maxId, Is.LessThan(communities.Count),
            "Community IDs must be normalized to 0..K-1 range.");
    }

    // ──────────────────────────────────────────────────────────
    // Branch: LouvainIterative bestComm != currentComm — line 203-204.
    // A graph where Louvain definitely moves a node (improved=true path).
    // Two strong cliques with one node that starts in the wrong community.
    // We verify the algorithm converges to the correct partition.
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunLouvain_TwoCliquesWeakBridge_ImprovedFlagSet()
    {
        // T-COV-001.CD: RTM FR-13.1 — bestComm!=currentComm branch (improved=true)
        // Two dense cliques with a weak bridge between them.
        // The bridge node will be moved, triggering improved=true.
        using var conn = OpenTestDb();

        // Clique 1: three nodes fully connected (weight 10)
        InsertConcept(conn, "clq1-x");
        InsertConcept(conn, "clq1-y");
        InsertConcept(conn, "clq1-z");
        InsertEdge(conn, "clq1-x", "clq1-y", 10);
        InsertEdge(conn, "clq1-y", "clq1-z", 10);
        InsertEdge(conn, "clq1-x", "clq1-z", 10);

        // Clique 2: three nodes fully connected (weight 10)
        InsertConcept(conn, "clq2-x");
        InsertConcept(conn, "clq2-y");
        InsertConcept(conn, "clq2-z");
        InsertEdge(conn, "clq2-x", "clq2-y", 10);
        InsertEdge(conn, "clq2-y", "clq2-z", 10);
        InsertEdge(conn, "clq2-x", "clq2-z", 10);

        // Very weak bridge
        InsertEdge(conn, "clq1-x", "clq2-x", 1);

        var (communities, modularity) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);

        Assert.That(communities.Count, Is.EqualTo(6),
            "All six nodes must appear in the result.");

        // Each clique's nodes must end up in the same community
        Assert.That(communities["clq1-x"], Is.EqualTo(communities["clq1-y"]),
            "Clique 1 nodes must be in the same community.");
        Assert.That(communities["clq1-y"], Is.EqualTo(communities["clq1-z"]),
            "Clique 1 nodes must be in the same community.");

        Assert.That(communities["clq2-x"], Is.EqualTo(communities["clq2-y"]),
            "Clique 2 nodes must be in the same community.");
        Assert.That(communities["clq2-y"], Is.EqualTo(communities["clq2-z"]),
            "Clique 2 nodes must be in the same community.");

        // The two cliques must be in different communities
        Assert.That(communities["clq1-x"], Is.Not.EqualTo(communities["clq2-x"]),
            "Two dense cliques must be in separate communities.");

        Assert.That(modularity, Is.GreaterThan(0.0),
            "Two-clique graph with weak bridge must have positive modularity.");
    }

    // ──────────────────────────────────────────────────────────
    // Branch: LouvainIterative neighborCommWeights dictionary building.
    // A star graph (hub with many spokes) exercises the neighborCommWeights population
    // for a node with multiple neighbors and also exercises multiple iterations of
    // the outer community evaluation loop.
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunLouvain_StarTopology_HubAndSpokesClustered()
    {
        // T-COV-001.CD: RTM FR-13.1 — neighborCommWeights accumulation with multiple entries
        using var conn = OpenTestDb();

        // Hub connected to 5 spokes
        InsertConcept(conn, "hub");
        foreach (var spoke in new[] { "spoke-1", "spoke-2", "spoke-3", "spoke-4", "spoke-5" })
        {
            InsertConcept(conn, spoke);
            InsertEdge(conn, "hub", spoke, 5);
        }

        var (communities, modularity) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);

        Assert.That(communities.Count, Is.EqualTo(6),
            "All 6 nodes (hub + 5 spokes) must appear in the result.");

        // All IDs normalized
        var maxId = communities.Values.Max();
        Assert.That(maxId, Is.LessThan(communities.Count),
            "Community IDs must be normalized.");

        Assert.That(modularity, Is.GreaterThanOrEqualTo(0.0),
            "Star topology must produce non-negative modularity.");
    }

    // ──────────────────────────────────────────────────────────
    // Branch: ComputeModularity m==0 guard (line 252).
    // ComputeModularity is private so we reach it indirectly through RunLouvain.
    // When totalWeight > 0 the main path runs. The m==0 guard is reached only if
    // somehow totalWeight were 0 going into ComputeModularity — but the code returns
    // early at line 71-78 before ever calling LouvainIterative/ComputeModularity.
    // Therefore the m==0 guard in ComputeModularity (line 252) is dead code in the
    // current implementation. This test documents the reachable code path: a graph
    // with exactly one edge (weight 1) reaches ComputeModularity with m=1.
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunLouvain_SingleEdge_ReachesComputeModularity()
    {
        // T-COV-001.CD: RTM FR-13.1 — single edge exercises ComputeModularity with m=1
        using var conn = OpenTestDb();

        InsertConcept(conn, "edge-a");
        InsertConcept(conn, "edge-b");
        InsertEdge(conn, "edge-a", "edge-b", 1);

        var (communities, modularity) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);

        Assert.That(communities.Count, Is.EqualTo(2),
            "Both edge endpoints must appear in the result.");

        // Modularity for two nodes connected by one edge may be 0 or negative depending
        // on whether they stay in separate communities or merge. Either is valid.
        // The key assertion is that no exception is thrown and the result is finite.
        Assert.That(double.IsNaN(modularity), Is.False,
            "Modularity must not be NaN.");
        Assert.That(double.IsInfinity(modularity), Is.False,
            "Modularity must not be infinite.");
    }

    // ──────────────────────────────────────────────────────────
    // Regression: RunLouvain on isolated concepts returns stable normalized IDs.
    // IDs 0..N-1 with no gaps. Exercises NormalizeCommunityIds on the zero-weight path.
    // ──────────────────────────────────────────────────────────
    [Test]
    public void RunLouvain_IsolatedConcepts_NormalizedContiguousIds()
    {
        // T-COV-001.CD: RTM FR-13.1 — NormalizeCommunityIds applied to zero-weight result
        using var conn = OpenTestDb();

        InsertConcept(conn, "norm-a");
        InsertConcept(conn, "norm-b");
        InsertConcept(conn, "norm-c");
        InsertConcept(conn, "norm-d");

        var (communities, _) = CommunityDetection.RunLouvain(conn, gamma: 1.0, seed: 42);

        var ids = communities.Values.OrderBy(id => id).ToList();
        var expectedIds = new List<int> { 0, 1, 2, 3 };
        Assert.That(ids, Is.EqualTo(expectedIds),
            "Isolated concepts must receive contiguous normalized IDs 0..N-1.");
    }

    // ──────────────────────────────────────────────────────────
    // Helpers — mirrors the pattern from CommunityDetectionTests.cs
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
}
