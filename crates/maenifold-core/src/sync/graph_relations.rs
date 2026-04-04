use super::file_index::load_file_concepts;
use crate::error::Result;
use crate::markdown::extract_wikilinks_with_sections;
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type RelationKey = (String, String);
pub(crate) type RelationSourceWeights = BTreeMap<String, f64>;
pub(crate) type RelationCounts = BTreeMap<RelationKey, (f64, RelationSourceWeights)>;

// T-13.6: RTM NFR-1 — loads per-file relation counts for incremental graph rebuild
pub(crate) fn load_relation_counts_for_files(
    connection: &Connection,
    source_files: &BTreeSet<String>,
    affected_concepts: Option<&BTreeSet<String>>,
) -> Result<RelationCounts> {
    let mut statement =
        connection.prepare("SELECT content FROM file_content WHERE file_path = ?1 LIMIT 1")?;
    let mut relation_counts = RelationCounts::new();

    for source_file in source_files {
        let content = statement
            .query_row(params![source_file], |row| row.get::<_, String>(0))
            .optional()?;
        let Some(content) = content else {
            continue;
        };
        let allowed_concepts = load_file_concepts(connection, source_file)?;
        if allowed_concepts.is_empty() {
            continue;
        }

        let mut per_file_counts = RelationCounts::new();
        flush_relation_counts(
            source_file,
            &content,
            Some(&allowed_concepts),
            &mut per_file_counts,
        );
        merge_relation_counts(&mut relation_counts, per_file_counts, affected_concepts);
    }

    Ok(relation_counts)
}

// T-13.6: RTM NFR-1 — merges per-file relation counts into aggregate totals
pub(crate) fn merge_relation_counts(
    target: &mut RelationCounts,
    source: RelationCounts,
    affected_concepts: Option<&BTreeSet<String>>,
) {
    for ((concept_a, concept_b), (_count, files)) in source {
        if let Some(affected_concepts) = affected_concepts
            && !affected_concepts.contains(&concept_a)
            && !affected_concepts.contains(&concept_b)
        {
            continue;
        }

        let entry = target
            .entry((concept_a, concept_b))
            .or_insert_with(|| (0.0, BTreeMap::new()));
        for (source_file, weight) in files {
            entry
                .1
                .entry(source_file)
                .and_modify(|existing| *existing = existing.max(weight))
                .or_insert(weight);
        }
        // T-14.4: RTM FR-11 — C# stores the number of distinct contributing files in
        // co_occurrence_count; proximity stays in source_files for downstream weighting.
        entry.0 = entry.1.len() as f64;
    }
}

// T-4.3: RTM FR-23.x
pub(crate) fn load_relation_counts(connection: &Connection) -> Result<RelationCounts> {
    let mut mentions_statement = connection.prepare(
        "SELECT source_file, concept_name FROM concept_mentions ORDER BY source_file, concept_name",
    )?;
    let mention_rows = mentions_statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut active_concepts_by_file = BTreeMap::<String, BTreeSet<String>>::new();
    for row in mention_rows {
        let (source_file, concept_name) = row?;
        active_concepts_by_file
            .entry(source_file)
            .or_default()
            .insert(concept_name);
    }

    let mut statement =
        connection.prepare("SELECT file_path, content FROM file_content ORDER BY file_path")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut relation_counts = RelationCounts::new();
    for row in rows {
        let (source_file, content) = row?;
        let Some(allowed_concepts) = active_concepts_by_file.get(&source_file) else {
            continue;
        };
        flush_relation_counts(
            &source_file,
            &content,
            Some(allowed_concepts),
            &mut relation_counts,
        );
    }

    Ok(relation_counts)
}

// T-4.3: RTM FR-23.x
pub(crate) fn flush_relation_counts(
    source_file: &str,
    content: &str,
    allowed_concepts: Option<&BTreeSet<String>>,
    relation_counts: &mut RelationCounts,
) {
    let mut concepts_by_section = BTreeMap::<String, BTreeSet<usize>>::new();
    for (concept, section_index) in extract_wikilinks_with_sections(content) {
        if allowed_concepts.is_some_and(|allowed| !allowed.contains(&concept)) {
            continue;
        }
        concepts_by_section
            .entry(concept)
            .or_default()
            .insert(section_index);
    }

    let concepts = concepts_by_section.into_iter().collect::<Vec<_>>();
    for index in 0..concepts.len() {
        for other_index in index + 1..concepts.len() {
            let (first, first_sections) = &concepts[index];
            let (second, second_sections) = &concepts[other_index];
            let key = if first <= second {
                (first.clone(), second.clone())
            } else {
                (second.clone(), first.clone())
            };
            let same_named_section = first_sections
                .iter()
                .copied()
                .find(|section_index| second_sections.contains(section_index));
            // T-4.3: RTM FR-23.2 - flat docs keep section_index=0 for extraction, but sync must
            // treat that implicit root as cross-section so headingless documents store half-weight pairs.
            let weight = if same_named_section.is_some_and(|section_index| section_index > 0) {
                1.0
            } else {
                0.5
            };
            let entry = relation_counts
                .entry(key)
                .or_insert_with(|| (0.0, BTreeMap::new()));
            entry
                .1
                .entry(source_file.to_string())
                .and_modify(|existing| *existing = existing.max(weight))
                .or_insert(weight);
            // T-14.4: RTM FR-11 — count each source file once even when the same concepts
            // appear multiple times; keep proximity weights only in source_files JSON.
            entry.0 = entry.1.len() as f64;
        }
    }
}
