#![allow(dead_code)]

use maenifold_core::{Config, vector};
use once_cell::sync::Lazy;
use ort::session::Session;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokenizers::{
    AddedToken, Tokenizer, models::wordpiece::WordPiece, normalizers::bert::BertNormalizer,
    pre_tokenizers::bert::BertPreTokenizer, processors::template::TemplateProcessing,
};

pub(crate) static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

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

pub(crate) fn packaged_assets_root() -> PathBuf {
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

pub(crate) fn table_exists(conn: &Connection, table_name: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        [table_name],
        |row| row.get::<_, i64>(0),
    )
    .expect("query sqlite_master")
        == 1
}

pub(crate) fn table_sql(conn: &Connection, table_name: &str) -> Option<String> {
    conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
        [table_name],
        |row| row.get(0),
    )
    .ok()
}

pub(crate) fn vec0_extension_path(assets_root: &Path) -> Option<PathBuf> {
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

pub(crate) fn load_vec_extension(
    connection: &Connection,
    extension_path: &Path,
) -> rusqlite::Result<()> {
    unsafe {
        connection.load_extension_enable()?;
        let load_result = connection.load_extension(extension_path, None::<&str>);
        let disable_result = connection.load_extension_disable();
        load_result?;
        disable_result?;
    }
    Ok(())
}

pub(crate) fn model_output_dimensions(model_path: &Path) -> usize {
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

fn install_valid_model_with_fallback_assets(models_dir: &Path, model_name: &str) {
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

pub(crate) struct TestContext {
    _tmp: TempDir,
    cfg: Config,
    shadow_assets_root: PathBuf,
    shadow_backup: Option<PathBuf>,
    hidden_packaged_model: Option<(PathBuf, PathBuf)>,
}

impl TestContext {
    pub(crate) fn new() -> Self {
        // T-9.6: RTM FR-7 / FR-8 - isolate runtime model state per split integration test.
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

    pub(crate) fn config(&self) -> &Config {
        &self.cfg
    }

    pub(crate) fn shadow_models_dir(&self) -> PathBuf {
        self.shadow_assets_root.join("models")
    }

    pub(crate) fn install_valid_shadow_model_with_fallback_assets(&self, model_name: &str) {
        install_valid_model_with_fallback_assets(&self.shadow_models_dir(), model_name);
    }

    // T-20.5: RTM FR-7 / RTM-148 — install a valid model only under the shared
    // runtime root (`Config::current().assets_path/models`) so discovery can
    // verify the C#-compatible fallback path.
    pub(crate) fn install_valid_shared_model_with_fallback_assets(&self, model_name: &str) {
        install_valid_model_with_fallback_assets(&self.cfg.assets_path.join("models"), model_name);
    }

    pub(crate) fn write_dummy_onnx(&self, model_name: &str, bytes: usize) {
        fs::write(self.shadow_models_dir().join(model_name), vec![b'x'; bytes])
            .expect("write dummy model");
    }

    pub(crate) fn write_tokenizer_config(&self, contents: &str) {
        let path = self.shadow_models_dir().join("tokenizer_config.json");
        if path.exists() {
            fs::remove_file(&path).expect("remove linked tokenizer config");
        }
        fs::write(path, contents).expect("write tokenizer config");
    }

    pub(crate) fn write_vocab(&self, contents: &str) {
        let path = self.shadow_models_dir().join("vocab.txt");
        if path.exists() {
            fs::remove_file(&path).expect("remove linked vocab");
        }
        fs::write(path, contents).expect("write vocab");
    }

    pub(crate) fn write_tokenizer_json(&self) {
        write_valid_tokenizer_json(&self.shadow_models_dir());
    }

    pub(crate) fn hide_packaged_model(&mut self) {
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
