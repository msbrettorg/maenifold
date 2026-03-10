use maenifold_core::{Config, GraphDatabase, db, vector};
use once_cell::sync::Lazy;
use ort::session::Session;
use parking_lot::Mutex;
use rusqlite::{Connection, params};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokenizers::{
    AddedToken, Tokenizer, models::wordpiece::WordPiece, normalizers::bert::BertNormalizer,
    pre_tokenizers::bert::BertPreTokenizer, processors::template::TemplateProcessing,
};

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

fn packaged_assets_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets")
        .canonicalize()
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets"))
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

fn table_sql(conn: &Connection, table_name: &str) -> Option<String> {
    conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
        [table_name],
        |row| row.get(0),
    )
    .ok()
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

fn model_output_dimensions(model_path: &Path) -> usize {
    let session = Session::builder()
        .expect("session builder")
        .commit_from_file(model_path)
        .expect("load model session");
    let mut candidates = session
        .outputs()
        .iter()
        .filter_map(|output| {
            let dims = output
                .dtype()
                .tensor_shape()?
                .iter()
                .copied()
                .collect::<Vec<_>>();
            Some((output.name().to_string(), dims))
        })
        .collect::<Vec<_>>();

    let preferred_names = [
        "sentence_embedding",
        "embedding",
        "pooled_output",
        "last_hidden_state",
    ];
    let chosen_dims = preferred_names
        .iter()
        .find_map(|preferred_name| {
            candidates
                .iter()
                .find(|(name, _)| name.to_ascii_lowercase().contains(preferred_name))
                .map(|(_, dims)| dims.clone())
        })
        .or_else(|| {
            candidates
                .iter()
                .find(|(_, dims)| dims.len() == 2)
                .map(|(_, dims)| dims.clone())
        })
        .or_else(|| {
            candidates
                .iter()
                .find(|(_, dims)| dims.len() == 1)
                .map(|(_, dims)| dims.clone())
        })
        .or_else(|| {
            candidates
                .iter()
                .find(|(_, dims)| dims.len() == 3)
                .map(|(_, dims)| dims.clone())
        })
        .or_else(|| candidates.pop().map(|(_, dims)| dims))
        .expect("model exposes tensor outputs");

    chosen_dims
        .last()
        .copied()
        .and_then(|dim| usize::try_from(dim).ok())
        .filter(|dim| *dim > 0)
        .expect("model output exposes concrete embedding dimension")
}

