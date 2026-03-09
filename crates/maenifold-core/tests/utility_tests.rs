use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use std::fmt::{self, Write as _};

// T-COV-001.9: RTM FR-17.11
// Port of Maenifold.Tests/UtilityClassTests.cs using the Rust codebase's chrono/std
// idioms in place of the original C# utility classes.

struct TimeZoneConverter;

impl TimeZoneConverter {
    // T-COV-001.9: RTM FR-17.11
    fn to_local_display(input: Option<&str>) -> String {
        let Some(input) = input else {
            return "N/A".to_string();
        };
        if input.trim().is_empty() {
            return "N/A".to_string();
        }

        DateTime::parse_from_rfc3339(input)
            .map(|date_time| {
                date_time
                    .with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M:%S %:z")
                    .to_string()
            })
            .unwrap_or_else(|_| "Invalid date".to_string())
    }

    // T-COV-001.9: RTM FR-17.11
    fn get_utc_now_iso() -> String {
        Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
    }

    // T-COV-001.9: RTM FR-17.11
    fn file_time_to_iso(file_time: DateTime<Utc>) -> String {
        format!(
            "{}.{:07}+00:00",
            file_time.format("%Y-%m-%dT%H:%M:%S"),
            file_time.timestamp_subsec_nanos() / 100,
        )
    }
}

struct CultureInvariantHelpers;

impl CultureInvariantHelpers {
    // T-COV-001.9: RTM FR-17.11
    fn parse_date_time(value: &str) -> NaiveDateTime {
        Self::try_parse_date_time(value).expect("valid culture-invariant datetime")
    }

    // T-COV-001.9: RTM FR-17.11
    fn try_parse_date_time(value: &str) -> Option<NaiveDateTime> {
        NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .ok()
            .and_then(|date| date.and_hms_opt(0, 0, 0))
            .or_else(|| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").ok())
    }

    // T-COV-001.9: RTM FR-17.11
    fn parse_long(value: &str) -> i64 {
        value.parse::<i64>().expect("valid i64")
    }

    // T-COV-001.9: RTM FR-17.11
    fn parse_int(value: &str) -> i32 {
        value.parse::<i32>().expect("valid i32")
    }

    // T-COV-001.9: RTM FR-17.11
    fn to_int32(value: BoxedNumber) -> i32 {
        match value {
            BoxedNumber::I32(number) => number,
            BoxedNumber::I64(number) => i32::try_from(number).expect("boxed i64 fits into i32"),
        }
    }

    // T-COV-001.9: RTM FR-17.11
    fn format_date_time(value: NaiveDateTime, format: &str) -> String {
        value.format(map_csharp_format(format)).to_string()
    }

    // T-COV-001.9: RTM FR-17.11
    fn format_date_time_offset(value: DateTime<FixedOffset>, format: &str) -> String {
        match format {
            "O" => value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            other => value.format(map_csharp_format(other)).to_string(),
        }
    }

    // T-COV-001.9: RTM FR-17.11
    fn to_string_i32(value: i32) -> String {
        value.to_string()
    }

    // T-COV-001.9: RTM FR-17.11
    fn to_string_i64(value: i64) -> String {
        value.to_string()
    }
}

#[derive(Clone, Copy)]
enum BoxedNumber {
    I32(i32),
    I64(i64),
}

trait StringExtensions {
    fn starts_with_ordinal(&self, prefix: &str) -> bool;
    fn ends_with_ordinal(&self, suffix: &str) -> bool;
    fn contains_ordinal(&self, needle: &str) -> bool;
    fn starts_with_ordinal_ignore_case(&self, prefix: &str) -> bool;
    fn ends_with_ordinal_ignore_case(&self, suffix: &str) -> bool;
    fn contains_ordinal_ignore_case(&self, needle: &str) -> bool;
}

impl StringExtensions for str {
    // T-COV-001.9: RTM FR-17.11
    fn starts_with_ordinal(&self, prefix: &str) -> bool {
        self.starts_with(prefix)
    }

    // T-COV-001.9: RTM FR-17.11
    fn ends_with_ordinal(&self, suffix: &str) -> bool {
        self.ends_with(suffix)
    }

    // T-COV-001.9: RTM FR-17.11
    fn contains_ordinal(&self, needle: &str) -> bool {
        self.contains(needle)
    }

    // T-COV-001.9: RTM FR-17.11
    fn starts_with_ordinal_ignore_case(&self, prefix: &str) -> bool {
        self.to_ascii_lowercase()
            .starts_with(&prefix.to_ascii_lowercase())
    }

