using System.Text;
using Maenifold.Utils;

namespace Maenifold.Models;

public class BuildContextResult
{
    public string ConceptName { get; set; } = string.Empty;
    public int Depth { get; set; }
    public List<RelatedConcept> DirectRelations { get; set; } = new();
    public List<string> ExpandedRelations { get; set; } = new();

    public override string ToString()
    {
        var sb = new StringBuilder();
        sb.AppendLineInvariant($"Context for CONCEPT: {ConceptName}");
        sb.AppendLine("");

        sb.AppendLineInvariant($"Direct relations ({DirectRelations.Count} CONCEPTS):");
        foreach (var related in DirectRelations)
        {
            sb.AppendLineInvariant($"  • {related.Name} (co-occurs in {related.CoOccurrenceCount} files)");
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
}
