using System.Text;
using Maenifold.Utils;

namespace Maenifold.Models;

public class BuildContextResult
{
    public string ConceptName { get; set; } = string.Empty;
    public int Depth { get; set; }
    public List<RelatedConcept> DirectRelations { get; set; } = new();
    public List<string> ExpandedRelations { get; set; } = new();

    // T-COMMUNITY-001.7: RTM FR-13.6 - Community of the query concept
    public int? CommunityId { get; set; }

    // T-COMMUNITY-001.8: RTM FR-13.7
    public List<CommunitySibling> CommunitySiblings { get; set; } = new();

    public override string ToString()
    {
        var sb = new StringBuilder();
        sb.AppendLineInvariant($"Context for CONCEPT: {ConceptName}");
        sb.AppendLine("");

        sb.AppendLineInvariant($"Direct relations ({DirectRelations.Count} CONCEPTS):");
        foreach (var related in DirectRelations)
        {
            // T-COMMUNITY-001.7: RTM FR-13.6 - Include community_id when available
            var communityTag = related.CommunityId.HasValue ? $" [community {related.CommunityId}]" : "";
            sb.AppendLineInvariant($"  • {related.Name} (co-occurs in {related.CoOccurrenceCount} files){communityTag}");
            if (related.Files.Count > 0)
                sb.AppendLineInvariant($"    Files: {string.Join(", ", related.Files.Take(3))}");

            if (related.ContentPreview.Count > 0)
            {
                sb.AppendLineInvariant($"    Content preview:");
                foreach (var (file, preview) in related.ContentPreview)
                {
                    sb.AppendLineInvariant($"      {file}: {preview}");
                }
            }
        }

        if (Depth > 1)
        {
            sb.AppendLineInvariant($"\nExpanded relations ({Depth} hops):");
            foreach (var concept in ExpandedRelations.Take(20))
                sb.AppendLineInvariant($"  • {concept}");
        }

        // T-COMMUNITY-001.8: RTM FR-13.7 - Community siblings section
        if (CommunitySiblings.Count > 0)
        {
            sb.AppendLineInvariant($"\nCommunity siblings ({CommunitySiblings.Count} concepts in community {CommunityId}):");
            foreach (var sibling in CommunitySiblings)
            {
                sb.AppendLineInvariant($"  • {sibling.Name} (shared neighbors: {sibling.SharedNeighborCount}, overlap: {sibling.NormalizedOverlap:F3})");
            }
        }

        return sb.ToString();
    }
}

public class RelatedConcept
{
    public string Name { get; set; } = string.Empty;
    public int CoOccurrenceCount { get; set; }
    public List<string> Files { get; set; } = new();
    public Dictionary<string, string> ContentPreview { get; set; } = new();

    // T-GRAPH-DECAY-001.1: RTM FR-7.5 - Decay weighting for recency-based ranking
    /// <summary>
    /// Average decay weight across source files (0.0-1.0). Higher = more recent.
    /// </summary>
    public double DecayWeight { get; set; } = 1.0;

    /// <summary>
    /// Weighted score: CoOccurrenceCount * DecayWeight. Used for sorting.
    /// </summary>
    public double WeightedScore { get; set; }

    // T-COMMUNITY-001.7: RTM FR-13.6 - Community assignment for this concept
    public int? CommunityId { get; set; }
}

// T-COMMUNITY-001.8: RTM FR-13.7, FR-13.8, FR-13.9
public class CommunitySibling
{
    public string Name { get; set; } = string.Empty;
    public int CommunityId { get; set; }
    public int SharedNeighborCount { get; set; }
    public double NormalizedOverlap { get; set; }
}
