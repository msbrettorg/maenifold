using System.Globalization;
using System.Text.Json;
using Microsoft.Data.Sqlite;
using Maenifold.Utils;

namespace Maenifold.Tools;

// T-COMMUNITY-001.2: RTM FR-13.1, FR-13.2, FR-13.11, NFR-13.1.1, NFR-13.1.2, NFR-13.2.1
// Louvain modularity optimization (Blondel et al. 2008) for concept community detection.
// In-memory implementation with no external dependencies. Deterministic via optional seed.
public static class CommunityDetection
{
    /// <summary>
    /// Run Louvain modularity optimization on the concept graph.
    /// Returns (concept → community_id mapping, modularity score).
    /// </summary>
    /// <param name="conn">Open SQLite connection with concept_graph table.</param>
    /// <param name="gamma">Resolution parameter controlling community granularity. Default 1.0.</param>
    /// <param name="seed">Optional seed for deterministic node ordering.</param>
    public static (Dictionary<string, int> Communities, double Modularity) RunLouvain(
        SqliteConnection conn, double gamma = 1.0, int? seed = null)
    {
        // T-PROX-003.2: RTM FR-23.6 — Load source_files; edge weight = sum(proximity)
        var edgeList = conn.Query<(string a, string b, string sourceFiles)>(
            "SELECT concept_a, concept_b, source_files FROM concept_graph").ToList();

        // T-PROX-003.2: RTM FR-23.6 - Edge weight = sum(proximity) from source_files
        var weightedEdges = edgeList.Select(e =>
        {
            double weight = 0.0;
            if (!string.IsNullOrWhiteSpace(e.sourceFiles))
            {
                var dict = JsonSerializer.Deserialize<Dictionary<string, double>>(e.sourceFiles, SafeJson.Options);
                if (dict != null)
                    weight = dict.Values.Sum();
            }
            return (a: e.a, b: e.b, weight: weight);
        }).ToList();

        // Build name ↔ ID mapping
        var nameToId = new Dictionary<string, int>();
        var idToName = new Dictionary<int, string>();

        void EnsureNode(string name)
        {
            if (!nameToId.ContainsKey(name))
            {
                int id = nameToId.Count;
                nameToId[name] = id;
                idToName[id] = name;
            }
        }

        foreach (var (a, b, _) in weightedEdges)
        {
            EnsureNode(a);
            EnsureNode(b);
        }

        // Include isolated concepts (in concepts table but no edges)
        var allConcepts = conn.Query<string>("SELECT concept_name FROM concepts");
        foreach (var concept in allConcepts)
            EnsureNode(concept);

        int n = nameToId.Count;
        if (n == 0)
            return (new Dictionary<string, int>(), 0.0);

        // Build adjacency lists with integer IDs
        // adj[i] = list of (neighbor, weight)
        var adj = new Dictionary<int, List<(int neighbor, double weight)>>();
        for (int i = 0; i < n; i++)
            adj[i] = new List<(int, double)>();

        double totalWeight = 0; // m = sum of all edge weights (each edge once)
        foreach (var (a, b, weight) in weightedEdges)
        {
            int ia = nameToId[a];
            int ib = nameToId[b];
            adj[ia].Add((ib, weight));
            adj[ib].Add((ia, weight));
            totalWeight += weight;
        }

        if (totalWeight == 0)
        {
            // No edges: each node is its own community
            var result = new Dictionary<string, int>();
            for (int i = 0; i < n; i++)
                result[idToName[i]] = i;
            return (result, 0.0);
        }

        // Phase 1 only: local node moves (no Phase 2 aggregation — sufficient for current graph scale)
        var community = new int[n];
        for (int i = 0; i < n; i++)
            community[i] = i;

        var finalCommunity = LouvainIterative(adj, community, n, totalWeight, gamma, seed);

        // Map back to concept names
        var communities = new Dictionary<string, int>();
        for (int i = 0; i < n; i++)
            communities[idToName[i]] = finalCommunity[i];

        // Normalize to 0..K-1
        communities = NormalizeCommunityIds(communities);

        double modularity = ComputeModularity(adj, finalCommunity, n, totalWeight, gamma);

        return (communities, modularity);
    }

