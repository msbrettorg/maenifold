use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

use crate::{ParseTaskError, StageVerdict};

/// A raw H2 section extracted from a markdown task file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSection {
    pub heading: String,
    pub heading_line: usize,
    pub verdict: Option<StageVerdict>,
}

#[derive(Debug, Clone)]
pub(super) struct RawSection {
    pub(super) heading: String,
    pub(super) heading_line: usize,
    pub(super) verdicts: Vec<RawVerdict>,
}

#[derive(Debug, Clone)]
pub(super) struct RawVerdict {
    pub(super) line: usize,
    pub(super) value: String,
    pub(super) parsed: Option<StageVerdict>,
}

#[derive(Debug)]
struct HeadingCapture {
    line: usize,
    text: String,
}

#[derive(Debug)]
enum VerdictState {
    Idle,
    Strong { line: usize, text: String },
    AfterStrong { line: usize, text: String },
}

/// Extract all H2 sections and their verdicts from markdown content.
pub fn extract_sections(content: &str) -> Result<Vec<ParsedSection>, ParseTaskError> {
    Ok(scan_sections(content)?
        .into_iter()
        .map(|section| ParsedSection {
            heading: section.heading,
            heading_line: section.heading_line,
            verdict: section.verdicts.first().and_then(|verdict| verdict.parsed),
        })
        .collect())
}

pub(super) fn scan_sections(content: &str) -> Result<Vec<RawSection>, ParseTaskError> {
    let line_index = LineIndex::new(content);
    let mut sections = Vec::<RawSection>::new();
    let mut heading = None::<HeadingCapture>;
    let mut current_section = None::<usize>;
    let mut verdict_state = VerdictState::Idle;

    for (event, range) in Parser::new(content).into_offset_iter() {
        if let Some(index) = current_section {
            update_verdict_state(
                &mut verdict_state,
                &event,
                line_index.line_number(range.start),
                &mut sections[index],
            );
        }

        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H2,
                ..
            }) => {
                current_section = None;
                verdict_state = VerdictState::Idle;
                heading = Some(HeadingCapture {
                    line: line_index.line_number(range.start),
                    text: String::new(),
                });
            }
            Event::End(TagEnd::Heading(HeadingLevel::H2)) => {
                if let Some(heading) = heading.take() {
                    sections.push(RawSection {
                        heading: heading.text.trim().to_string(),
                        heading_line: heading.line,
                        verdicts: Vec::new(),
                    });
                    current_section = Some(sections.len() - 1);
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(heading) = heading.as_mut() {
                    heading.text.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(heading) = heading.as_mut() {
                    heading.text.push(' ');
                }
            }
            _ => {}
        }
    }

    if let Some(index) = current_section {
        finalize_verdict_state(&mut verdict_state, &mut sections[index]);
    }

    Ok(sections)
}

fn update_verdict_state(
    state: &mut VerdictState,
    event: &Event<'_>,
    line: usize,
    section: &mut RawSection,
) {
    match state {
        VerdictState::Idle => {
            if matches!(event, Event::Start(Tag::Strong)) {
                *state = VerdictState::Strong {
                    line,
                    text: String::new(),
                };
            }
        }
        VerdictState::Strong { line, text } => match event {
            Event::Text(value) | Event::Code(value) => text.push_str(value),
            Event::SoftBreak | Event::HardBreak => text.push(' '),
            Event::End(TagEnd::Strong) => {
                if text.trim() == "Verdict" {
                    *state = VerdictState::AfterStrong {
                        line: *line,
                        text: String::new(),
                    };
                } else {
                    *state = VerdictState::Idle;
                }
            }
            _ => {}
        },
        VerdictState::AfterStrong { text, .. } => match event {
            Event::Text(value) | Event::Code(value) => text.push_str(value),
            Event::SoftBreak | Event::HardBreak => text.push(' '),
            Event::End(TagEnd::Paragraph | TagEnd::Item) => finalize_verdict_state(state, section),
            Event::Start(Tag::Heading { .. } | Tag::Paragraph | Tag::Item) => {
                finalize_verdict_state(state, section)
            }
            _ => {}
        },
    }
}

fn finalize_verdict_state(state: &mut VerdictState, section: &mut RawSection) {
    if let VerdictState::AfterStrong { line, text } = state {
        if let Some((_, raw)) = text.split_once(':') {
            let value = raw.trim().to_string();
            if !value.is_empty() {
                section.verdicts.push(RawVerdict {
                    line: *line,
                    parsed: parse_generic_verdict(&value),
                    value,
                });
            }
        }
    }
    *state = VerdictState::Idle;
}

fn parse_generic_verdict(raw: &str) -> Option<StageVerdict> {
    match raw
        .split(|ch: char| !ch.is_alphanumeric())
        .find(|token| matches!(*token, "PASS" | "REJECT" | "FAIL"))
    {
        Some("PASS") => Some(StageVerdict::Pass),
        Some("REJECT") => Some(StageVerdict::Reject),
        Some("FAIL") => Some(StageVerdict::Fail),
        _ => None,
    }
}

#[derive(Debug)]
struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    fn new(content: &str) -> Self {
        let mut starts = vec![0];
        for (index, byte) in content.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(index + 1);
            }
        }
        Self { starts }
    }

    fn line_number(&self, offset: usize) -> usize {
        self.starts.partition_point(|start| *start <= offset)
    }
}
