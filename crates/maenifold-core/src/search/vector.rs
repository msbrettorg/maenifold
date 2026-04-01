use crate::db;
use crate::error::{MaenifoldError, Result};
use rusqlite::{Connection, params_from_iter};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(crate) type FileMetadata = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

pub(crate) struct VectorMetadataHydration {
    pub(crate) metadata: BTreeMap<String, FileMetadata>,
    pub(crate) orphan_paths: BTreeSet<String>,
}

#[allow(clippy::too_many_lines)]
pub(crate) fn query_vec_memory_files(
    connection: &Connection,
    assets_path: &Path,
    query: &str,
    folder_prefix: Option<&str>,
    max_candidates: usize,
) -> Result<Option<Vec<(String, f64)>>> {
    // T-9.2 / T-3.1: RTM FR-8, FR-6 — use shared load_vec0_extension helper
    // instead of inline unsafe block, eliminating duplication.
    if db::load_vec0_extension(connection, assets_path).is_err() {
        return Ok(None);
    }

    let query_embedding = match crate::vector::generate_embedding(query) {
        Ok(embedding) if crate::repair::embedding_is_valid_for_similarity_search(&embedding) => {
            embedding
        }
        Ok(_) => return Ok(Some(Vec::new())),
        Err(_) => return Ok(None),
    };
    let query_blob = crate::vector::to_sqlite_vector_blob(&query_embedding);

    let mut sql = String::from(
        "SELECT file_path, vec_distance_cosine(embedding, ?1) as distance \
         FROM vec_memory_files \
         WHERE embedding IS NOT NULL",
    );
    if folder_prefix.is_some() {
        sql.push_str(" AND (file_path = ?2 OR file_path LIKE ?2 || '/%')");
    }
    sql.push_str(" ORDER BY distance LIMIT ?");
    sql.push_str(if folder_prefix.is_some() { "3" } else { "2" });

    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(query_blob)];
    if let Some(prefix) = folder_prefix {
        params_vec.push(Box::new(prefix.to_string()));
    }
    params_vec.push(Box::new(max_candidates as i64));

    let mut statement = match connection.prepare(&sql) {
        Ok(statement) => statement,
        Err(error) => {
            let error = MaenifoldError::from(error);
            if crate::repair::is_vec_embedding_unavailable(&error) {
                return Ok(None);
            }
            return Err(error);
        }
    };

    let mut rows = match statement.query(params_from_iter(
        params_vec.iter().map(|param| param.as_ref()),
    )) {
        Ok(rows) => rows,
        Err(error) => {
            let error = MaenifoldError::from(error);
            if crate::repair::is_vec_embedding_unavailable(&error) {
                return Ok(None);
            }
            return Err(error);
        }
    };

    let mut db_results: Vec<(String, f64)> = Vec::new();
    while let Some(row) = rows.next()? {
        let path: String = row.get(0)?;
        // PARITY-S01: RTM FR-21.3 (RTM-109/RTM-112) — vec_distance_cosine() returns NULL
        // for malformed or zero embeddings; skip instead of crashing.
        let distance: Option<f64> = row.get(1)?;
        let distance = match distance {
            Some(d) => d,
            None => continue,
        };
        if distance.is_finite() {
            db_results.push((path, distance));
        }
    }

    if db_results.is_empty() {
        return Ok(Some(Vec::new()));
    }

    let min_distance = db_results
        .iter()
        .map(|(_, distance)| *distance)
        .fold(f64::INFINITY, f64::min);
    let max_distance = db_results
        .iter()
        .map(|(_, distance)| *distance)
        .fold(f64::NEG_INFINITY, f64::max);

    let normalized = db_results
        .into_iter()
        .map(|(path, distance)| {
            let score = if (max_distance - min_distance).abs() < 1e-12 {
                1.0
            } else {
                1.0 - ((distance - min_distance) / (max_distance - min_distance))
            };
            (path, score)
        })
        .collect();

    Ok(Some(normalized))
}