    /// <summary>
    /// Core Louvain loop: Phase 1 local node moves, repeat until no modularity improvement.
    /// </summary>
    private static int[] LouvainIterative(
        Dictionary<int, List<(int neighbor, double weight)>> adj,
        int[] community, int nodeCount, double m, double gamma, int? seed)
    {
        // Precompute weighted degrees
        var degree = new double[nodeCount];
        for (int i = 0; i < nodeCount; i++)
        {
            double d = 0;
            foreach (var (_, w) in adj[i])
                d += w;
            degree[i] = d;
        }

        // Precompute community aggregates
        // sumTot[c] = sum of weighted degrees of all nodes in community c
        // sumIn[c] = sum of edge weights with both endpoints in community c (each edge once)
        var sumTot = new Dictionary<int, double>();
        var sumIn = new Dictionary<int, double>();
        for (int i = 0; i < nodeCount; i++)
        {
            if (!sumTot.TryGetValue(community[i], out _))
            {
                sumTot[community[i]] = 0;
                sumIn[community[i]] = 0;
            }
            sumTot[community[i]] += degree[i];
        }

        // Compute initial sumIn
        for (int i = 0; i < nodeCount; i++)
        {
            foreach (var (j, w) in adj[i])
            {
                if (j > i && community[i] == community[j])
                {
                    sumIn[community[i]] += w;
                }
            }
        }

        // Phase 1: Local node moves
        bool improved = true;
        while (improved)
        {
            improved = false;

            var rng = seed.HasValue ? new Random(seed.Value) : new Random();
            var order = Enumerable.Range(0, nodeCount).ToList();
            Shuffle(order, rng);

            foreach (var i in order)
            {
                var currentComm = community[i];
                double ki = degree[i];

                // Compute k_i_in for current community and each neighbor community
                var neighborCommWeights = new Dictionary<int, double>();
                foreach (var (j, w) in adj[i])
                {
                    var jComm = community[j];
                    if (!neighborCommWeights.TryGetValue(jComm, out _))
                        neighborCommWeights[jComm] = 0;
                    neighborCommWeights[jComm] += w;
                }

                double kiInCurrent = neighborCommWeights.GetValueOrDefault(currentComm, 0);

                // Remove node from its community
                sumTot[currentComm] -= ki;
                sumIn[currentComm] -= kiInCurrent;

                // Evaluate gain for each neighbor community
                int bestComm = currentComm;
                double bestGain = 0;

                foreach (var (targetComm, kiIn) in neighborCommWeights)
                {
                    // Delta Q for moving i into targetComm:
                    // gain = kiIn/m - gamma * sumTot[targetComm] * ki / (2m^2)
                    double gain = kiIn / m - gamma * sumTot[targetComm] * ki / (2.0 * m * m);

                    // Compare against staying removed (gain of re-inserting into current):
                    double currentGain = kiInCurrent / m - gamma * sumTot[currentComm] * ki / (2.0 * m * m);

                    double netGain = gain - currentGain;

                    if (netGain > bestGain)
                    {
                        bestGain = netGain;
                        bestComm = targetComm;
                    }
                }

                // Re-insert node into best community
                double kiInBest = neighborCommWeights.GetValueOrDefault(bestComm, 0);
                community[i] = bestComm;
                sumTot[bestComm] += ki;
                sumIn[bestComm] += kiInBest;

                if (bestComm != currentComm)
                    improved = true;
            }
        }

        return community;
    }

    /// <summary>
    /// Run Louvain and persist results to concept_communities table.
    /// Returns (number of distinct communities, modularity).
    /// </summary>
    public static (int CommunityCount, double Modularity) RunAndPersist(
        SqliteConnection conn, double gamma = 1.0, int? seed = null)
    {
        var (communities, modularity) = RunLouvain(conn, gamma, seed);

        if (communities.Count == 0)
            return (0, 0.0);

        var timestamp = CultureInvariantHelpers.FormatDateTime(DateTime.UtcNow, "O");

        // T-COMMUNITY-001.11: RTM FAIL-001 remediation — atomic DELETE+INSERT in transaction
        // Prevents concurrent RunAndPersist calls from interleaving and corrupting the partition.
        using var transaction = conn.BeginTransaction();

        conn.Execute("DELETE FROM concept_communities");

        foreach (var (conceptName, communityId) in communities)
        {
            conn.Execute(
                @"INSERT OR IGNORE INTO concept_communities (concept_name, community_id, modularity, resolution, detected_at)
                  VALUES (@name, @cid, @mod, @res, @ts)",
                new { name = conceptName, cid = communityId, mod = modularity, res = gamma, ts = timestamp });
        }

        transaction.Commit();

        var distinctCommunities = communities.Values.Distinct().Count();
        return (distinctCommunities, modularity);
    }

