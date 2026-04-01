use crate::config::Config;
use crate::decay::get_decay_weight_for_file_with_connection;
use crate::error::Result;
use rusqlite::Connection;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug)]
pub(crate) struct GraphData {
    pub(crate) concepts: Vec<String>,
    pub(crate) adjacency: Vec<Vec<(usize, f64)>>,
    pub(crate) total_edge_weight: f64,
}

// T-12.0: RTM FR-23.6
pub(crate) fn load_graph(connection: &Connection, config: &Config) -> Result<GraphData> {
    let mut concepts_stmt =
        connection.prepare("SELECT concept_name FROM concepts ORDER BY concept_name")?;
    let concepts = concepts_stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let index_by_name = concepts
        .iter()
        .enumerate()
        .map(|(index, name)| (name.clone(), index))
        .collect::<HashMap<_, _>>();

    let mut adjacency = vec![Vec::new(); concepts.len()];
    let mut edge_stmt = connection.prepare(
        "SELECT concept_a, concept_b, COALESCE(source_files, '') \
         FROM concept_graph \
         ORDER BY concept_a, concept_b",
    )?;
    let edge_rows = edge_stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    for edge_row in edge_rows {
        let (concept_a, concept_b, source_files) = edge_row?;
        let Some(&index_a) = index_by_name.get(&concept_a) else {
            continue;
        };
        let Some(&index_b) = index_by_name.get(&concept_b) else {
            continue;
        };

        let weight = graph_edge_weight(&source_files, connection, config);
        if weight <= 0.0 {
            continue;
        }
        adjacency[index_a].push((index_b, weight));
        adjacency[index_b].push((index_a, weight));
    }

    for neighbors in &mut adjacency {
        neighbors.sort_by_key(|(neighbor, _)| *neighbor);
    }

    let total_edge_weight = adjacency
        .iter()
        .flat_map(|neighbors| neighbors.iter().map(|(_, weight)| *weight))
        .sum::<f64>()
        / 2.0;

    Ok(GraphData {
        concepts,
        adjacency,
        total_edge_weight,
    })
}

// T-15.8: RTM FR-13 — edge weight = sum(proximity × decay) matching C# RunLouvain.
// Each source file's proximity is multiplied by its temporal decay weight before summing.
pub(crate) fn graph_edge_weight(
    source_files: &str,
    connection: &Connection,
    config: &Config,
) -> f64 {
    let Ok(weights) = parse_source_file_weights(source_files) else {
        return 0.0;
    };
    weights
        .iter()
        .map(|(uri, proximity)| {
            let decay = get_decay_weight_for_file_with_connection(uri, connection, config);
            proximity * decay
        })
        .sum()
}