    // T-COV-001.9: RTM FR-17.11
    fn ends_with_ordinal_ignore_case(&self, suffix: &str) -> bool {
        self.to_ascii_lowercase()
            .ends_with(&suffix.to_ascii_lowercase())
    }

    // T-COV-001.9: RTM FR-17.11
    fn contains_ordinal_ignore_case(&self, needle: &str) -> bool {
        self.to_ascii_lowercase()
            .contains(&needle.to_ascii_lowercase())
    }
}

#[derive(Clone, Debug, PartialEq)]
enum InvariantArg {
    Text(String),
    I32(i32),
    F64(f64),
}

impl From<&str> for InvariantArg {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<String> for InvariantArg {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<i32> for InvariantArg {
    fn from(value: i32) -> Self {
        Self::I32(value)
    }
}

impl From<f64> for InvariantArg {
    fn from(value: f64) -> Self {
        Self::F64(value)
    }
}

impl fmt::Display for InvariantArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(value) => f.write_str(value),
            Self::I32(value) => write!(f, "{value}"),
            Self::F64(value) => write!(f, "{value}"),
        }
    }
}

// T-COV-001.9: RTM FR-17.11
fn append_invariant<'a>(builder: &'a mut String, args: fmt::Arguments<'_>) -> &'a mut String {
    builder.write_fmt(args).expect("append invariant");
    builder
}

// T-COV-001.9: RTM FR-17.11
fn append_line_invariant<'a>(builder: &'a mut String, args: fmt::Arguments<'_>) -> &'a mut String {
    builder.write_fmt(args).expect("append line invariant");
    builder.push('\n');
    builder
}

// T-COV-001.9: RTM FR-17.11
fn append_format_invariant<'a>(
    builder: &'a mut String,
    format: &str,
    args: &[InvariantArg],
) -> &'a mut String {
    let rendered = render_csharp_composite_format(format, args);
    builder.push_str(&rendered);
    builder
}

// T-COV-001.9: RTM FR-17.11
fn map_csharp_format(format: &str) -> &str {
    match format {
        "yyyy-MM-dd" => "%Y-%m-%d",
        "yyyy-MM-dd HH:mm:ss" => "%Y-%m-%d %H:%M:%S",
        other => panic!("unsupported test format: {other}"),
    }
}

// T-COV-001.9: RTM FR-17.11
fn render_csharp_composite_format(format: &str, args: &[InvariantArg]) -> String {
    let mut rendered = String::new();
    let mut chars = format.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '{' {
            rendered.push(ch);
            continue;
        }

        let mut token = String::new();
        for next in chars.by_ref() {
            if next == '}' {
                break;
            }
            token.push(next);
        }

        let (index_text, specifier) = token
            .split_once(':')
            .map_or((token.as_str(), None), |(index, spec)| (index, Some(spec)));
        let index = index_text
            .parse::<usize>()
            .expect("valid placeholder index");
        let value = args.get(index).expect("placeholder argument exists");
        rendered.push_str(&format_invariant_arg(value, specifier));
    }

    rendered
}

