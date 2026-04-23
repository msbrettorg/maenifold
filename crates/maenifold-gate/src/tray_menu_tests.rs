
use std::path::Path;

#[cfg(target_os = "macos")]
use super::launch_agent_plist;
use super::open_ui_enabled;
use super::ui_args;

#[test]
fn ui_args_target_requested_root() {
    let args = ui_args(Path::new("/tmp/example"));
    assert_eq!(args, vec!["ui", "--root", "/tmp/example"]);
}

#[test]
fn open_ui_is_enabled_only_when_root_exists() {
    assert!(open_ui_enabled(true));
    assert!(!open_ui_enabled(false));
}

#[cfg(target_os = "macos")]
#[test]
fn http_login_item_plist_contains_gate_service_label_and_port() {
    let plist = launch_agent_plist(Path::new("/usr/local/bin/gate"), 8181, None);
    assert!(plist.contains("com.ma-collective.gate"));
    assert!(plist.contains("--http"));
    assert!(plist.contains("<string>8181</string>"));
    assert!(!plist.contains("--root"));
}

#[cfg(target_os = "macos")]
#[test]
fn http_login_item_plist_preserves_root_when_present() {
    let plist = launch_agent_plist(
        Path::new("/usr/local/bin/gate"),
        8181,
        Some(Path::new("/workspace/project")),
    );
    assert!(plist.contains("<string>--root</string>"));
    assert!(plist.contains("<string>/workspace/project</string>"));
}
