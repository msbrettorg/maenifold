namespace Maenifold.Tools;

public static class AssumptionLedgerValidation
{
    private static readonly string[] ValidStatuses = ["active", "validated", "invalidated", "refined"];

    public static string? ValidateAppendInput(string assumption, string[]? concepts)
    {
        if (string.IsNullOrWhiteSpace(assumption))
            return "ERROR: Assumption text is required for 'append' action.";

        if (concepts == null || concepts.Length == 0)
            return "ERROR: At least one [[WikiLink]] tag must be provided. Example: concepts: [\"dialogue\", \"workflow-dispatch\"]";

        var conceptFormatError = ValidateConceptFormat(concepts);
        if (conceptFormatError != null)
            return conceptFormatError;

        return null;
    }

    public static string? ValidateUpdateInput(string uri, string? status)
    {
        if (string.IsNullOrWhiteSpace(uri))
            return "ERROR: URI is required for 'update' action.";

        if (!string.IsNullOrWhiteSpace(status))
        {
            var statusError = ValidateStatus(status);
            if (statusError != null)
                return statusError;
        }

        return null;
    }

    public static string? ValidateReadInput(string uri)
    {
        if (string.IsNullOrWhiteSpace(uri))
            return "ERROR: URI is required for 'read' action.";

        return null;
    }

    private static string? ValidateConceptFormat(string[] concepts)
    {
        foreach (var concept in concepts)
        {
            if (concept.Contains("[[") || concept.Contains("]]"))
                return "ERROR: Concept tags should not include brackets. Use 'dialogue' not '[[dialogue]]'.";
        }

        return null;
    }

    public static string? ValidateStatus(string status)
    {
        if (!ValidStatuses.Contains(status.ToLowerInvariant()))
            return $"ERROR: Invalid status. Use one of: {string.Join(", ", ValidStatuses)}";

        return null;
    }
}
