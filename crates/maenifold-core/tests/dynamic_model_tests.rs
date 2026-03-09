use maenifold_core::{Config, GraphDatabase, db, vector};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn isolated_root() -> (TempDir, Config) {
    let tmp = TempDir::new().expect("tmp");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let cfg = Config::current();
    cfg.ensure_directories().expect("dirs");
    (tmp, cfg)
}

fn packaged_models_dir() -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models");
    path.canonicalize().unwrap_or(path)
}

fn packaged_model_path() -> PathBuf {
    packaged_models_dir().join("all-MiniLM-L6-v2.onnx")
}

fn direct_assets_root() -> PathBuf {
    env::current_exe()
        .expect("current exe")
        .parent()
        .expect("test binary parent")
        .join("assets")
}

fn link_or_copy(src: &Path, dst: &Path) {
    fs::create_dir_all(dst.parent().expect("dst parent")).expect("create dst parent");
    match fs::hard_link(src, dst) {
        Ok(()) => {}
        Err(_) => {
            fs::copy(src, dst).expect("copy asset");
        }
    }
}

fn table_exists(conn: &Connection, table_name: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        [table_name],
        |row| row.get::<_, i64>(0),
    )
    .expect("query sqlite_master")
        == 1
}

struct TestContext {
    _tmp: TempDir,
    cfg: Config,
    shadow_assets_root: PathBuf,
    shadow_backup: Option<PathBuf>,
    hidden_packaged_model: Option<(PathBuf, PathBuf)>,
}

impl TestContext {
    fn new() -> Self {
        vector::cleanup();
        let (tmp, cfg) = isolated_root();

        let shadow_assets_root = direct_assets_root();
        let shadow_backup = if shadow_assets_root.exists() {
            let backup = shadow_assets_root.with_file_name(format!(
                "{}.dynamic-model-tests-backup",
                shadow_assets_root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("assets")
            ));
            if backup.exists() {
                fs::remove_dir_all(&backup).expect("remove stale assets backup");
            }
            fs::rename(&shadow_assets_root, &backup).expect("backup direct assets");
            Some(backup)
        } else {
            None
        };

        fs::create_dir_all(shadow_assets_root.join("models")).expect("create shadow models dir");

        Self {
            _tmp: tmp,
            cfg,
            shadow_assets_root,
            shadow_backup,
            hidden_packaged_model: None,
        }
    }

    fn config(&self) -> &Config {
        &self.cfg
    }

    fn shadow_models_dir(&self) -> PathBuf {
        self.shadow_assets_root.join("models")
    }

    fn install_valid_shadow_model_with_fallback_assets(&self, model_name: &str) {
        let models_dir = self.shadow_models_dir();
        link_or_copy(&packaged_model_path(), &models_dir.join(model_name));
        link_or_copy(
            &packaged_models_dir().join("tokenizer_config.json"),
            &models_dir.join("tokenizer_config.json"),
        );
        link_or_copy(
            &packaged_models_dir().join("vocab.txt"),
            &models_dir.join("vocab.txt"),
        );
    }

    fn write_dummy_onnx(&self, model_name: &str, bytes: usize) {
        fs::write(self.shadow_models_dir().join(model_name), vec![b'x'; bytes])
            .expect("write dummy model");
    }

    fn hide_packaged_model(&mut self) {
        let model_path = packaged_model_path();
        let hidden_path = model_path.with_file_name("all-MiniLM-L6-v2.onnx.dynamic-model-tests");
        if hidden_path.exists() {
            fs::remove_file(&hidden_path).expect("remove stale hidden model");
        }
        fs::rename(&model_path, &hidden_path).expect("hide packaged model");
        self.hidden_packaged_model = Some((model_path, hidden_path));
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        vector::cleanup();
        Config::reset_overrides();

        if let Some((model_path, hidden_path)) = self.hidden_packaged_model.take()
            && hidden_path.exists()
            && !model_path.exists()
        {
            fs::rename(hidden_path, model_path).expect("restore packaged model");
        }

        if self.shadow_assets_root.exists() {
            fs::remove_dir_all(&self.shadow_assets_root).expect("remove shadow assets");
        }

        if let Some(backup) = &self.shadow_backup
            && backup.exists()
        {
            fs::rename(backup, &self.shadow_assets_root).expect("restore direct assets");
        }
    }
}

