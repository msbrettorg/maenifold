use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

pub(super) fn cron_to_start_calendar_interval_xml(cron: &str) -> Result<String> {
    let mut parts = cron.split_whitespace();
    let fields = [
        parts
            .next()
            .context("cron expression is missing minute field")?,
        parts
            .next()
            .context("cron expression is missing hour field")?,
        parts
            .next()
            .context("cron expression is missing day-of-month field")?,
        parts
            .next()
            .context("cron expression is missing month field")?,
        parts
            .next()
            .context("cron expression is missing day-of-week field")?,
    ];
    if parts.next().is_some() {
        bail!("launchd requires 5-field cron expressions");
    }
    let expanded = [
        ("Minute", expand_field(fields[0], 0, 59, false)?),
        ("Hour", expand_field(fields[1], 0, 23, false)?),
        ("Day", expand_field(fields[2], 1, 31, false)?),
        ("Month", expand_field(fields[3], 1, 12, false)?),
        ("Weekday", expand_field(fields[4], 0, 7, true)?),
    ];
    let schedules = expanded
        .iter()
        .fold(vec![BTreeMap::new()], |acc, (key, values)| {
            if values.is_empty() {
                return acc;
            }
            acc.into_iter()
                .flat_map(|base| {
                    values.iter().map(move |value| {
                        let mut next = base.clone();
                        next.insert(*key, *value);
                        next
                    })
                })
                .collect::<Vec<_>>()
        });

    if schedules.is_empty() {
        bail!("cron expression produced no launchd calendar intervals");
    }
    Ok(if schedules.len() == 1 {
        render_calendar_dict(&schedules[0])
    } else {
        let body = schedules
            .iter()
            .map(render_calendar_dict)
            .collect::<String>();
        format!("<array>{body}</array>")
    })
}

fn expand_field(expr: &str, min: u32, max: u32, normalize_sunday: bool) -> Result<Vec<u32>> {
    if expr == "*" {
        return Ok(Vec::new());
    }
    let mut values = Vec::new();
    for segment in expr.split(',') {
        let (base, step) = match segment.split_once('/') {
            Some((base, step)) => {
                let step = parse_number(step, min, max)?;
                if step == 0 {
                    bail!("cron step value cannot be zero in '{segment}'");
                }
                (base, step)
            }
            None => (segment, 1),
        };
        let (start, end) = if base == "*" {
            (min, max)
        } else if let Some((start, end)) = base.split_once('-') {
            let start = parse_number(start, min, max)?;
            let end = parse_number(end, min, max)?;
            if start > end {
                bail!("invalid cron range '{segment}'");
            }
            (start, end)
        } else {
            let value = parse_number(base, min, max)?;
            (value, value)
        };
        let mut value = start;
        while value <= end {
            let normalized = if normalize_sunday && value == 7 {
                0
            } else {
                value
            };
            if !values.contains(&normalized) {
                values.push(normalized);
            }
            if end - value < step {
                break;
            }
            value += step;
        }
    }
    values.sort_unstable();
    Ok(values)
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

fn render_calendar_dict(fields: &BTreeMap<&str, u32>) -> String {
    let body = fields
        .iter()
        .map(|(key, value)| format!("<key>{key}</key><integer>{value}</integer>"))
        .collect::<String>();
    format!("<dict>{body}</dict>")
}