    /// <summary>
    /// Compute modularity Q = (1/2m) * sum[A_ij - gamma * k_i * k_j / (2m)] * delta(c_i, c_j)
    /// </summary>
    private static double ComputeModularity(
        Dictionary<int, List<(int neighbor, double weight)>> adj,
        int[] community, int nodeCount, double m, double gamma)
    {
        if (m == 0) return 0.0;

        // Precompute weighted degrees
        var degree = new double[nodeCount];
        for (int i = 0; i < nodeCount; i++)
        {
            foreach (var (_, w) in adj[i])
                degree[i] += w;
        }

        double q = 0;
        for (int i = 0; i < nodeCount; i++)
        {
            foreach (var (j, w) in adj[i])
            {
                if (community[i] == community[j])
                {
                    q += w - gamma * degree[i] * degree[j] / (2.0 * m);
                }
            }
        }

        return q / (2.0 * m);
    }

    /// <summary>
    /// Normalize community IDs to contiguous 0..N-1 range.
    /// </summary>
    private static Dictionary<string, int> NormalizeCommunityIds(Dictionary<string, int> communities)
    {
        var idMap = new Dictionary<int, int>();
        int nextId = 0;

        var result = new Dictionary<string, int>();
        foreach (var (node, comm) in communities.OrderBy(kv => kv.Key, StringComparer.Ordinal))
        {
            if (!idMap.TryGetValue(comm, out var normalizedId))
            {
                normalizedId = nextId++;
                idMap[comm] = normalizedId;
            }
            result[node] = normalizedId;
        }

        return result;
    }

    /// <summary>
    /// Fisher-Yates shuffle for deterministic node ordering (NFR-13.1.2).
    /// </summary>
    private static void Shuffle<T>(List<T> list, Random rng)
    {
        for (int i = list.Count - 1; i > 0; i--)
        {
            int j = rng.Next(i + 1);
            (list[i], list[j]) = (list[j], list[i]);
        }
    }

    // T-PROX-001.3: RTM FR-23.4
    /// <summary>
    /// Computes the effective edge weight as the average of (proximity × decay) across all source files.
    /// <para>
    /// <paramref name="sourceFilesJson"/> must be a JSON object mapping memory:// URIs to their
    /// proximity weights (e.g. <c>{"memory://notes/foo.md": 0.5}</c>).
    /// For each entry the temporal decay weight is fetched via
    /// <see cref="MemorySearchTools.GetDecayWeightForFile"/> and multiplied by the proximity value.
    /// The average of these products is returned.
    /// </para>
    /// <para>Returns 0.0 for an empty source-files dict.</para>
    /// <para>Throws if <paramref name="sourceFilesJson"/> is not valid JSON (NO FAKE AI — no silent catch).</para>
    /// </summary>
    internal static double ComputeEdgeDecayWeight(string sourceFilesJson)
    {
        var sourceFiles = JsonSerializer.Deserialize<Dictionary<string, double>>(sourceFilesJson, SafeJson.Options);

        if (sourceFiles == null || sourceFiles.Count == 0)
            return 0.0;

        double total = 0.0;
        foreach (var (uri, proximity) in sourceFiles)
        {
            double decay = MemorySearchTools.GetDecayWeightForFile(uri);
            total += proximity * decay;
        }

        return total / sourceFiles.Count;
    }

    // T-PROX-003.3: RTM FR-23.7
    /// <summary>
    /// Compute Normalized Mutual Information between two clusterings.
    /// NMI(A, B) = 2 * I(A; B) / (H(A) + H(B)), range [0, 1].
    /// Returns 1.0 when clusterings are identical, approaches 0.0 for independent clusterings.
    /// </summary>
    /// <remarks>
    /// STUB — implementation added by T-PROX-003.3.
    /// </remarks>
    public static double ComputeNMI(Dictionary<string, int> oldCommunities, Dictionary<string, int> newCommunities)
    {
        throw new NotImplementedException("T-PROX-003.3: ComputeNMI not yet implemented.");
    }
}
