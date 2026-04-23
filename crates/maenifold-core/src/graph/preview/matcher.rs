// T-01.16: concept matching + section discovery extracted from graph/preview.rs.
// No behavioural changes.

use regex::Regex;

#[derive(Clone, Copy)]
pub(super) struct SectionRange {
    pub(super) start: usize,
    pub(super) end: usize,
}

#[derive(Clone, Copy)]
pub(super) struct MatchSpan {
    pub(super) start: usize,
    pub(super) end: usize,
}

pub(super) struct ConceptMatcher {
    wikilink_regexes: Vec<Regex>,
    plain_regex: Option<Regex>,
}

impl ConceptMatcher {
    pub(super) fn new(normalized_concept: &str) -> Self {
        let plain_concepts = [
            normalized_concept.replace('-', " "),
            normalized_concept.to_string(),
            normalized_concept.replace('-', "_"),
        ]
        .into_iter()
        .fold(Vec::<String>::new(), |mut variants, variant| {
            if !variant.is_empty() && !variants.contains(&variant) {
                variants.push(variant);
            }
            variants
        });
        let plain_regex = if plain_concepts.is_empty() {
            None
        } else {
            let alternation = plain_concepts
                .iter()
                .map(|variant| regex::escape(variant))
                .collect::<Vec<_>>()
                .join("|");
            let pattern = format!(
                r"(?iu)(?:^|[^\p{{L}}\p{{N}}_-])({})(?:$|[^\p{{L}}\p{{N}}_-])",
                alternation
            );
            Regex::new(&pattern).ok()
        };

        let mut wikilink_variants = Vec::new();
        for variant in [
            normalized_concept.to_string(),
            normalized_concept.replace('-', "_"),
            normalized_concept.replace('-', " "),
        ] {
            let wikilink = format!("[[{variant}]]");
            if !wikilink_variants.contains(&wikilink) {
                wikilink_variants.push(wikilink);
            }
        }

        let wikilink_regexes = wikilink_variants
            .iter()
            .filter_map(|wikilink| {
                let pattern = format!(r"(?i){}", regex::escape(wikilink));
                Regex::new(&pattern).ok()
            })
            .collect();

        Self {
            wikilink_regexes,
            plain_regex,
        }
    }

    fn find_first(&self, text: &str) -> Option<MatchSpan> {
        let wikilink_match = self
            .wikilink_regexes
            .iter()
            .filter_map(|regex| {
                regex.find(text).map(|matched| MatchSpan {
                    start: matched.start(),
                    end: matched.end(),
                })
            })
            .min_by_key(|found| found.start);

        let plain_match = self.plain_regex.as_ref().and_then(|regex| {
            regex.captures(text).and_then(|captures| {
                captures.get(1).map(|matched| MatchSpan {
                    start: matched.start(),
                    end: matched.end(),
                })
            })
        });

        match (wikilink_match, plain_match) {
            (Some(left), Some(right)) => Some(if left.start <= right.start {
                left
            } else {
                right
            }),
            (Some(found), None) | (None, Some(found)) => Some(found),
            (None, None) => None,
        }
    }
}

pub(super) fn collect_sections(body: &str) -> Vec<SectionRange> {
    let mut sections = Vec::new();
    let mut current_start = 0;
    let mut current_has_content = false;
    let mut offset = 0;

    for line in body.split_inclusive('\n') {
        let line_content = line.strip_suffix('\n').unwrap_or(line);
        if line_content.starts_with("## ") && current_has_content {
            if !body[current_start..offset].trim().is_empty() {
                sections.push(SectionRange {
                    start: current_start,
                    end: offset,
                });
            }
            current_start = offset;
        }

        if !line_content.trim().is_empty() {
            current_has_content = true;
        }

        offset += line.len();
    }

    if !body[current_start..].trim().is_empty() {
        sections.push(SectionRange {
            start: current_start,
            end: body.len(),
        });
    }

    sections
}

pub(super) fn find_concept_match(
    text: &str,
    matcher: &ConceptMatcher,
    allow_heading_matches: bool,
) -> Option<MatchSpan> {
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let line_content = line.strip_suffix('\n').unwrap_or(line);
        let is_heading = line_content.trim_start().starts_with('#');
        if (allow_heading_matches || !is_heading)
            && let Some(concept_match) = matcher.find_first(line_content)
        {
            return Some(MatchSpan {
                start: offset + concept_match.start,
                end: offset + concept_match.end,
            });
        }

        offset += line.len();
    }

    None
}
