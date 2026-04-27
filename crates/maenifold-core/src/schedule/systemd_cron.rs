use anyhow::{Context, Result, bail};

pub(super) fn cron_to_oncalendar(cron: &str) -> Result<String> {
    let mut parts = cron.split_whitespace();
    let minute = parts
        .next()
        .context("cron expression is missing minute field")?;
    let hour = parts
        .next()
        .context("cron expression is missing hour field")?;
    let day = parts
        .next()
        .context("cron expression is missing day-of-month field")?;
    let month = parts
        .next()
        .context("cron expression is missing month field")?;
    let weekday = parts
        .next()
        .context("cron expression is missing day-of-week field")?;
    if parts.next().is_some() {
        bail!("systemd requires 5-field cron expressions");
    }

    let date = format!(
        "*-{}-{}",
        translate_numeric_field(month, 1, 12, 2)?,
        translate_numeric_field(day, 1, 31, 2)?
    );
    let time = format!(
        "{}:{}:00",
        translate_numeric_field(hour, 0, 23, 2)?,
        translate_numeric_field(minute, 0, 59, 2)?
    );
    let weekday = translate_weekday_field(weekday)?;

    Ok(if weekday == "*" {
        format!("{date} {time}")
    } else {
        format!("{weekday} {date} {time}")
    })
}
fn translate_numeric_field(expr: &str, min: u32, max: u32, width: usize) -> Result<String> {
    translate_field(expr, min, max, &|value| format!("{value:0width$}"))
}
fn translate_weekday_field(expr: &str) -> Result<String> {
    translate_field(expr, 0, 7, &|value| match value {
        0 | 7 => "Sun".to_string(),
        1 => "Mon".to_string(),
        2 => "Tue".to_string(),
        3 => "Wed".to_string(),
        4 => "Thu".to_string(),
        5 => "Fri".to_string(),
        6 => "Sat".to_string(),
        _ => value.to_string(),
    })
}
fn translate_field(
    expr: &str,
    min: u32,
    max: u32,
    render_value: &dyn Fn(u32) -> String,
) -> Result<String> {
    if expr == "*" {
        return Ok("*".to_string());
    }
    expr.split(',')
        .map(|segment| translate_segment(segment, min, max, render_value))
        .collect::<Result<Vec<_>>>()
        .map(|parts| parts.join(","))
}
fn translate_segment(
    segment: &str,
    min: u32,
    max: u32,
    render_value: &dyn Fn(u32) -> String,
) -> Result<String> {
    let (base, step) = match segment.split_once('/') {
        Some((base, step)) => {
            let step_value = parse_number(step, min, max)?;
            if step_value == 0 {
                bail!("cron step value cannot be zero in '{segment}'");
            }
            (base, Some(step_value))
        }
        None => (segment, None),
    };

    let translated = if base == "*" {
        "*".to_string()
    } else if let Some((start, end)) = base.split_once('-') {
        let start = parse_number(start, min, max)?;
        let end = parse_number(end, min, max)?;
        if start > end {
            bail!("invalid cron range '{segment}'");
        }
        format!("{}..{}", render_value(start), render_value(end))
    } else {
        render_value(parse_number(base, min, max)?)
    };

    Ok(match step {
        Some(step) => format!("{translated}/{step}"),
        None => translated,
    })
}
fn parse_number(input: &str, min: u32, max: u32) -> Result<u32> {
    let value = input
        .parse::<u32>()
        .with_context(|| format!("invalid cron value '{input}'"))?;
    if !(min..=max).contains(&value) {
        bail!("cron value '{input}' is out of range {min}-{max}");
    }
    Ok(value)
}
