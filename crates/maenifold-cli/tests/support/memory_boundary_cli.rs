// T-12.7: RTM NFR-1 - share CLI harness helpers while splitting regression coverage into themed files.
use assert_cmd::Command;
use tempfile::TempDir;

pub fn isolated_root() -> TempDir {
    TempDir::new().expect("tmp")
}

pub fn maenifold_cmd(tmp: &TempDir) -> Command {
    // T-0.3g: RTM-047 - avoid deprecated assert_cmd constructor without changing test behavior.
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("maenifold"));
    cmd.env("MAENIFOLD_ROOT", tmp.path())
        .env("MAENIFOLD_DATABASE_PATH", tmp.path().join("memory.db"));
    cmd
}
