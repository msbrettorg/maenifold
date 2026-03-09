use maenifold_core::{Config, GraphDatabase};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct TestEnv {
    _tmp: TempDir,
    cfg: Config,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        Config::reset_overrides();
    }
}

fn isolated_env() -> TestEnv {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    GraphDatabase::initialize_database(&cfg).expect("db init");
    TestEnv { _tmp: tmp, cfg }
}

// T-0.1: RTM NFR-3 — Port of C# T-QUAL-FSC2.4 / FR-7.4 hard-measurement diagnostics.
#[test]
#[ignore = "ONNX-dependent"]
fn capture_embedding_plateau_diagnostics() {
    let _guard = TEST_MUTEX.lock();
    let env = isolated_env();

    let Some(vec0_path) = vec0_extension_path(&packaged_assets_root()) else {
        eprintln!("Test skipped: sqlite-vec extension asset unavailable");
        return;
    };

    if onnx_model_path(&packaged_assets_root()).is_none() {
        eprintln!("Test skipped: ONNX model asset unavailable");
        return;
    }

    let queries = [
        "mcp",
        "oauth2",
        "finops",
        "graphrag",
        "xqz",
        "zzzzzzzz",
        "graph-rag",
    ];

    let connection = Connection::open(&env.cfg.database_path).expect("open db");

    if let Err(error) = load_vec_extension(&connection, &vec0_path) {
        eprintln!("Test skipped: sqlite-vec extension unavailable ({error})");
        return;
    }

    if let Err(error) = connection.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS vec_concepts USING vec0(concept_name TEXT PRIMARY KEY, embedding FLOAT[384]);",
    ) {
        eprintln!("Test skipped: sqlite-vec tables unavailable ({error})");
        return;
    }

    let mut concept_names = Vec::new();
    let mut hash_groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut inserted = 0usize;

    for query in queries {
        let embedding = match generate_embedding(query) {
            Ok(embedding) => embedding,
            Err(reason) => {
                eprintln!("Test skipped: {reason}");
                return;
            }
        };
        let blob = to_sqlite_vector_blob(&embedding);
        let hash = compute_sha256_hex(&blob);

        hash_groups.entry(hash).or_default().push(query.to_string());
        concept_names.push(query.to_string());

        connection
            .execute("DELETE FROM vec_concepts WHERE concept_name = ?1", [query])
            .expect("delete existing vec row");

        inserted += connection
            .execute(
                "INSERT INTO vec_concepts (concept_name, embedding) VALUES (?1, ?2)",
                params![query, blob],
            )
            .expect("insert vec row");
    }

    println!("Embedding SHA256 groups with identical hashes:");
    let duplicate_groups: Vec<_> = hash_groups
        .iter()
        .filter(|(_, values)| values.len() > 1)
        .collect();
    if duplicate_groups.is_empty() {
        println!("  (none)");
    } else {
        for (hash, values) in duplicate_groups {
            println!("  {hash}: {}", values.join(", "));
        }
    }

    let probe_embedding = match generate_embedding("mcp") {
        Ok(embedding) => embedding,
        Err(reason) => {
            eprintln!("Test skipped: {reason}");
            return;
        }
    };
    let probe_blob = to_sqlite_vector_blob(&probe_embedding);

    let count_1e6 = count_distances_at_or_below(&connection, &probe_blob, 1e-6);
    let count_1e12 = count_distances_at_or_below(&connection, &probe_blob, 1e-12);

    println!("Distances <= 1e-6 (probe=mcp): {count_1e6}");
    println!("Distances <= 1e-12 (probe=mcp): {count_1e12}");

    println!("Top distances (probe=mcp):");
    for (concept_name, distance) in top_distances(&connection, &probe_blob, 20) {
        println!("  {concept_name} => {distance:.12}");
    }

    assert_eq!(
        inserted,
        queries.len(),
        "Expected all embeddings to be inserted."
    );
    assert!(
        count_1e6 >= 1,
        "Expected probe embedding to match at least itself."
    );
    assert_eq!(
        concept_names.len(),
        queries.len(),
        "expected concept_names to track every inserted query"
    );
}

// T-0.1: RTM NFR-3 — Port of C# T-QUAL-FSC2.4 / FR-7.4 distance threshold helper.
fn count_distances_at_or_below(connection: &Connection, probe_blob: &[u8], threshold: f64) -> i64 {
    connection
        .query_row(
            "SELECT COUNT(*) FROM vec_concepts WHERE vec_distance_cosine(embedding, ?1) <= ?2",
            params![probe_blob, threshold],
            |row| row.get(0),
        )
        .expect("count distances")
}

// T-0.1: RTM NFR-3 — Port of C# T-QUAL-FSC2.4 / FR-7.4 top-distance helper.
fn top_distances(connection: &Connection, probe_blob: &[u8], limit: i64) -> Vec<(String, f64)> {
    let mut statement = connection
        .prepare(
            "SELECT concept_name, vec_distance_cosine(embedding, ?1) AS distance
             FROM vec_concepts
             ORDER BY distance ASC
             LIMIT ?2",
        )
        .expect("prepare top distances");

    let rows = statement
        .query_map(params![probe_blob, limit], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
        })
        .expect("query top distances");

    rows.collect::<Result<Vec<_>, _>>()
        .expect("collect top distances")
}

// T-0.1: RTM NFR-3 — Port of C# T-QUAL-FSC2.4 / FR-7.4 SHA256 grouping helper.
fn compute_sha256_hex(data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    let mut encoded = String::with_capacity(hash.len() * 2);
    for value in hash {
        write!(&mut encoded, "{value:02x}").expect("write sha256 hex");
    }
    encoded
}

fn to_sqlite_vector_blob(embedding: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(std::mem::size_of_val(embedding));
    for value in embedding {
        blob.extend_from_slice(&value.to_le_bytes());
    }
    blob
}

fn generate_embedding(_text: &str) -> Result<Vec<f32>, String> {
    Err(
        "ONNX embedding generation is not available in maenifold-rs yet; keep this port #[ignore] until the Rust embedding pipeline lands."
            .to_string(),
    )
}

fn load_vec_extension(connection: &Connection, extension_path: &Path) -> rusqlite::Result<()> {
    unsafe {
        connection.load_extension_enable()?;
        let load_result = connection.load_extension(extension_path, None::<&str>);
        let disable_result = connection.load_extension_disable();
        load_result?;
        disable_result?;
    }
    Ok(())
}

fn packaged_assets_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets"))
}

fn onnx_model_path(assets_root: &Path) -> Option<PathBuf> {
    let path = assets_root.join("models").join("all-MiniLM-L6-v2.onnx");
    path.exists().then_some(path)
}

fn vec0_extension_path(assets_root: &Path) -> Option<PathBuf> {
    let (platform_dir, filename) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => ("osx-arm64", "vec0.dylib"),
        ("macos", "x86_64") => ("osx-x64", "vec0.dylib"),
        ("linux", "x86_64") => ("linux-x64", "vec0.so"),
        ("linux", "aarch64") => ("linux-arm64", "vec0.so"),
        ("windows", "x86_64") => ("win-x64", "vec0.dll"),
        _ => return None,
    };

    let path = assets_root.join("native").join(platform_dir).join(filename);
    path.exists().then_some(path)
}
