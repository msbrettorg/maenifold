mod support;

use std::fs;

use predicates::prelude::*;
use support::{
    bundled_asset_path, gate_command, new_pipeline_root, runtime_assets_dir, write_pending_task,
};

#[test]
fn sync_warns_when_refreshing_stale_reserved_builtin_runtime_asset() {
    let root = new_pipeline_root();
    write_pending_task(root.path(), "T-90.23");

    let runtime_dir = runtime_assets_dir(root.path());
    fs::create_dir_all(&runtime_dir).expect("create runtime assets dir");
    let runtime_sdd = runtime_dir.join("sdd.json");
    fs::write(
        &runtime_sdd,
        r#"{"schemaVersion":1,"id":"sdd","name":"stale","initial":"pending","states":{"pending":{"terminal":true}}}"#,
    )
    .expect("write stale runtime builtin");

    gate_command(root.path())
        .args([
            "sync",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--rebuild",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "warning: refreshed bundled built-in gate definition 'sdd'",
        ));

    assert_eq!(
        fs::read_to_string(&runtime_sdd).expect("read refreshed runtime builtin"),
        fs::read_to_string(bundled_asset_path("sdd")).expect("read bundled sdd"),
    );

    gate_command(root.path())
        .args([
            "sync",
            "--root",
            root.path().to_str().expect("root utf8"),
            "--rebuild",
        ])
        .assert()
        .success()
        .stderr(
            predicate::str::contains("warning: refreshed bundled built-in gate definition").not(),
        );
}
