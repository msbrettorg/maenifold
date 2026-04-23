#[cfg(test)]
mod tests {
    use super::super::rank_hybrid_results;
    use crate::search::shared::TextResult;
    use rusqlite::{Connection, params};

    fn setup_connection(paths: &[&str]) -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory sqlite");
        connection
            .execute(
                "CREATE TABLE file_content (
                    file_path TEXT PRIMARY KEY,
                    title TEXT,
                    content TEXT,
                    created TEXT,
                    last_accessed TEXT,
                    status TEXT
                )",
                [],
            )
            .expect("create file_content");
        for path in paths {
            connection
                .execute(
                    "INSERT INTO file_content (file_path, title, content, created, last_accessed, status)
                     VALUES (?1, ?2, ?3, NULL, NULL, NULL)",
                    params![path, format!("Title for {path}"), format!("Content for {path}")],
                )
                .expect("insert file_content row");
        }
        connection
    }

    fn text_result(path: &str, score: f64) -> TextResult {
        (
            path.to_string(),
            score,
            format!("Title for {path}"),
            format!("Content for {path}"),
            None,
            None,
            None,
        )
    }

    #[test]
    fn rank_hybrid_results_preserves_default_behavior() {
        let connection = setup_connection(&["memory://alpha", "memory://beta"]);
        let text_results = vec![
            text_result("memory://alpha", 80.0),
            text_result("memory://beta", 50.0),
        ];
        let mut vector_results = Some(vec![
            ("memory://alpha".to_string(), 0.9),
            ("memory://beta".to_string(), 0.4),
        ]);

        let (rrf_scores, original_scores, file_metadata) = rank_hybrid_results(
            &connection,
            &mut vector_results,
            &text_results,
            60.0,
            1.0,
            1.0,
            0.0,
        )
        .expect("rank hybrid results");

        assert_eq!(vector_results.as_ref().map(Vec::len), Some(2));
        assert!((rrf_scores["memory://alpha"] - (2.0 / 61.0)).abs() < 1e-12);
        assert!((rrf_scores["memory://beta"] - (2.0 / 62.0)).abs() < 1e-12);
        assert_eq!(original_scores["memory://alpha"], (0.8, 0.9));
        assert_eq!(original_scores["memory://beta"], (0.5, 0.4));
        assert!(file_metadata.contains_key("memory://alpha"));
        assert!(file_metadata.contains_key("memory://beta"));
    }

    #[test]
    fn rank_hybrid_results_applies_weights_and_semantic_min_score() {
        let connection = setup_connection(&["memory://alpha", "memory://beta"]);
        let text_results = vec![
            text_result("memory://alpha", 80.0),
            text_result("memory://beta", 50.0),
        ];
        let mut vector_results = Some(vec![
            ("memory://beta".to_string(), 0.2),
            ("memory://alpha".to_string(), 0.8),
        ]);

        let (rrf_scores, original_scores, _) = rank_hybrid_results(
            &connection,
            &mut vector_results,
            &text_results,
            10.0,
            2.0,
            3.0,
            0.5,
        )
        .expect("rank hybrid results");

        assert_eq!(
            vector_results,
            Some(vec![("memory://alpha".to_string(), 0.8)])
        );
        assert!((rrf_scores["memory://alpha"] - (5.0 / 11.0)).abs() < 1e-12);
        assert!((rrf_scores["memory://beta"] - (2.0 / 12.0)).abs() < 1e-12);
        assert_eq!(original_scores["memory://alpha"], (0.8, 0.8));
        assert_eq!(original_scores["memory://beta"], (0.5, 0.0));
    }
}
