#[path = "../support/cli_runner.rs"]
mod cli_runner;
#[path = "../support/compat_env.rs"]
mod compat_env;
#[path = "../support/json_semantic.rs"]
mod json_semantic;

use cli_runner::{run_csharp_cli, run_rust_cli};
use compat_env::IsolatedRoot;


#[test]
fn watcher_lifecycle_text_matches_csharp_oracle() {
    let sandbox = IsolatedRoot::new("watcher-parity").expect("sandbox");

    let start_args = ["--tool", "StartWatcher", "--payload", r#"{"debounceMs":250}"#];
    let csharp_start = run_csharp_cli(&start_args, &sandbox).expect("csharp start");
    assert!(csharp_start.success(), "C# start failed: {}", csharp_start.stderr);

    let rust_start = run_rust_cli(&start_args, &sandbox).expect("rust start");
    assert!(rust_start.success(), "Rust start failed: {}", rust_start.stderr);
    assert_eq!(csharp_start.stdout, rust_start.stdout, "StartWatcher output drifted from C# oracle");

    let stop_args = ["--tool", "StopWatcher", "--payload", "{}"];
    let csharp_stop = run_csharp_cli(&stop_args, &sandbox).expect("csharp stop");
    assert!(csharp_stop.success(), "C# stop failed: {}", csharp_stop.stderr);

    let rust_stop = run_rust_cli(&stop_args, &sandbox).expect("rust stop");
    assert!(rust_stop.success(), "Rust stop failed: {}", rust_stop.stderr);
    assert_eq!(csharp_stop.stdout, rust_stop.stdout, "StopWatcher output drifted from C# oracle");
}


