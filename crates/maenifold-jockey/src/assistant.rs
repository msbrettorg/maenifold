use crate::acp;
use crate::runtime_kind::RuntimeKind;
use crate::types::*;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const ASSISTANT_CATALOG_TTL: Duration = Duration::from_secs(15);

type CatalogCache = Option<(Instant, Vec<AssistantRuntime>)>;

fn catalog_cache() -> &'static Mutex<CatalogCache> {
    static CACHE: OnceLock<Mutex<CatalogCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn detect_binary_version(binary: &str) -> Option<String> {
    match Command::new(binary).arg("--version").output() {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout);
            text.lines()
                .next()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        }
        _ => None,
    }
}

pub(crate) fn build_assistant_catalog() -> Vec<AssistantRuntime> {
    let kinds = [
        RuntimeKind::ClaudeCode,
        RuntimeKind::GeminiCli,
        RuntimeKind::CodexCli,
    ];
    kinds
        .into_iter()
        .map(|kind| {
            let fallback = format!("{} adapter unavailable", kind.runtime_key());
            let (available, binary) =
                acp::probe_runtime(kind.runtime_key()).unwrap_or((false, fallback));
            let version = if available && binary != "npx" {
                detect_binary_version(&binary)
            } else {
                None
            };
            let install_hint = if available {
                None
            } else {
                let h = kind.install_hint();
                if h.is_empty() {
                    None
                } else {
                    Some(h.to_string())
                }
            };
            AssistantRuntime {
                key: kind.runtime_key().to_string(),
                label: kind.label().to_string(),
                binary,
                available,
                version,
                install_hint,
            }
        })
        .collect()
}

pub(crate) fn cached_assistant_catalog() -> Vec<AssistantRuntime> {
    if let Ok(guard) = catalog_cache().lock() {
        if let Some((at, rows)) = guard.as_ref() {
            if at.elapsed() <= ASSISTANT_CATALOG_TTL {
                return rows.clone();
            }
        }
    }
    refresh_assistant_catalog()
}

pub fn refresh_assistant_catalog() -> Vec<AssistantRuntime> {
    let rows = build_assistant_catalog();
    if let Ok(mut guard) = catalog_cache().lock() {
        *guard = Some((Instant::now(), rows.clone()));
    }
    rows
}

pub(crate) fn normalize_runtime_key(runtime: &str) -> Option<&'static str> {
    RuntimeKind::from_str(runtime).map(|k| k.runtime_key())
}

pub(crate) async fn detect_assistants() -> Result<Vec<AssistantRuntime>, String> {
    tokio::task::spawn_blocking(|| {
        acp::clear_adapter_cache();
        refresh_assistant_catalog()
    })
    .await
    .map_err(|e| e.to_string())
}