pub(crate) fn hydrate_vector_file_metadata(
    connection: &Connection,
    candidate_paths: &BTreeSet<String>,
) -> Result<VectorMetadataHydration> {
    if candidate_paths.is_empty() {
        return Ok(VectorMetadataHydration {
            metadata: BTreeMap::new(),
            orphan_paths: BTreeSet::new(),
        });
    }

    let path_values = candidate_paths.iter().cloned().collect::<Vec<_>>();
    let sql = format!(
        "SELECT file_path, COALESCE(title, ''), COALESCE(content, ''), created, last_accessed, status \
         FROM file_content WHERE file_path IN ({})",
        sql_placeholders(path_values.len())
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(path_values.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;

    let mut metadata = BTreeMap::new();
    for row in rows {
        let (path, title, content, created, last_accessed, status) = row?;
        metadata.insert(path, (title, content, created, last_accessed, status));
    }

    let orphan_paths = candidate_paths
        .iter()
        .filter(|path| !metadata.contains_key(*path))
        .cloned()
        .collect();

    Ok(VectorMetadataHydration {
        metadata,
        orphan_paths,
    })
}

pub(crate) fn cleanup_orphan_vec_memory_files(
    connection: &Connection,
    orphan_paths: &BTreeSet<String>,
) -> Result<()> {
    if orphan_paths.is_empty() {
        return Ok(());
    }

    let path_values = orphan_paths.iter().cloned().collect::<Vec<_>>();
    let sql = format!(
        "DELETE FROM vec_memory_files WHERE file_path IN ({})",
        sql_placeholders(path_values.len())
    );
    connection.execute(&sql, params_from_iter(path_values.iter()))?;
    Ok(())
}

fn sql_placeholders(count: usize) -> String {
    std::iter::repeat_n("?", count)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn path_set(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|path| (*path).to_string()).collect()
    }

    #[test]
    fn hydrate_vector_file_metadata_batches_hits_and_reports_orphans() {
        let connection = Connection::open_in_memory().expect("open in-memory db");
        connection
            .execute_batch(
                "CREATE TABLE file_content (
                    file_path TEXT PRIMARY KEY,
                    title TEXT,
                    content TEXT,
                    created TEXT,
                    last_accessed TEXT,
                    status TEXT
                );
                INSERT INTO file_content (file_path, title, content, created, last_accessed, status)
                VALUES ('memory://doc-1', 'Doc 1', 'alpha beta', '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z', 'active');",
            )
            .expect("seed file_content");

        let hydration = hydrate_vector_file_metadata(
            &connection,
            &path_set(&["memory://doc-1", "memory://missing"]),
        )
        .expect("hydrate metadata");

        assert_eq!(hydration.metadata.len(), 1);
        assert!(hydration.metadata.contains_key("memory://doc-1"));
        assert_eq!(hydration.orphan_paths, path_set(&["memory://missing"]));
    }

    #[test]
    fn cleanup_orphan_vec_memory_files_deletes_only_requested_rows() {
        let connection = Connection::open_in_memory().expect("open in-memory db");
        connection
            .execute_batch(
                "CREATE TABLE vec_memory_files (file_path TEXT PRIMARY KEY);
                INSERT INTO vec_memory_files (file_path)
                VALUES ('memory://keep'), ('memory://orphan');",
            )
            .expect("seed vec_memory_files");

        cleanup_orphan_vec_memory_files(&connection, &path_set(&["memory://orphan"]))
            .expect("cleanup orphan rows");

        let keep_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM vec_memory_files WHERE file_path = 'memory://keep'",
                [],
                |row| row.get(0),
            )
            .expect("count keep row");
        let orphan_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM vec_memory_files WHERE file_path = 'memory://orphan'",
                [],
                |row| row.get(0),
            )
            .expect("count orphan row");

        assert_eq!(keep_count, 1);
        assert_eq!(orphan_count, 0);
    }
}