fn write_valid_tokenizer_json(model_dir: &Path) {
    let vocab_path = packaged_models_dir().join("vocab.txt");
    let vocab_path_str = vocab_path.to_str().expect("utf8 vocab path");
    let word_piece = WordPiece::from_file(vocab_path_str)
        .unk_token("[UNK]".to_string())
        .build()
        .expect("build wordpiece tokenizer");

    let mut tokenizer = Tokenizer::new(word_piece);
    tokenizer.with_normalizer(Some(BertNormalizer::new(true, true, None, true)));
    tokenizer.with_pre_tokenizer(Some(BertPreTokenizer));
    tokenizer.with_post_processor(Some(
        TemplateProcessing::builder()
            .try_single(vec!["[CLS]", "$0", "[SEP]"])
            .expect("single template")
            .try_pair("[CLS]:0 $A:0 [SEP]:0 $B:1 [SEP]:1")
            .expect("pair template")
            .special_tokens(vec![("[CLS]", 101), ("[SEP]", 102)])
            .build()
            .expect("build tokenizer template"),
    ));
    tokenizer.add_special_tokens(&[
        AddedToken::from("[PAD]".to_string(), true),
        AddedToken::from("[UNK]".to_string(), true),
        AddedToken::from("[CLS]".to_string(), true),
        AddedToken::from("[SEP]".to_string(), true),
        AddedToken::from("[MASK]".to_string(), true),
    ]);
    tokenizer
        .save(model_dir.join("tokenizer.json"), false)
        .expect("save tokenizer.json");
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

    fn write_tokenizer_config(&self, contents: &str) {
        let path = self.shadow_models_dir().join("tokenizer_config.json");
        if path.exists() {
            fs::remove_file(&path).expect("remove linked tokenizer config");
        }
        fs::write(path, contents).expect("write tokenizer config");
    }

    fn write_vocab(&self, contents: &str) {
        let path = self.shadow_models_dir().join("vocab.txt");
        if path.exists() {
            fs::remove_file(&path).expect("remove linked vocab");
        }
        fs::write(path, contents).expect("write vocab");
    }

    fn write_tokenizer_json(&self) {
        write_valid_tokenizer_json(&self.shadow_models_dir());
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

// T-9.6: RTM FR-7 / RTM-051 — runtime embedding dimensions must match the loaded
// ONNX output tensor metadata instead of a hardcoded constant.
#[test]
fn t9_6_embedding_dimensions_match_model_output_tensor() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();
    ctx.install_valid_shadow_model_with_fallback_assets("output-dims.onnx");
    let expected_dims = model_output_dimensions(&ctx.shadow_models_dir().join("output-dims.onnx"));

    vector::load_model().expect("load model for output dimension check");

    assert_eq!(vector::embedding_dimensions(), Some(expected_dims));
    assert_eq!(
        vector::generate_embedding("runtime dimensions")
            .expect("embedding with runtime dimensions")
            .len(),
        expected_dims
    );
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

// T-10.11: RTM FR-18 / RTM-098 — when both tokenizer.json and manual fallback assets exist,
// runtime loading must prefer tokenizer.json rather than rebuilding from vocab.txt.
#[test]
fn t10_11_tokenizer_json_is_preferred_over_vocab_txt() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();
    ctx.install_valid_shadow_model_with_fallback_assets("preferred-tokenizer.onnx");
    ctx.write_vocab("this is intentionally not a valid wordpiece vocab");
    ctx.write_tokenizer_config(r#"{"pad_token":"[PAD]","model_max_length":96}"#);
    ctx.write_tokenizer_json();

    vector::load_model().expect("load model via tokenizer.json");

    assert_eq!(
        vector::model_filename().as_deref(),
        Some("preferred-tokenizer.onnx")
    );
    assert_eq!(vector::max_tokens(), Some(96));
    assert_eq!(vector::embedding_dimensions(), Some(384));
    assert_eq!(
        vector::generate_embedding("tokenizer preference")
            .expect("embedding from tokenizer.json")
            .len(),
        384
    );
}

// T-10.11: RTM FR-18 / RTM-099 — tokenizer_config.json should supply model_max_length when
// present, and runtime loading should fall back to 512 when the field is absent.
#[test]
fn t10_11_max_tokens_reads_config_and_defaults_to_512() {
    let _guard = TEST_MUTEX.lock();

    {
        let ctx = TestContext::new();
        ctx.install_valid_shadow_model_with_fallback_assets("configured-max-tokens.onnx");
        ctx.write_tokenizer_config(r#"{"pad_token":"[PAD]","model_max_length":128}"#);
        ctx.write_tokenizer_json();

        vector::load_model().expect("load configured max tokens model");
        assert_eq!(vector::max_tokens(), Some(128));
    }

    {
        let ctx = TestContext::new();
        ctx.install_valid_shadow_model_with_fallback_assets("default-max-tokens.onnx");
        ctx.write_tokenizer_config(r#"{"pad_token":"[PAD]"}"#);
        ctx.write_tokenizer_json();

        vector::load_model().expect("load default max tokens model");
        assert_eq!(vector::max_tokens(), Some(512));
    }
}

// T-10.11: RTM FR-18 / RTM-100 / T-9.6: RTM FR-8 / RTM-057 — vec0 virtual tables must use
// the runtime-discovered embedding dimension and persist both model filename + dimensions.
#[test]
fn t10_11_vec0_tables_use_runtime_embedding_dimensions() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();
    ctx.install_valid_shadow_model_with_fallback_assets("vec0-runtime-dims.onnx");

    vector::load_model().expect("load model for vec0 dims");
    let runtime_dims = vector::embedding_dimensions().expect("runtime dimensions");

    let conn = db::open_connection(&ctx.config().database_path).expect("open db");
    let Some(vec0_path) = vec0_extension_path(&packaged_assets_root()) else {
        eprintln!("Test skipped: sqlite-vec extension asset unavailable");
        return;
    };
    if let Err(error) = load_vec_extension(&conn, &vec0_path) {
        eprintln!("Test skipped: sqlite-vec extension unavailable ({error})");
        return;
    }

    db::ensure_vec0_tables(&conn).expect("ensure vec0 tables");

    let expected_fragment = format!("FLOAT[{runtime_dims}]");
    let runtime_dims_str = runtime_dims.to_string();
    let vec_concepts_sql = table_sql(&conn, "vec_concepts").expect("vec_concepts schema");
    let vec_memory_files_sql =
        table_sql(&conn, "vec_memory_files").expect("vec_memory_files schema");

    assert!(
        vec_concepts_sql.contains(&expected_fragment),
        "vec_concepts schema should contain {expected_fragment}, got {vec_concepts_sql}"
    );
    assert!(
        vec_memory_files_sql.contains(&expected_fragment),
        "vec_memory_files schema should contain {expected_fragment}, got {vec_memory_files_sql}"
    );
    assert_eq!(
        db::get_model_metadata(&conn, "embedding_dimensions").as_deref(),
        Some(runtime_dims_str.as_str())
    );
    assert_eq!(
        db::get_model_metadata(&conn, "model_filename").as_deref(),
        Some("vec0-runtime-dims.onnx")
    );
}

// T-10.11: RTM FR-18 / RTM-101 — when stored vec0 schema metadata reflects a prior model with
// different dimensions, ensure_vec0_tables should recreate both vec0 tables at the current size.
#[test]
fn t10_11_vec0_tables_recreate_when_dimensions_change() {
    let _guard = TEST_MUTEX.lock();
    let ctx = TestContext::new();
    ctx.install_valid_shadow_model_with_fallback_assets("vec0-recreate.onnx");

    vector::load_model().expect("load model for vec0 recreation");
    let runtime_dims = vector::embedding_dimensions().expect("runtime dimensions");
    let previous_dims = runtime_dims + 1;

    let conn = db::open_connection(&ctx.config().database_path).expect("open db");
    let Some(vec0_path) = vec0_extension_path(&packaged_assets_root()) else {
        eprintln!("Test skipped: sqlite-vec extension asset unavailable");
        return;
    };
    if let Err(error) = load_vec_extension(&conn, &vec0_path) {
        eprintln!("Test skipped: sqlite-vec extension unavailable ({error})");
        return;
    }

    db::ensure_vec0_tables(&conn).expect("create vec0 tables");

    conn.execute(
        "INSERT INTO model_metadata (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["embedding_dimensions", previous_dims.to_string()],
    )
    .expect("seed prior embedding dimensions");
    conn.execute_batch(&format!(
        r#"
        DROP TABLE IF EXISTS vec_concepts;
        DROP TABLE IF EXISTS vec_memory_files;
        CREATE VIRTUAL TABLE vec_concepts USING vec0(
            concept_name TEXT PRIMARY KEY,
            embedding FLOAT[{previous_dims}]
        );
        CREATE VIRTUAL TABLE vec_memory_files USING vec0(
            file_path TEXT PRIMARY KEY,
            embedding FLOAT[{previous_dims}]
        );
        "#
    ))
    .expect("seed prior vec0 schema");

    assert!(
        table_sql(&conn, "vec_concepts")
            .expect("seeded vec_concepts schema")
            .contains(&format!("FLOAT[{previous_dims}]"))
    );

    db::ensure_vec0_tables(&conn).expect("recreate vec0 tables at runtime dimensions");

    let expected_fragment = format!("FLOAT[{runtime_dims}]");
    let runtime_dims_str = runtime_dims.to_string();
    assert!(
        table_sql(&conn, "vec_concepts")
            .expect("recreated vec_concepts schema")
            .contains(&expected_fragment)
    );
    assert!(
        table_sql(&conn, "vec_memory_files")
            .expect("recreated vec_memory_files schema")
            .contains(&expected_fragment)
    );
    assert_eq!(
        db::get_model_metadata(&conn, "embedding_dimensions").as_deref(),
        Some(runtime_dims_str.as_str())
    );
}