// T-9.6: RTM FR-7 / RTM-050 / RTM-051 / RTM-053 — runtime model discovery should
// find a valid *.onnx file in assets/models and load fallback tokenizer assets from that directory.
#[test]
fn t9_6_model_discovery_finds_shadow_onnx_file() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();
    ctx.install_valid_shadow_model_with_fallback_assets("test-model.onnx");

    vector::load_model().expect("load discovered shadow model");

    assert_eq!(vector::model_filename().as_deref(), Some("test-model.onnx"));
    assert_eq!(vector::embedding_dimensions(), Some(384));

    let embedding = vector::generate_embedding("dynamic model loading").expect("embedding");
    assert_eq!(embedding.len(), 384);
}

// T-9.6: RTM FR-7 / RTM-055 — when no *.onnx file exists anywhere in assets/models,
// embedding APIs should degrade gracefully without panicking or reporting stale dimensions.
#[test]
fn t9_6_no_model_generate_embedding_returns_err_and_dimensions_none() {
    let _guard = TEST_MUTEX.lock();
    let mut ctx = TestContext::new();
    ctx.hide_packaged_model();

    vector::cleanup();
    assert_eq!(vector::embedding_dimensions(), None);
    assert_eq!(vector::model_filename(), None);

    let result = vector::generate_embedding("hello world");

    assert!(
        result.is_err(),
        "missing model should return Err, not panic"
    );
    assert_eq!(vector::embedding_dimensions(), None);
    assert_eq!(vector::model_filename(), None);
}

// T-9.6: RTM FR-7 / RTM-056 — when multiple *.onnx files exist, the largest file
// should be selected for runtime model loading.
#[test]
fn t9_6_multiple_onnx_files_select_largest() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();
    ctx.install_valid_shadow_model_with_fallback_assets("large.onnx");
    ctx.write_dummy_onnx("small.onnx", 100);

    vector::load_model().expect("load largest discovered model");

    assert_eq!(vector::model_filename().as_deref(), Some("large.onnx"));
    assert_eq!(vector::embedding_dimensions(), Some(384));
}

// T-9.6: RTM FR-8 / RTM-057 — database setup should always create the model_metadata table,
// and callers should be able to read persisted values through get_model_metadata.
#[test]
fn t9_6_model_metadata_table_created_and_readable() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();

    GraphDatabase::initialize_database(ctx.config()).expect("db init");
    let conn = db::open_connection(&ctx.config().database_path).expect("open db");

    assert!(
        table_exists(&conn, "model_metadata"),
        "model_metadata table should always exist after db init"
    );

    conn.execute(
        "INSERT INTO model_metadata (key, value) VALUES (?1, ?2)",
        params!["model_filename", "test-model.onnx"],
    )
    .expect("insert model filename metadata");
    conn.execute(
        "INSERT INTO model_metadata (key, value) VALUES (?1, ?2)",
        params!["embedding_dimensions", "384"],
    )
    .expect("insert embedding dimension metadata");

    assert_eq!(
        db::get_model_metadata(&conn, "model_filename").as_deref(),
        Some("test-model.onnx")
    );
    assert_eq!(
        db::get_model_metadata(&conn, "embedding_dimensions").as_deref(),
        Some("384")
    );
}

// T-9.6: RTM FR-8 / RTM-060 — with no loaded ONNX model, db initialization should
// not crash, should still create model_metadata, and should skip vec0 virtual tables.
#[test]
fn t9_6_no_model_db_setup_skips_vec0_tables_without_crashing() {
    let _guard = TEST_MUTEX.lock();
    let mut ctx = TestContext::new();
    ctx.hide_packaged_model();

    vector::cleanup();
    GraphDatabase::initialize_database(ctx.config()).expect("db init without model");
    let conn = db::open_connection(&ctx.config().database_path).expect("open db");

    assert!(table_exists(&conn, "model_metadata"));
    assert!(!table_exists(&conn, "vec_concepts"));
    assert!(!table_exists(&conn, "vec_memory_files"));
}
