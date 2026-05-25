use crate::error::{MaenifoldError, Result};
use rusqlite::{Connection, params};
use serde::Serialize;

pub(super) const CORPUS_PRIMER_URI: &str = "graph://corpus/primer";
const DEFAULT_LIMIT: usize = 20;
const MAX_MEMBERS_PER_COMMUNITY: usize = 10;
const MAX_PAYLOAD_BYTES: usize = 8 * 1024;

#[derive(Serialize)]
struct CorpusPrimerPayload {
    lines: Vec<String>,
}

struct PrimerCommunityRow {
    community_id: i64,
    concept_name: String,
    rank: i64,
}

struct PrimerCommunity {
    community_id: i64,
    anchor: String,
    members: Vec<String>,
}

pub(super) fn parse_limit(uri: &str) -> Result<Option<usize>> {
    let Some(query) = uri.strip_prefix(CORPUS_PRIMER_URI) else {
        return Ok(None);
    };

    if query.is_empty() {
        return Ok(Some(DEFAULT_LIMIT));
    }

    let query = query.strip_prefix('?').ok_or_else(|| {
        MaenifoldError::InvalidResourceUri(format!(
            "{uri}. Expected '{CORPUS_PRIMER_URI}' or '{CORPUS_PRIMER_URI}?limit=<n>'"
        ))
    })?;

    let mut limit = DEFAULT_LIMIT;
    for pair in query.split('&').filter(|segment| !segment.is_empty()) {
        let (key, value) = pair.split_once('=').ok_or_else(|| {
            MaenifoldError::InvalidResourceUri(format!("{uri}. Invalid query parameter '{pair}'"))
        })?;
        if key != "limit" {
            return Err(MaenifoldError::InvalidResourceUri(format!(
                "{uri}. Unsupported query parameter '{key}'"
            )));
        }
        limit = value.parse::<usize>().map_err(|error| {
            MaenifoldError::InvalidResourceUri(format!("{uri}. Invalid limit '{value}': {error}"))
        })?;
    }

    Ok(Some(limit))
}

pub(super) fn render_corpus_primer(connection: &Connection, limit: usize) -> Result<String> {
    let mut lines = Vec::new();

    for community in load_primer_communities(connection)?.into_iter().take(limit) {
        let line = format_line(&community);
        let mut next_lines = lines.clone();
        next_lines.push(line.clone());
        let payload = serde_json::to_string(&CorpusPrimerPayload { lines: next_lines })?;
        if payload.as_bytes().len() > MAX_PAYLOAD_BYTES {
            break;
        }
        lines.push(line);
    }

    serde_json::to_string(&CorpusPrimerPayload { lines }).map_err(Into::into)
}

fn load_primer_communities(connection: &Connection) -> Result<Vec<PrimerCommunity>> {
    let mut statement = connection.prepare(
        "WITH concept_decay AS (
             SELECT cm.concept_name,
                    MAX(
                        CASE
                            WHEN julianday('now') - julianday(COALESCE(fc.last_accessed, fc.created, fc.last_indexed)) <=
                                CASE
                                    WHEN fc.file_path LIKE '%thinking/sequential/%' THEN 7
                                    WHEN fc.file_path LIKE '%thinking/compaction/%' THEN 7
                                    WHEN fc.file_path LIKE '%thinking/workflow/%' THEN 14
                                    ELSE 28
                                END
                            THEN 1.0
                            ELSE MIN(1.0, 1.0 / sqrt(MAX(1,
                                julianday('now') - julianday(COALESCE(fc.last_accessed, fc.created, fc.last_indexed)) -
                                CASE
                                    WHEN fc.file_path LIKE '%thinking/sequential/%' THEN 7
                                    WHEN fc.file_path LIKE '%thinking/compaction/%' THEN 7
                                    WHEN fc.file_path LIKE '%thinking/workflow/%' THEN 14
                                    ELSE 28
                                END
                            )))
                        END
                    ) AS max_decay
             FROM concept_mentions cm
             JOIN file_content fc ON cm.source_file = fc.file_path
             WHERE COALESCE(fc.last_accessed, fc.created, fc.last_indexed) IS NOT NULL
             GROUP BY cm.concept_name
         ),
         community_sizes AS (
             SELECT community_id, COUNT(*) AS size
             FROM concept_communities
             GROUP BY community_id
             HAVING size >= 3
         ),
         concept_degree AS (
             SELECT cc.concept_name,
                    cc.community_id,
                    SUM(cg.co_occurrence_count) AS raw_weight,
                    COALESCE(cd.max_decay, 0.1) AS max_decay
             FROM concept_communities cc
             JOIN community_sizes cs ON cc.community_id = cs.community_id
             LEFT JOIN concept_graph cg
                 ON cc.concept_name = cg.concept_a OR cc.concept_name = cg.concept_b
             LEFT JOIN concept_decay cd ON cc.concept_name = cd.concept_name
             WHERE cc.concept_name NOT GLOB '*[.]*'
               AND length(cc.concept_name) >= 3
               AND cc.concept_name NOT GLOB '[0-9]*'
             GROUP BY cc.concept_name, cc.community_id
         ),
         ranked AS (
             SELECT concept_name,
                    community_id,
                    raw_weight * max_decay AS weighted_score,
                    ROW_NUMBER() OVER (
                        PARTITION BY community_id
                        ORDER BY raw_weight * max_decay DESC, concept_name ASC
                    ) AS rn
             FROM concept_degree
             WHERE raw_weight > 0
         ),
         anchors AS (
             SELECT community_id,
                    weighted_score AS anchor_score
             FROM ranked
             WHERE rn = 1
         )
         SELECT r.community_id,
                r.concept_name,
                r.rn
         FROM ranked r
         JOIN anchors a ON a.community_id = r.community_id
         WHERE r.rn <= ?1
         ORDER BY a.anchor_score DESC, r.community_id ASC, r.rn ASC, r.concept_name ASC",
    )?;

    let rows = statement.query_map(params![(MAX_MEMBERS_PER_COMMUNITY + 1) as i64], |row| {
        Ok(PrimerCommunityRow {
            community_id: row.get(0)?,
            concept_name: row.get(1)?,
            rank: row.get(2)?,
        })
    })?;

    let mut communities: Vec<PrimerCommunity> = Vec::new();
    for row in rows.collect::<std::result::Result<Vec<_>, _>>()? {
        match communities.last_mut() {
            Some(community) if community.community_id == row.community_id => {
                if row.rank > 1 {
                    community.members.push(row.concept_name);
                }
            }
            _ => communities.push(PrimerCommunity {
                community_id: row.community_id,
                anchor: row.concept_name,
                members: Vec::new(),
            }),
        }
    }

    Ok(communities)
}

fn format_line(community: &PrimerCommunity) -> String {
    let mut line = format!("**[[{}]]**", community.anchor);
    for member in &community.members {
        line.push(' ');
        line.push_str(&format!("[[{member}]]"));
    }
    line
}