// T-4.3: RTM FR-23.x
pub(crate) fn parse_source_file_weights(source_files: &str) -> Result<BTreeMap<String, f64>> {
    let trimmed = source_files.trim();
    if trimmed.is_empty() {
        return Ok(BTreeMap::new());
    }

    match serde_json::from_str::<Value>(trimmed)? {
        Value::Object(map) => Ok(map
            .into_iter()
            .filter_map(|(path, value)| value.as_f64().map(|weight| (path, weight)))
            .collect()),
        Value::Array(entries) => Ok(entries
            .into_iter()
            .filter_map(|value| value.as_str().map(|path| (path.to_string(), 1.0)))
            .collect()),
        _ => Ok(BTreeMap::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::graph_edge_weight;
    use crate::config::Config;
    use crate::db::GraphDatabase;
    use chrono::{Duration, Utc};
    use rusqlite::{Connection, params};
    use serial_test::serial;
    use tempfile::TempDir;

    struct TestDb {
        _tmp: TempDir,
        conn: Connection,
        cfg: Config,
    }

    impl TestDb {
        fn setup() -> Self {
            let tmp = TempDir::new().expect("tempdir");
            Config::override_root(tmp.path());
            Config::set_database_path(tmp.path().join("memory.db"));
            let cfg = Config::current();
            cfg.ensure_directories().expect("dirs");
            GraphDatabase::initialize_database(&cfg).expect("db init");
            let conn = Connection::open(cfg.database_path.as_path()).expect("open db");
            conn.execute_batch("PRAGMA busy_timeout=5000;")
                .expect("pragmas");
            Self {
                _tmp: tmp,
                conn,
                cfg,
            }
        }

        fn insert_file_content(&self, uri: &str, created: &str, last_accessed: Option<&str>) {
            self.conn
                .execute(
                    "INSERT OR REPLACE INTO file_content (file_path, title, content, created, last_accessed) \
                     VALUES (?1, ?2, '', ?3, ?4)",
                    params![uri, uri, created, last_accessed],
                )
                .expect("insert file_content");
        }
    }

    impl Drop for TestDb {
        fn drop(&mut self) {
            Config::reset_overrides();
        }
    }

    // T-14.7: RTM FR-13
    #[test]
    #[serial(config)]
    fn community_edge_weight_is_zero_when_source_files_are_empty() {
        let db = TestDb::setup();
        assert_eq!(graph_edge_weight("{}", &db.conn, &db.cfg), 0.0);
        assert_eq!(graph_edge_weight("", &db.conn, &db.cfg), 0.0);
    }

    // T-14.7: RTM FR-13
    // When source files have no DB row and no disk file, decay falls back to 1.0 (neutral).
    #[test]
    #[serial(config)]
    fn community_edge_weight_uses_source_file_weights_when_no_db_row() {
        let db = TestDb::setup();
        let source_files = r#"{"memory://a.md":1.25,"memory://b.md":0.75}"#;
        assert_eq!(graph_edge_weight(source_files, &db.conn, &db.cfg), 2.0);
    }

    // T-15.8: RTM FR-13 — recent source files yield edge weight near raw proximity sum.
    #[test]
    #[serial(config)]
    fn community_edge_weight_recent_files_near_raw_sum() {
        let db = TestDb::setup();
        let now = Utc::now().to_rfc3339();
        db.insert_file_content("memory://recent_a.md", &now, Some(&now));
        db.insert_file_content("memory://recent_b.md", &now, Some(&now));

        let source_files = r#"{"memory://recent_a.md":1.25,"memory://recent_b.md":0.75}"#;
        let weight = graph_edge_weight(source_files, &db.conn, &db.cfg);

        // Decay weight for recently accessed files should be ~1.0 (within grace period).
        assert!(
            (weight - 2.0).abs() < 0.01,
            "expected weight near 2.0 for recent files, got {weight}"
        );
    }

    // T-15.8: RTM FR-13 — old source files yield edge weight decayed below raw sum.
    #[test]
    #[serial(config)]
    fn community_edge_weight_old_files_decayed_below_raw_sum() {
        let db = TestDb::setup();
        let old = (Utc::now() - Duration::days(365)).to_rfc3339();
        db.insert_file_content("memory://old_a.md", &old, Some(&old));
        db.insert_file_content("memory://old_b.md", &old, Some(&old));

        let source_files = r#"{"memory://old_a.md":1.25,"memory://old_b.md":0.75}"#;
        let weight = graph_edge_weight(source_files, &db.conn, &db.cfg);

        // Decay weight for 365-day-old files should be well below 1.0.
        assert!(
            weight < 2.0,
            "expected decayed weight below raw sum 2.0, got {weight}"
        );
        assert!(
            weight > 0.0,
            "expected positive weight for decayed files, got {weight}"
        );
    }

    // T-15.8: RTM FR-13 — mix of old and new source files yields weighted average.
    #[test]
    #[serial(config)]
    fn community_edge_weight_mixed_ages_reflects_decay() {
        let db = TestDb::setup();
        let now = Utc::now().to_rfc3339();
        let old = (Utc::now() - Duration::days(365)).to_rfc3339();
        db.insert_file_content("memory://fresh.md", &now, Some(&now));
        db.insert_file_content("memory://stale.md", &old, Some(&old));

        let source_files = r#"{"memory://fresh.md":1.0,"memory://stale.md":1.0}"#;
        let weight = graph_edge_weight(source_files, &db.conn, &db.cfg);

        // Raw sum would be 2.0. With one fresh (decay ~1.0) and one stale (decay < 1.0),
        // the result should be between 1.0 and 2.0.
        assert!(
            weight < 2.0,
            "expected mixed weight below raw sum 2.0, got {weight}"
        );
        assert!(
            weight > 1.0,
            "expected mixed weight above 1.0 (fresh file contributes ~1.0), got {weight}"
        );
    }
}
