use anyhow::{Result, anyhow, bail};
use croner::parser::CronParser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSchedule {
    pub cron: String,
    pub description: String,
}

/// Parse a natural language or raw cron expression into a validated 5-field cron string.
pub fn parse_schedule(input: &str) -> Result<ParsedSchedule> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        bail!("schedule expression cannot be empty");
    }

    let cron = natural_language_to_cron(trimmed).unwrap_or_else(|| trimmed.to_string());
    let description = describe_cron(&cron)?;

    Ok(ParsedSchedule { cron, description })
}

fn natural_language_to_cron(input: &str) -> Option<String> {
    let normalized = normalize_input(input);

    let time = parse_time_suffix(&normalized)?;
    let base = normalized
        .split_once(" at ")
        .map(|(prefix, _)| prefix)
        .unwrap_or(normalized.as_str());

    let cron = match base {
        "every minute" => "* * * * *".to_string(),
        "every hour" => "0 * * * *".to_string(),
        "every day" | "daily" => cron_with_time(0, 0, "*", "*", "*", time),
        "every weekday" => cron_with_time(0, 0, "*", "*", "1-5", time),
        "every week" | "weekly" => cron_with_time(0, 0, "*", "*", "0", time),
        "every month" | "monthly" => cron_with_time(0, 0, "1", "*", "*", time),
        _ => {
            if let Some(weekday) = parse_every_weekday(base) {
                cron_with_time(0, 0, "*", "*", weekday, time)
            } else {
                return None;
            }
        }
    };

    Some(cron)
}

fn cron_with_time(
    default_minute: u32,
    default_hour: u32,
    day_of_month: &str,
    month: &str,
    day_of_week: &str,
    time: Option<(u32, u32)>,
) -> String {
    let (minute, hour) = time.unwrap_or((default_minute, default_hour));
    format!("{minute} {hour} {day_of_month} {month} {day_of_week}")
}

fn parse_every_weekday(base: &str) -> Option<&'static str> {
    let day = base.strip_prefix("every ")?;
    match day {
        "monday" => Some("1"),
        "tuesday" => Some("2"),
        "wednesday" => Some("3"),
        "thursday" => Some("4"),
        "friday" => Some("5"),
        "saturday" => Some("6"),
        "sunday" => Some("0"),
        _ => None,
    }
}

fn parse_time_suffix(normalized: &str) -> Option<Option<(u32, u32)>> {
    let Some((_, raw_time)) = normalized.split_once(" at ") else {
        return Some(None);
    };

    let time = match raw_time {
        "midnight" => (0, 0),
        "noon" => (0, 12),
        _ => parse_clock_time(raw_time)?,
    };

    Some(Some(time))
}

fn parse_clock_time(raw: &str) -> Option<(u32, u32)> {
    if let Some(value) = raw.strip_suffix("am").or_else(|| raw.strip_suffix("pm")) {
        let is_pm = raw.ends_with("pm");
        let (hour, minute) = parse_hour_minute(value.trim(), false)?;
        if hour > 12 {
            return None;
        }
        let converted_hour = match (hour, is_pm) {
            (12, false) => 0,
            (12, true) => 12,
            (h, true) => h + 12,
            (h, false) => h,
        };
        return Some((minute, converted_hour));
    }

    let (hour, minute) = parse_hour_minute(raw, true)?;
    Some((minute, hour))
}

fn parse_hour_minute(raw: &str, allow_24_hour: bool) -> Option<(u32, u32)> {
    if let Some((hour, minute)) = raw.split_once(':') {
        let hour = hour.parse::<u32>().ok()?;
        let minute = minute.parse::<u32>().ok()?;
        let valid_hour = if allow_24_hour {
            hour < 24
        } else {
            (1..=12).contains(&hour)
        };
        if valid_hour && minute < 60 {
            return Some((hour, minute));
        }
        return None;
    }

    let hour = raw.parse::<u32>().ok()?;
    let valid_hour = if allow_24_hour {
        hour < 24
    } else {
        (1..=12).contains(&hour)
    };
    valid_hour.then_some((hour, 0))
}

fn normalize_input(input: &str) -> String {
    input
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn describe_cron(expr: &str) -> Result<String> {
    let cron = validate_cron(expr)?;
    let description = cron.describe().trim().trim_end_matches('.').to_string();
    if description.is_empty() {
        Ok(format!("Cron schedule: {expr}"))
    } else {
        Ok(description)
    }
}

fn validate_cron(expr: &str) -> Result<croner::Cron> {
    CronParser::new()
        .parse(expr)
        .map_err(|error| anyhow!("invalid cron: {error}"))
}

#[cfg(test)]
mod tests {
    use super::parse_schedule;

    #[test]
    fn parses_natural_language_daily_time() {
        let parsed = parse_schedule("daily at 3pm").expect("daily schedule should parse");
        assert_eq!(parsed.cron, "0 15 * * *");
        assert!(!parsed.description.is_empty());
    }

    #[test]
    fn parses_weekday_schedule() {
        let parsed = parse_schedule("every weekday at 9am").expect("weekday schedule should parse");
        assert_eq!(parsed.cron, "0 9 * * 1-5");
    }

    #[test]
    fn passes_through_raw_cron() {
        let parsed = parse_schedule("*/15 9-17 * * 1-5").expect("raw cron should parse");
        assert_eq!(parsed.cron, "*/15 9-17 * * 1-5");
    }

    #[test]
    fn rejects_invalid_natural_language_time() {
        let error = parse_schedule("daily at 25:00").expect_err("invalid time should fail");
        assert!(error.to_string().contains("invalid cron"));
    }
}
