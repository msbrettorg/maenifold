// T-COV-001.7: RTM FR-17.9

using Maenifold.Tools;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Tests for AssumptionLedgerValidation — all validation rules (required fields,
/// enum values, concept format) and edge cases (empty strings, null values).
///
/// Ma Protocol Compliance: These tests use REAL static method calls only.
/// - No mocks, no stubs, no test doubles
/// - Pure validation logic — no I/O required
/// - All branches exercised directly
///
/// T-COV-001.7: RTM FR-17.9
/// </summary>
[TestFixture]
[Category("T-COV-001.7")]
public class AssumptionLedgerValidationTests
{
    // Static readonly arrays to satisfy CA1861 (no constant array arguments).
    private static readonly string[] SingleConcept = ["concept"];
    private static readonly string[] ConceptWithFullBrackets = ["[[foo]]"];
    private static readonly string[] ConceptWithOpenBracket = ["[[dialogue"];
    private static readonly string[] ConceptWithCloseBracket = ["dialogue]]"];
    private static readonly string[] SingleValidConcept = ["dialogue"];
    private static readonly string[] MultipleValidConcepts = ["dialogue", "workflow-dispatch", "assumption-ledger"];
    private static readonly string[] GoodThenBadConcept = ["good-concept", "[[bad-concept]]"];

    // -------------------------------------------------------------------------
    // ValidateAppendInput
    // -------------------------------------------------------------------------

    [Test]
    [Description("null assumption text returns required-field error")]
    public void ValidateAppendInput_NullAssumption_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateAppendInput(null!, SingleConcept);

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("Assumption text is required"));
    }

    [Test]
    [Description("empty string assumption returns required-field error")]
    public void ValidateAppendInput_EmptyAssumption_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateAppendInput(string.Empty, SingleConcept);

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("Assumption text is required"));
    }

    [Test]
    [Description("whitespace-only assumption returns required-field error")]
    public void ValidateAppendInput_WhitespaceAssumption_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateAppendInput("   ", SingleConcept);

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("Assumption text is required"));
    }

    [Test]
    [Description("null concepts array returns missing-concepts error")]
    public void ValidateAppendInput_NullConcepts_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateAppendInput("Valid assumption", null);

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("[[WikiLink]]"));
    }

    [Test]
    [Description("empty concepts array returns missing-concepts error")]
    public void ValidateAppendInput_EmptyConceptsArray_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateAppendInput("Valid assumption", System.Array.Empty<string>());

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("[[WikiLink]]"));
    }

    [Test]
    [Description("concept containing full brackets [[foo]] returns bracket-format error")]
    public void ValidateAppendInput_ConceptWithFullBrackets_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateAppendInput("Valid assumption", ConceptWithFullBrackets);

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("brackets"));
    }

    [Test]
    [Description("concept containing only opening bracket sequence returns bracket-format error")]
    public void ValidateAppendInput_ConceptWithOnlyOpenBracket_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateAppendInput("Valid assumption", ConceptWithOpenBracket);

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("brackets"));
    }

    [Test]
    [Description("concept containing only closing bracket sequence returns bracket-format error")]
    public void ValidateAppendInput_ConceptWithOnlyCloseBracket_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateAppendInput("Valid assumption", ConceptWithCloseBracket);

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("brackets"));
    }

    [Test]
    [Description("valid assumption with single valid concept returns null (success)")]
    public void ValidateAppendInput_ValidSingleConcept_ReturnsNull()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateAppendInput("Valid assumption text", SingleValidConcept);

        Assert.That(result, Is.Null);
    }

    [Test]
    [Description("valid assumption with multiple valid concepts returns null (success)")]
    public void ValidateAppendInput_ValidMultipleConcepts_ReturnsNull()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateAppendInput("Valid assumption text", MultipleValidConcepts);

        Assert.That(result, Is.Null);
    }

    [Test]
    [Description("bracket error detected on second concept when first concept is valid")]
    public void ValidateAppendInput_BracketErrorOnSecondConcept_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateAppendInput("Valid assumption text", GoodThenBadConcept);

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("brackets"));
    }

    // -------------------------------------------------------------------------
    // ValidateUpdateInput
    // -------------------------------------------------------------------------

    [Test]
    [Description("null URI returns required-field error for update")]
    public void ValidateUpdateInput_NullUri_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateUpdateInput(null!, null);

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("URI is required"));
    }

    [Test]
    [Description("empty URI returns required-field error for update")]
    public void ValidateUpdateInput_EmptyUri_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateUpdateInput(string.Empty, null);

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("URI is required"));
    }

    [Test]
    [Description("whitespace URI returns required-field error for update")]
    public void ValidateUpdateInput_WhitespaceUri_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateUpdateInput("   ", null);

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("URI is required"));
    }

    [Test]
    [Description("valid URI with null status returns null (status is optional)")]
    public void ValidateUpdateInput_ValidUri_NullStatus_ReturnsNull()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateUpdateInput(
            "memory://assumptions/2026/02/assumption-abc123", null);

        Assert.That(result, Is.Null);
    }

    [Test]
    [Description("valid URI with empty status returns null (empty status is skipped)")]
    public void ValidateUpdateInput_ValidUri_EmptyStatus_ReturnsNull()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateUpdateInput(
            "memory://assumptions/2026/02/assumption-abc123", string.Empty);

        Assert.That(result, Is.Null);
    }

    [Test]
    [Description("valid URI with status 'active' returns null")]
    public void ValidateUpdateInput_ValidUri_StatusActive_ReturnsNull()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateUpdateInput(
            "memory://assumptions/2026/02/assumption-abc123", "active");

        Assert.That(result, Is.Null);
    }

    [Test]
    [Description("valid URI with status 'validated' returns null")]
    public void ValidateUpdateInput_ValidUri_StatusValidated_ReturnsNull()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateUpdateInput(
            "memory://assumptions/2026/02/assumption-abc123", "validated");

        Assert.That(result, Is.Null);
    }

    [Test]
    [Description("valid URI with status 'invalidated' returns null")]
    public void ValidateUpdateInput_ValidUri_StatusInvalidated_ReturnsNull()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateUpdateInput(
            "memory://assumptions/2026/02/assumption-abc123", "invalidated");

        Assert.That(result, Is.Null);
    }

    [Test]
    [Description("valid URI with status 'refined' returns null")]
    public void ValidateUpdateInput_ValidUri_StatusRefined_ReturnsNull()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateUpdateInput(
            "memory://assumptions/2026/02/assumption-abc123", "refined");

        Assert.That(result, Is.Null);
    }

    [Test]
    [Description("valid URI with invalid status returns status error")]
    public void ValidateUpdateInput_ValidUri_InvalidStatus_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateUpdateInput(
            "memory://assumptions/2026/02/assumption-abc123", "bogus");

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("Invalid status"));
    }

    // -------------------------------------------------------------------------
    // ValidateReadInput
    // -------------------------------------------------------------------------

    [Test]
    [Description("null URI returns required-field error for read")]
    public void ValidateReadInput_NullUri_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateReadInput(null!);

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("URI is required"));
    }

    [Test]
    [Description("empty URI returns required-field error for read")]
    public void ValidateReadInput_EmptyUri_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateReadInput(string.Empty);

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("URI is required"));
    }

    [Test]
    [Description("valid URI returns null (success)")]
    public void ValidateReadInput_ValidUri_ReturnsNull()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateReadInput(
            "memory://assumptions/2026/02/assumption-abc123");

        Assert.That(result, Is.Null);
    }

    // -------------------------------------------------------------------------
    // ValidateStatus (public method, direct tests)
    // -------------------------------------------------------------------------

    [Test]
    [Description("status 'active' is valid")]
    public void ValidateStatus_Active_ReturnsNull()
    {
        // T-COV-001.7: RTM FR-17.9
        Assert.That(AssumptionLedgerValidation.ValidateStatus("active"), Is.Null);
    }

    [Test]
    [Description("status 'validated' is valid")]
    public void ValidateStatus_Validated_ReturnsNull()
    {
        // T-COV-001.7: RTM FR-17.9
        Assert.That(AssumptionLedgerValidation.ValidateStatus("validated"), Is.Null);
    }

    [Test]
    [Description("status 'invalidated' is valid")]
    public void ValidateStatus_Invalidated_ReturnsNull()
    {
        // T-COV-001.7: RTM FR-17.9
        Assert.That(AssumptionLedgerValidation.ValidateStatus("invalidated"), Is.Null);
    }

    [Test]
    [Description("status 'refined' is valid")]
    public void ValidateStatus_Refined_ReturnsNull()
    {
        // T-COV-001.7: RTM FR-17.9
        Assert.That(AssumptionLedgerValidation.ValidateStatus("refined"), Is.Null);
    }

    [Test]
    [Description("status 'Active' (mixed case) is valid — case-insensitive comparison")]
    public void ValidateStatus_ActiveMixedCase_ReturnsNull()
    {
        // T-COV-001.7: RTM FR-17.9
        Assert.That(AssumptionLedgerValidation.ValidateStatus("Active"), Is.Null);
    }

    [Test]
    [Description("status 'VALIDATED' (uppercase) is valid — case-insensitive comparison")]
    public void ValidateStatus_ValidatedUpperCase_ReturnsNull()
    {
        // T-COV-001.7: RTM FR-17.9
        Assert.That(AssumptionLedgerValidation.ValidateStatus("VALIDATED"), Is.Null);
    }

    [Test]
    [Description("unknown status 'bogus' returns error")]
    public void ValidateStatus_InvalidStatus_ReturnsError()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateStatus("bogus");

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.StartWith("ERROR:"));
        Assert.That(result, Does.Contain("Invalid status"));
    }

    [Test]
    [Description("error message for invalid status lists all valid values")]
    public void ValidateStatus_InvalidStatus_ErrorMessageContainsAllValidValues()
    {
        // T-COV-001.7: RTM FR-17.9
        var result = AssumptionLedgerValidation.ValidateStatus("bogus");

        Assert.That(result, Is.Not.Null);
        Assert.That(result, Does.Contain("active"));
        Assert.That(result, Does.Contain("validated"));
        Assert.That(result, Does.Contain("invalidated"));
        Assert.That(result, Does.Contain("refined"));
    }
}
