// T-COV-001.9: RTM FR-17.11
using System;
using System.Globalization;
using System.Text;
using System.Text.RegularExpressions;
using Maenifold.Utils;
using NUnit.Framework;

namespace Maenifold.Tests;

/// <summary>
/// Targeted branch-coverage tests for four zero-coverage utility classes:
///   TimeZoneConverter, CultureInvariantHelpers, StringExtensions, StringBuilderExtensions.
///
/// T-COV-001.9: RTM FR-17.11
/// </summary>
[TestFixture]
public class UtilityClassTests
{
    // ═══════════════════════════════════════════════════════════════
    // TimeZoneConverter
    // ═══════════════════════════════════════════════════════════════

    #region TimeZoneConverter.ToLocalDisplay — null / whitespace branches

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — null input returns N/A")]
    public void TimeZoneConverter_ToLocalDisplay_NullInput_ReturnsNA()
    {
        var result = TimeZoneConverter.ToLocalDisplay(null);

        Assert.That(result, Is.EqualTo("N/A"));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — empty string returns N/A")]
    public void TimeZoneConverter_ToLocalDisplay_EmptyString_ReturnsNA()
    {
        var result = TimeZoneConverter.ToLocalDisplay(string.Empty);

        Assert.That(result, Is.EqualTo("N/A"));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — whitespace-only string returns N/A")]
    public void TimeZoneConverter_ToLocalDisplay_WhitespaceString_ReturnsNA()
    {
        var result = TimeZoneConverter.ToLocalDisplay("   ");

        Assert.That(result, Is.EqualTo("N/A"));
    }

    #endregion

    #region TimeZoneConverter.ToLocalDisplay — valid ISO 8601 timestamps

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — valid UTC ISO timestamp is formatted correctly")]
    public void TimeZoneConverter_ToLocalDisplay_ValidUtcTimestamp_ReturnsFormattedString()
    {
        const string iso = "2026-02-18T10:30:00Z";

        var result = TimeZoneConverter.ToLocalDisplay(iso);

        Assert.That(result, Is.Not.EqualTo("N/A"));
        Assert.That(result, Is.Not.EqualTo("Invalid date"));

        // Must match the output format pattern: yyyy-MM-dd HH:mm:ss ±HH:mm
        var pattern = new Regex(@"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} [+-]\d{2}:\d{2}$");
        Assert.That(pattern.IsMatch(result), Is.True,
            $"Expected yyyy-MM-dd HH:mm:ss zzz format, got: {result}");
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — timestamp with explicit offset is formatted")]
    public void TimeZoneConverter_ToLocalDisplay_TimestampWithOffset_ReturnsFormattedString()
    {
        const string iso = "2026-02-18T10:30:00+05:30";

        var result = TimeZoneConverter.ToLocalDisplay(iso);

        Assert.That(result, Is.Not.EqualTo("N/A"));
        Assert.That(result, Is.Not.EqualTo("Invalid date"));
    }

    #endregion

    #region TimeZoneConverter.ToLocalDisplay — invalid timestamps

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — non-date string returns Invalid date")]
    public void TimeZoneConverter_ToLocalDisplay_NonDateString_ReturnsInvalidDate()
    {
        var result = TimeZoneConverter.ToLocalDisplay("not-a-date");

        Assert.That(result, Is.EqualTo("Invalid date"));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — garbled timestamp returns Invalid date")]
    public void TimeZoneConverter_ToLocalDisplay_GarbledTimestamp_ReturnsInvalidDate()
    {
        var result = TimeZoneConverter.ToLocalDisplay("2026-99-99T99:99:99Z");

        Assert.That(result, Is.EqualTo("Invalid date"));
    }

    #endregion

    #region TimeZoneConverter.GetUtcNowIso

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — GetUtcNowIso returns parseable ISO 8601 string")]
    public void TimeZoneConverter_GetUtcNowIso_ReturnsParsableDateTimeOffset()
    {
        var before = DateTimeOffset.UtcNow.AddSeconds(-1);

        var result = TimeZoneConverter.GetUtcNowIso();

        var after = DateTimeOffset.UtcNow.AddSeconds(1);
        var parsed = DateTimeOffset.Parse(result, CultureInfo.InvariantCulture);

        Assert.That(parsed >= before, Is.True, $"Parsed time {parsed} should be >= {before}");
        Assert.That(parsed <= after, Is.True, $"Parsed time {parsed} should be <= {after}");
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — GetUtcNowIso result contains UTC offset indicator")]
    public void TimeZoneConverter_GetUtcNowIso_ResultContainsUtcIndicator()
    {
        var result = TimeZoneConverter.GetUtcNowIso();

        // ISO 8601 round-trip format ("O") includes "+00:00" or "Z" for UTC
        Assert.That(result, Does.Contain("+00:00").Or.Contain("Z"),
            $"Expected UTC indicator in ISO string, got: {result}");
    }

    #endregion

    #region TimeZoneConverter.FileTimeToIso

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — FileTimeToIso returns parseable ISO 8601 string")]
    public void TimeZoneConverter_FileTimeToIso_UtcNow_ReturnsParsableIso()
    {
        var fileTime = DateTime.UtcNow;

        var result = TimeZoneConverter.FileTimeToIso(fileTime);

        var parsed = DateTimeOffset.Parse(result, CultureInfo.InvariantCulture);
        Assert.That(parsed.UtcDateTime, Is.EqualTo(fileTime).Within(TimeSpan.FromMilliseconds(1)));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — FileTimeToIso uses UTC offset zero for known DateTime")]
    public void TimeZoneConverter_FileTimeToIso_KnownDateTime_HasZeroOffset()
    {
        var fileTime = new DateTime(2026, 2, 18, 10, 30, 0, DateTimeKind.Utc);

        var result = TimeZoneConverter.FileTimeToIso(fileTime);

        var parsed = DateTimeOffset.Parse(result, CultureInfo.InvariantCulture);
        Assert.That(parsed.Offset, Is.EqualTo(TimeSpan.Zero),
            "FileTimeToIso should always produce UTC offset +00:00");
        Assert.That(parsed.UtcDateTime, Is.EqualTo(fileTime));
    }

    #endregion

    // ═══════════════════════════════════════════════════════════════
    // CultureInvariantHelpers
    // ═══════════════════════════════════════════════════════════════

    #region CultureInvariantHelpers.ParseDateTime

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ParseDateTime parses a valid date string")]
    public void CultureInvariant_ParseDateTime_ValidDateString_ReturnsDateTime()
    {
        var result = CultureInvariantHelpers.ParseDateTime("2026-02-18");

        Assert.That(result.Year, Is.EqualTo(2026));
        Assert.That(result.Month, Is.EqualTo(2));
        Assert.That(result.Day, Is.EqualTo(18));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ParseDateTime parses ISO date-time string")]
    public void CultureInvariant_ParseDateTime_ValidIsoDateTime_ReturnsCorrectValue()
    {
        var result = CultureInvariantHelpers.ParseDateTime("2026-02-18T10:30:00");

        Assert.That(result, Is.EqualTo(new DateTime(2026, 2, 18, 10, 30, 0)));
    }

    #endregion

    #region CultureInvariantHelpers.TryParseDateTime

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — TryParseDateTime returns true for valid input")]
    public void CultureInvariant_TryParseDateTime_ValidDateString_ReturnsTrueAndCorrectValue()
    {
        var success = CultureInvariantHelpers.TryParseDateTime("2026-02-18", out var result);

        Assert.That(success, Is.True);
        Assert.That(result.Year, Is.EqualTo(2026));
        Assert.That(result.Month, Is.EqualTo(2));
        Assert.That(result.Day, Is.EqualTo(18));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — TryParseDateTime returns false for invalid input")]
    public void CultureInvariant_TryParseDateTime_InvalidString_ReturnsFalse()
    {
        var success = CultureInvariantHelpers.TryParseDateTime("not-a-date", out var result);

        Assert.That(success, Is.False);
        Assert.That(result, Is.EqualTo(default(DateTime)));
    }

    #endregion

    #region CultureInvariantHelpers.ParseLong

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ParseLong parses a large integer string")]
    public void CultureInvariant_ParseLong_ValidLongString_ReturnsLong()
    {
        var result = CultureInvariantHelpers.ParseLong("1234567890");

        Assert.That(result, Is.EqualTo(1234567890L));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ParseLong parses max long value")]
    public void CultureInvariant_ParseLong_MaxLongValue_ReturnsMaxLong()
    {
        var result = CultureInvariantHelpers.ParseLong("9223372036854775807");

        Assert.That(result, Is.EqualTo(long.MaxValue));
    }

    #endregion

    #region CultureInvariantHelpers.ParseInt

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ParseInt parses a simple integer string")]
    public void CultureInvariant_ParseInt_ValidIntString_ReturnsInt()
    {
        var result = CultureInvariantHelpers.ParseInt("42");

        Assert.That(result, Is.EqualTo(42));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ParseInt parses negative integer")]
    public void CultureInvariant_ParseInt_NegativeIntString_ReturnsNegativeInt()
    {
        var result = CultureInvariantHelpers.ParseInt("-7");

        Assert.That(result, Is.EqualTo(-7));
    }

    #endregion

    #region CultureInvariantHelpers.ToInt32

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ToInt32 converts boxed int to int")]
    public void CultureInvariant_ToInt32_BoxedInt_ReturnsInt()
    {
        var result = CultureInvariantHelpers.ToInt32((object)42);

        Assert.That(result, Is.EqualTo(42));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ToInt32 converts boxed long to int")]
    public void CultureInvariant_ToInt32_BoxedLong_ReturnsInt()
    {
        var result = CultureInvariantHelpers.ToInt32((object)100L);

        Assert.That(result, Is.EqualTo(100));
    }

    #endregion

    #region CultureInvariantHelpers.FormatDateTime

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — FormatDateTime formats with yyyy-MM-dd pattern")]
    public void CultureInvariant_FormatDateTime_DateOnlyFormat_ReturnsFormattedString()
    {
        var dt = new DateTime(2026, 2, 18, 10, 30, 0);

        var result = CultureInvariantHelpers.FormatDateTime(dt, "yyyy-MM-dd");

        Assert.That(result, Is.EqualTo("2026-02-18"));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — FormatDateTime formats with full date-time pattern")]
    public void CultureInvariant_FormatDateTime_FullDateTimeFormat_ReturnsFormattedString()
    {
        var dt = new DateTime(2026, 2, 18, 10, 30, 0);

        var result = CultureInvariantHelpers.FormatDateTime(dt, "yyyy-MM-dd HH:mm:ss");

        Assert.That(result, Is.EqualTo("2026-02-18 10:30:00"));
    }

    #endregion

    #region CultureInvariantHelpers.FormatDateTimeOffset

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — FormatDateTimeOffset formats with round-trip format")]
    public void CultureInvariant_FormatDateTimeOffset_RoundTripFormat_ReturnsIsoString()
    {
        var dto = new DateTimeOffset(2026, 2, 18, 10, 30, 0, TimeSpan.Zero);

        var result = CultureInvariantHelpers.FormatDateTimeOffset(dto, "O");

        var reparsed = DateTimeOffset.Parse(result, CultureInfo.InvariantCulture);
        Assert.That(reparsed, Is.EqualTo(dto));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — FormatDateTimeOffset formats with date-only pattern")]
    public void CultureInvariant_FormatDateTimeOffset_DateOnlyFormat_ReturnsDatePart()
    {
        var dto = new DateTimeOffset(2026, 2, 18, 10, 30, 0, TimeSpan.Zero);

        var result = CultureInvariantHelpers.FormatDateTimeOffset(dto, "yyyy-MM-dd");

        Assert.That(result, Is.EqualTo("2026-02-18"));
    }

    #endregion

    #region CultureInvariantHelpers.ToString(int) and ToString(long)

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ToString(int) converts integer to invariant string")]
    public void CultureInvariant_ToString_Int_ReturnsInvariantString()
    {
        var result = CultureInvariantHelpers.ToString(42);

        Assert.That(result, Is.EqualTo("42"));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ToString(int) handles negative value")]
    public void CultureInvariant_ToString_NegativeInt_ReturnsInvariantString()
    {
        var result = CultureInvariantHelpers.ToString(-99);

        Assert.That(result, Is.EqualTo("-99"));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ToString(long) converts long to invariant string")]
    public void CultureInvariant_ToString_Long_ReturnsInvariantString()
    {
        var result = CultureInvariantHelpers.ToString(1234567890L);

        Assert.That(result, Is.EqualTo("1234567890"));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ToString(long) handles max long value")]
    public void CultureInvariant_ToString_MaxLong_ReturnsInvariantString()
    {
        var result = CultureInvariantHelpers.ToString(long.MaxValue);

        Assert.That(result, Is.EqualTo("9223372036854775807"));
    }

    #endregion

    // ═══════════════════════════════════════════════════════════════
    // StringExtensions
    // ═══════════════════════════════════════════════════════════════

    #region StringExtensions.StartsWithOrdinal (case-sensitive)

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — StartsWithOrdinal returns true when prefix matches exactly")]
    public void StringExtensions_StartsWithOrdinal_MatchingPrefix_ReturnsTrue()
    {
        Assert.That("Hello".StartsWithOrdinal("Hel"), Is.True);
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — StartsWithOrdinal returns false when case differs")]
    public void StringExtensions_StartsWithOrdinal_DifferentCase_ReturnsFalse()
    {
        Assert.That("Hello".StartsWithOrdinal("hel"), Is.False);
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — StartsWithOrdinal returns false when prefix is absent")]
    public void StringExtensions_StartsWithOrdinal_NoMatch_ReturnsFalse()
    {
        Assert.That("Hello".StartsWithOrdinal("World"), Is.False);
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — StartsWithOrdinal with empty prefix always returns true")]
    public void StringExtensions_StartsWithOrdinal_EmptyPrefix_ReturnsTrue()
    {
        Assert.That("Hello".StartsWithOrdinal(string.Empty), Is.True);
    }

    #endregion

    #region StringExtensions.EndsWithOrdinal (case-sensitive)

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — EndsWithOrdinal returns true when suffix matches exactly")]
    public void StringExtensions_EndsWithOrdinal_MatchingSuffix_ReturnsTrue()
    {
        Assert.That("Hello".EndsWithOrdinal("llo"), Is.True);
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — EndsWithOrdinal returns false when case differs")]
    public void StringExtensions_EndsWithOrdinal_DifferentCase_ReturnsFalse()
    {
        Assert.That("Hello".EndsWithOrdinal("LLO"), Is.False);
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — EndsWithOrdinal returns false when suffix is absent")]
    public void StringExtensions_EndsWithOrdinal_NoMatch_ReturnsFalse()
    {
        Assert.That("Hello".EndsWithOrdinal("World"), Is.False);
    }

    #endregion

    #region StringExtensions.ContainsOrdinal (case-sensitive)

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ContainsOrdinal returns true when substring is present")]
    public void StringExtensions_ContainsOrdinal_MatchingSubstring_ReturnsTrue()
    {
        Assert.That("Hello World".ContainsOrdinal("lo W"), Is.True);
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ContainsOrdinal returns false when case differs")]
    public void StringExtensions_ContainsOrdinal_DifferentCase_ReturnsFalse()
    {
        Assert.That("Hello World".ContainsOrdinal("LO W"), Is.False);
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ContainsOrdinal returns false when substring is absent")]
    public void StringExtensions_ContainsOrdinal_NoMatch_ReturnsFalse()
    {
        Assert.That("Hello World".ContainsOrdinal("xyz"), Is.False);
    }

    #endregion

    #region StringExtensions.StartsWithOrdinalIgnoreCase

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — StartsWithOrdinalIgnoreCase returns true for lower-case prefix")]
    public void StringExtensions_StartsWithOrdinalIgnoreCase_LowerCasePrefix_ReturnsTrue()
    {
        Assert.That("Hello".StartsWithOrdinalIgnoreCase("hel"), Is.True);
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — StartsWithOrdinalIgnoreCase returns true for exact-case prefix")]
    public void StringExtensions_StartsWithOrdinalIgnoreCase_ExactCasePrefix_ReturnsTrue()
    {
        Assert.That("Hello".StartsWithOrdinalIgnoreCase("Hel"), Is.True);
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — StartsWithOrdinalIgnoreCase returns false when prefix is absent")]
    public void StringExtensions_StartsWithOrdinalIgnoreCase_NoMatch_ReturnsFalse()
    {
        Assert.That("Hello".StartsWithOrdinalIgnoreCase("World"), Is.False);
    }

    #endregion

    #region StringExtensions.EndsWithOrdinalIgnoreCase

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — EndsWithOrdinalIgnoreCase returns true for upper-case suffix")]
    public void StringExtensions_EndsWithOrdinalIgnoreCase_UpperCaseSuffix_ReturnsTrue()
    {
        Assert.That("Hello".EndsWithOrdinalIgnoreCase("LLO"), Is.True);
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — EndsWithOrdinalIgnoreCase returns true for lower-case suffix")]
    public void StringExtensions_EndsWithOrdinalIgnoreCase_LowerCaseSuffix_ReturnsTrue()
    {
        Assert.That("Hello".EndsWithOrdinalIgnoreCase("llo"), Is.True);
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — EndsWithOrdinalIgnoreCase returns false when suffix is absent")]
    public void StringExtensions_EndsWithOrdinalIgnoreCase_NoMatch_ReturnsFalse()
    {
        Assert.That("Hello".EndsWithOrdinalIgnoreCase("World"), Is.False);
    }

    #endregion

    #region StringExtensions.ContainsOrdinalIgnoreCase

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ContainsOrdinalIgnoreCase returns true for mixed-case substring")]
    public void StringExtensions_ContainsOrdinalIgnoreCase_MixedCase_ReturnsTrue()
    {
        Assert.That("Hello World".ContainsOrdinalIgnoreCase("LO W"), Is.True);
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ContainsOrdinalIgnoreCase returns true for exact-case match")]
    public void StringExtensions_ContainsOrdinalIgnoreCase_ExactMatch_ReturnsTrue()
    {
        Assert.That("Hello World".ContainsOrdinalIgnoreCase("lo W"), Is.True);
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — ContainsOrdinalIgnoreCase returns false when substring is absent")]
    public void StringExtensions_ContainsOrdinalIgnoreCase_NoMatch_ReturnsFalse()
    {
        Assert.That("Hello World".ContainsOrdinalIgnoreCase("xyz"), Is.False);
    }

    #endregion

    // ═══════════════════════════════════════════════════════════════
    // StringBuilderExtensions
    // ═══════════════════════════════════════════════════════════════

    #region StringBuilderExtensions.AppendInvariant

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — AppendInvariant produces invariant decimal notation")]
    public void StringBuilderExtensions_AppendInvariant_DecimalInterpolation_UsesInvariantCulture()
    {
        var sb = new StringBuilder();
        double value = 1.5;

        sb.AppendInvariant($"value={value}");

        // Invariant culture uses '.' as decimal separator regardless of OS locale
        Assert.That(sb.ToString(), Is.EqualTo("value=1.5"));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — AppendInvariant returns same StringBuilder for fluent chaining")]
    public void StringBuilderExtensions_AppendInvariant_ReturnsSameStringBuilder()
    {
        var sb = new StringBuilder();

        var returned = sb.AppendInvariant($"hello");

        Assert.That(returned, Is.SameAs(sb));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — AppendInvariant supports fluent chaining")]
    public void StringBuilderExtensions_AppendInvariant_FluentChaining_ProducesCorrectResult()
    {
        var sb = new StringBuilder();

        sb.AppendInvariant($"a=").AppendInvariant($"b");

        Assert.That(sb.ToString(), Is.EqualTo("a=b"));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — AppendInvariant with integer interpolation")]
    public void StringBuilderExtensions_AppendInvariant_IntegerInterpolation_ProducesCorrectResult()
    {
        var sb = new StringBuilder();
        int count = 42;

        sb.AppendInvariant($"count={count}");

        Assert.That(sb.ToString(), Is.EqualTo("count=42"));
    }

    #endregion

    #region StringBuilderExtensions.AppendLineInvariant

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — AppendLineInvariant appends string followed by newline")]
    public void StringBuilderExtensions_AppendLineInvariant_StringInterpolation_AppendsNewline()
    {
        var sb = new StringBuilder();

        sb.AppendLineInvariant($"hello");

        Assert.That(sb.ToString(), Is.EqualTo($"hello{Environment.NewLine}"));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — AppendLineInvariant produces invariant decimal notation")]
    public void StringBuilderExtensions_AppendLineInvariant_DecimalInterpolation_UsesInvariantCulture()
    {
        var sb = new StringBuilder();
        double value = 3.14;

        sb.AppendLineInvariant($"pi={value}");

        Assert.That(sb.ToString(), Does.StartWith("pi=3.14"));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — AppendLineInvariant returns same StringBuilder for fluent chaining")]
    public void StringBuilderExtensions_AppendLineInvariant_ReturnsSameStringBuilder()
    {
        var sb = new StringBuilder();

        var returned = sb.AppendLineInvariant($"line");

        Assert.That(returned, Is.SameAs(sb));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — AppendLineInvariant supports multiple chained lines")]
    public void StringBuilderExtensions_AppendLineInvariant_MultipleLines_ProducesCorrectResult()
    {
        var sb = new StringBuilder();

        sb.AppendLineInvariant($"line1")
          .AppendLineInvariant($"line2");

        var result = sb.ToString();
        Assert.That(result, Does.Contain("line1"));
        Assert.That(result, Does.Contain("line2"));
    }

    #endregion

    #region StringBuilderExtensions.AppendFormatInvariant

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — AppendFormatInvariant applies format string with args")]
    public void StringBuilderExtensions_AppendFormatInvariant_FormatWithArgs_ProducesCorrectResult()
    {
        var sb = new StringBuilder();

        sb.AppendFormatInvariant("x={0}, y={1}", 10, 20);

        Assert.That(sb.ToString(), Is.EqualTo("x=10, y=20"));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — AppendFormatInvariant uses invariant culture for decimals")]
    public void StringBuilderExtensions_AppendFormatInvariant_DecimalArg_UsesInvariantCulture()
    {
        var sb = new StringBuilder();

        sb.AppendFormatInvariant("{0:F2}", 1.5);

        Assert.That(sb.ToString(), Is.EqualTo("1.50"));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — AppendFormatInvariant returns same StringBuilder for fluent chaining")]
    public void StringBuilderExtensions_AppendFormatInvariant_ReturnsSameStringBuilder()
    {
        var sb = new StringBuilder();

        var returned = sb.AppendFormatInvariant("{0}", "test");

        Assert.That(returned, Is.SameAs(sb));
    }

    [Test]
    [Description("T-COV-001.9: RTM FR-17.11 — AppendFormatInvariant supports fluent chaining with AppendInvariant")]
    public void StringBuilderExtensions_AppendFormatInvariant_ChainingWithAppendInvariant_ProducesCorrectResult()
    {
        var sb = new StringBuilder();

        sb.AppendFormatInvariant("{0}", "prefix-")
          .AppendInvariant($"suffix");

        Assert.That(sb.ToString(), Is.EqualTo("prefix-suffix"));
    }

    #endregion
}