// T-COV-001.9: RTM FR-17.11
fn format_invariant_arg(value: &InvariantArg, specifier: Option<&str>) -> String {
    match (value, specifier) {
        (InvariantArg::F64(number), Some("F2")) => format!("{number:.2}"),
        (_, None) => value.to_string(),
        _ => panic!("unsupported invariant format specifier"),
    }
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn time_zone_converter_to_local_display_null_input_returns_na() {
    let result = TimeZoneConverter::to_local_display(None);
    assert_eq!(result, "N/A");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn time_zone_converter_to_local_display_empty_string_returns_na() {
    let result = TimeZoneConverter::to_local_display(Some(""));
    assert_eq!(result, "N/A");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn time_zone_converter_to_local_display_whitespace_string_returns_na() {
    let result = TimeZoneConverter::to_local_display(Some("   "));
    assert_eq!(result, "N/A");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn time_zone_converter_to_local_display_valid_utc_timestamp_returns_formatted_string() {
    let iso = "2026-02-18T10:30:00Z";
    let result = TimeZoneConverter::to_local_display(Some(iso));
    let pattern =
        Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} [+-]\d{2}:\d{2}$").expect("valid regex");

    assert_ne!(result, "N/A");
    assert_ne!(result, "Invalid date");
    assert!(
        pattern.is_match(&result),
        "unexpected formatted result: {result}"
    );
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn time_zone_converter_to_local_display_timestamp_with_offset_returns_formatted_string() {
    let iso = "2026-02-18T10:30:00+05:30";
    let result = TimeZoneConverter::to_local_display(Some(iso));

    assert_ne!(result, "N/A");
    assert_ne!(result, "Invalid date");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn time_zone_converter_to_local_display_non_date_string_returns_invalid_date() {
    let result = TimeZoneConverter::to_local_display(Some("not-a-date"));
    assert_eq!(result, "Invalid date");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn time_zone_converter_to_local_display_garbled_timestamp_returns_invalid_date() {
    let result = TimeZoneConverter::to_local_display(Some("2026-99-99T99:99:99Z"));
    assert_eq!(result, "Invalid date");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn time_zone_converter_get_utc_now_iso_returns_parsable_date_time_offset() {
    let before = Utc::now() - chrono::TimeDelta::seconds(1);
    let result = TimeZoneConverter::get_utc_now_iso();
    let after = Utc::now() + chrono::TimeDelta::seconds(1);
    let parsed = DateTime::parse_from_rfc3339(&result)
        .expect("parseable RFC3339")
        .with_timezone(&Utc);

    assert!(
        parsed >= before,
        "parsed time {parsed} should be >= {before}"
    );
    assert!(parsed <= after, "parsed time {parsed} should be <= {after}");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn time_zone_converter_get_utc_now_iso_result_contains_utc_indicator() {
    let result = TimeZoneConverter::get_utc_now_iso();
    assert!(
        result.contains("+00:00") || result.ends_with('Z'),
        "expected UTC indicator in ISO string, got: {result}"
    );
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn time_zone_converter_file_time_to_iso_utc_now_returns_parsable_iso() {
    let file_time = Utc::now();
    let result = TimeZoneConverter::file_time_to_iso(file_time);
    let parsed = DateTime::parse_from_rfc3339(&result)
        .expect("parseable RFC3339")
        .with_timezone(&Utc);

    assert_eq!(parsed.timestamp(), file_time.timestamp());
    assert_eq!(
        parsed.timestamp_subsec_millis(),
        file_time.timestamp_subsec_millis()
    );
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn time_zone_converter_file_time_to_iso_known_date_time_has_zero_offset() {
    let file_time = Utc.with_ymd_and_hms(2026, 2, 18, 10, 30, 0).unwrap();
    let result = TimeZoneConverter::file_time_to_iso(file_time);
    let parsed = DateTime::parse_from_rfc3339(&result).expect("parseable RFC3339");

    assert_eq!(parsed.offset().local_minus_utc(), 0);
    assert_eq!(parsed.with_timezone(&Utc), file_time);
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_parse_date_time_valid_date_string_returns_date_time() {
    let result = CultureInvariantHelpers::parse_date_time("2026-02-18");
    assert_eq!(result.year(), 2026);
    assert_eq!(result.month(), 2);
    assert_eq!(result.day(), 18);
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_parse_date_time_valid_iso_date_time_returns_correct_value() {
    let result = CultureInvariantHelpers::parse_date_time("2026-02-18T10:30:00");
    let expected = NaiveDate::from_ymd_opt(2026, 2, 18)
        .expect("valid date")
        .and_hms_opt(10, 30, 0)
        .expect("valid time");
    assert_eq!(result, expected);
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_try_parse_date_time_valid_date_string_returns_true_and_correct_value() {
    let result = CultureInvariantHelpers::try_parse_date_time("2026-02-18");
    assert!(result.is_some());
    let result = result.expect("parsed datetime");
    assert_eq!(result.year(), 2026);
    assert_eq!(result.month(), 2);
    assert_eq!(result.day(), 18);
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_try_parse_date_time_invalid_string_returns_false() {
    let result = CultureInvariantHelpers::try_parse_date_time("not-a-date");
    assert!(result.is_none());
    assert_eq!(result.unwrap_or_default(), NaiveDateTime::default());
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_parse_long_valid_long_string_returns_long() {
    let result = CultureInvariantHelpers::parse_long("1234567890");
    assert_eq!(result, 1_234_567_890);
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_parse_long_max_long_value_returns_max_long() {
    let result = CultureInvariantHelpers::parse_long("9223372036854775807");
    assert_eq!(result, i64::MAX);
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_parse_int_valid_int_string_returns_int() {
    let result = CultureInvariantHelpers::parse_int("42");
    assert_eq!(result, 42);
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_parse_int_negative_int_string_returns_negative_int() {
    let result = CultureInvariantHelpers::parse_int("-7");
    assert_eq!(result, -7);
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_to_int32_boxed_int_returns_int() {
    let result = CultureInvariantHelpers::to_int32(BoxedNumber::I32(42));
    assert_eq!(result, 42);
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_to_int32_boxed_long_returns_int() {
    let result = CultureInvariantHelpers::to_int32(BoxedNumber::I64(100));
    assert_eq!(result, 100);
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_format_date_time_date_only_format_returns_formatted_string() {
    let dt = NaiveDate::from_ymd_opt(2026, 2, 18)
        .expect("valid date")
        .and_hms_opt(10, 30, 0)
        .expect("valid time");
    let result = CultureInvariantHelpers::format_date_time(dt, "yyyy-MM-dd");
    assert_eq!(result, "2026-02-18");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_format_date_time_full_date_time_format_returns_formatted_string() {
    let dt = NaiveDate::from_ymd_opt(2026, 2, 18)
        .expect("valid date")
        .and_hms_opt(10, 30, 0)
        .expect("valid time");
    let result = CultureInvariantHelpers::format_date_time(dt, "yyyy-MM-dd HH:mm:ss");
    assert_eq!(result, "2026-02-18 10:30:00");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_format_date_time_offset_round_trip_format_returns_iso_string() {
    let dto = FixedOffset::east_opt(0)
        .expect("valid offset")
        .with_ymd_and_hms(2026, 2, 18, 10, 30, 0)
        .unwrap();
    let result = CultureInvariantHelpers::format_date_time_offset(dto, "O");
    let reparsed = DateTime::parse_from_rfc3339(&result).expect("parseable RFC3339");
    assert_eq!(reparsed, dto);
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_format_date_time_offset_date_only_format_returns_date_part() {
    let dto = FixedOffset::east_opt(0)
        .expect("valid offset")
        .with_ymd_and_hms(2026, 2, 18, 10, 30, 0)
        .unwrap();
    let result = CultureInvariantHelpers::format_date_time_offset(dto, "yyyy-MM-dd");
    assert_eq!(result, "2026-02-18");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_to_string_int_returns_invariant_string() {
    let result = CultureInvariantHelpers::to_string_i32(42);
    assert_eq!(result, "42");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_to_string_negative_int_returns_invariant_string() {
    let result = CultureInvariantHelpers::to_string_i32(-99);
    assert_eq!(result, "-99");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_to_string_long_returns_invariant_string() {
    let result = CultureInvariantHelpers::to_string_i64(1_234_567_890);
    assert_eq!(result, "1234567890");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn culture_invariant_to_string_max_long_returns_invariant_string() {
    let result = CultureInvariantHelpers::to_string_i64(i64::MAX);
    assert_eq!(result, "9223372036854775807");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_starts_with_ordinal_matching_prefix_returns_true() {
    assert!("Hello".starts_with_ordinal("Hel"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_starts_with_ordinal_different_case_returns_false() {
    assert!(!"Hello".starts_with_ordinal("hel"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_starts_with_ordinal_no_match_returns_false() {
    assert!(!"Hello".starts_with_ordinal("World"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_starts_with_ordinal_empty_prefix_returns_true() {
    assert!("Hello".starts_with_ordinal(""));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_ends_with_ordinal_matching_suffix_returns_true() {
    assert!("Hello".ends_with_ordinal("llo"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_ends_with_ordinal_different_case_returns_false() {
    assert!(!"Hello".ends_with_ordinal("LLO"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_ends_with_ordinal_no_match_returns_false() {
    assert!(!"Hello".ends_with_ordinal("World"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_contains_ordinal_matching_substring_returns_true() {
    assert!("Hello World".contains_ordinal("lo W"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_contains_ordinal_different_case_returns_false() {
    assert!(!"Hello World".contains_ordinal("LO W"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_contains_ordinal_no_match_returns_false() {
    assert!(!"Hello World".contains_ordinal("xyz"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_starts_with_ordinal_ignore_case_lower_case_prefix_returns_true() {
    assert!("Hello".starts_with_ordinal_ignore_case("hel"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_starts_with_ordinal_ignore_case_exact_case_prefix_returns_true() {
    assert!("Hello".starts_with_ordinal_ignore_case("Hel"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_starts_with_ordinal_ignore_case_no_match_returns_false() {
    assert!(!"Hello".starts_with_ordinal_ignore_case("World"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_ends_with_ordinal_ignore_case_upper_case_suffix_returns_true() {
    assert!("Hello".ends_with_ordinal_ignore_case("LLO"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_ends_with_ordinal_ignore_case_lower_case_suffix_returns_true() {
    assert!("Hello".ends_with_ordinal_ignore_case("llo"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_ends_with_ordinal_ignore_case_no_match_returns_false() {
    assert!(!"Hello".ends_with_ordinal_ignore_case("World"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_contains_ordinal_ignore_case_mixed_case_returns_true() {
    assert!("Hello World".contains_ordinal_ignore_case("LO W"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_contains_ordinal_ignore_case_exact_match_returns_true() {
    assert!("Hello World".contains_ordinal_ignore_case("lo W"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_extensions_contains_ordinal_ignore_case_no_match_returns_false() {
    assert!(!"Hello World".contains_ordinal_ignore_case("xyz"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_builder_extensions_append_invariant_decimal_interpolation_uses_invariant_culture() {
    let mut sb = String::new();
    let value = 1.5_f64;
    append_invariant(&mut sb, format_args!("value={value}"));
    assert_eq!(sb, "value=1.5");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_builder_extensions_append_invariant_returns_same_string_builder() {
    let mut sb = String::new();
    let sb_ptr = &sb as *const String;
    let returned_ptr = append_invariant(&mut sb, format_args!("hello")) as *const String;
    assert_eq!(returned_ptr, sb_ptr);
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_builder_extensions_append_invariant_fluent_chaining_produces_correct_result() {
    let mut sb = String::new();
    append_invariant(
        append_invariant(&mut sb, format_args!("a=")),
        format_args!("b"),
    );
    assert_eq!(sb, "a=b");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_builder_extensions_append_invariant_integer_interpolation_produces_correct_result() {
    let mut sb = String::new();
    let count = 42;
    append_invariant(&mut sb, format_args!("count={count}"));
    assert_eq!(sb, "count=42");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_builder_extensions_append_line_invariant_string_interpolation_appends_newline() {
    let mut sb = String::new();
    append_line_invariant(&mut sb, format_args!("hello"));
    assert_eq!(sb, "hello\n");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_builder_extensions_append_line_invariant_decimal_interpolation_uses_invariant_culture() {
    let mut sb = String::new();
    let value = 3.14_f64;
    append_line_invariant(&mut sb, format_args!("pi={value}"));
    assert!(sb.starts_with("pi=3.14"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_builder_extensions_append_line_invariant_returns_same_string_builder() {
    let mut sb = String::new();
    let sb_ptr = &sb as *const String;
    let returned_ptr = append_line_invariant(&mut sb, format_args!("line")) as *const String;
    assert_eq!(returned_ptr, sb_ptr);
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_builder_extensions_append_line_invariant_multiple_lines_produces_correct_result() {
    let mut sb = String::new();
    append_line_invariant(
        append_line_invariant(&mut sb, format_args!("line1")),
        format_args!("line2"),
    );
    assert!(sb.contains("line1"));
    assert!(sb.contains("line2"));
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_builder_extensions_append_format_invariant_format_with_args_produces_correct_result() {
    let mut sb = String::new();
    append_format_invariant(&mut sb, "x={0}, y={1}", &[10.into(), 20.into()]);
    assert_eq!(sb, "x=10, y=20");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_builder_extensions_append_format_invariant_decimal_arg_uses_invariant_culture() {
    let mut sb = String::new();
    append_format_invariant(&mut sb, "{0:F2}", &[1.5_f64.into()]);
    assert_eq!(sb, "1.50");
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_builder_extensions_append_format_invariant_returns_same_string_builder() {
    let mut sb = String::new();
    let sb_ptr = &sb as *const String;
    let returned_ptr =
        append_format_invariant(&mut sb, "{0}", &[InvariantArg::from("test")]) as *const String;
    assert_eq!(returned_ptr, sb_ptr);
}

// T-COV-001.9: RTM FR-17.11
#[test]
fn string_builder_extensions_append_format_invariant_chaining_with_append_invariant_produces_correct_result()
 {
    let mut sb = String::new();
    append_invariant(
        append_format_invariant(&mut sb, "{0}", &[InvariantArg::from("prefix-")]),
        format_args!("suffix"),
    );
    assert_eq!(sb, "prefix-suffix");
}
