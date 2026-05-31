use agent_client_protocol::schema::{self as acp};
use std::collections::HashSet;
use std::sync::atomic::AtomicU64;

pub(super) const INIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
pub(super) const SESSION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
pub(super) const LOAD_SESSION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);
pub(super) static LIVE_CONNECTION_SEQ: AtomicU64 = AtomicU64::new(1);

pub(crate) fn default_client_capabilities() -> acp::ClientCapabilities {
    // Filesystem methods are advertised by default, but JockeyUiClient enforces
    // absolute paths, canonical workspace containment, and explicit parent
    // directory creation for every fs request.
    acp::ClientCapabilities::new()
        .fs(acp::FileSystemCapabilities::new()
            .read_text_file(true)
            .write_text_file(true))
        .terminal(true)
}

pub(super) fn collect_models_from_select_options(
    options: &acp::SessionConfigSelectOptions,
    out: &mut HashSet<String>,
) {
    match options {
        acp::SessionConfigSelectOptions::Ungrouped(items) => {
            for item in items {
                out.insert(item.value.to_string());
            }
        }
        acp::SessionConfigSelectOptions::Grouped(groups) => {
            for group in groups {
                for item in &group.options {
                    out.insert(item.value.to_string());
                }
            }
        }
        _ => {}
    }
}

pub(super) fn extract_models_from_config_options(
    options: Option<&Vec<acp::SessionConfigOption>>,
) -> Vec<String> {
    let Some(options) = options else {
        return Vec::new();
    };
    let mut out = HashSet::new();
    for option in options {
        let is_model_category = matches!(
            option.category,
            Some(acp::SessionConfigOptionCategory::Model)
        );
        let is_model_id = option.id.to_string().eq_ignore_ascii_case("model");
        if !(is_model_category || is_model_id) {
            continue;
        }
        if let acp::SessionConfigKind::Select(select) = &option.kind {
            out.insert(select.current_value.to_string());
            collect_models_from_select_options(&select.options, &mut out);
        }
    }
    let mut models = out.into_iter().collect::<Vec<_>>();
    models.sort_unstable();
    models
}
